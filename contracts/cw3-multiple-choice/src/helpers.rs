use crate::{
    query::ProposalResponse,
    state::{parse_id, Proposal},
    ContractError,
};
use cosmwasm_std::{
    from_slice, to_binary, Addr, BlockInfo, CosmosMsg, Decimal, Deps, Env, MessageInfo,
    QuerierWrapper, StdError, StdResult, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use packages::constants::STAKING_CONTRACT_KEY;
use serde::de::DeserializeOwned;
use stake_cw20::msg::{
    QueryMsg as StakingContractQueryMsg, StakedBalanceAtHeightResponse, TotalStakedAtHeightResponse,
};

// Settings for pagination
pub const MAX_LIMIT: u32 = 30;
pub const DEFAULT_LIMIT: u32 = 10;

// we multiply by this when calculating needed_votes in order to round up properly
// Note: `10u128.pow(9)` fails as "u128::pow` is not yet stable as a const fn"
const PRECISION_FACTOR: u128 = 1_000_000_000;

// this is a helper function so Decimal works with u64 rather than Uint128
// also, we must *round up* here, as we need 8, not 7 votes to reach 50% of 15 total
pub fn votes_needed(weight: Uint128, percentage: Decimal) -> Uint128 {
    let applied = percentage * Uint128::from(PRECISION_FACTOR * weight.u128());
    // Divide by PRECISION_FACTOR, rounding up to the nearest integer
    Uint128::from((applied.u128() + PRECISION_FACTOR - 1) / PRECISION_FACTOR)
}

pub fn map_proposal(
    block: &BlockInfo,
    item: StdResult<(Vec<u8>, Proposal)>,
) -> StdResult<ProposalResponse> {
    let (key, mut prop) = item?;
    let status = prop.current_status(block);
    let threshold = prop.threshold.to_response(prop.total_weight);
    Ok(ProposalResponse {
        id: parse_id(&key)?,
        title: prop.title,
        description: prop.description,
        proposer: prop.proposer,
        msgs: prop.msgs,
        status,
        expires: prop.expires,
        threshold,
        deposit_amount: prop.deposit,
        start_height: prop.start_height,
    })
}

pub fn get_and_check_limit(limit: Option<u32>, max: u32, default: u32) -> StdResult<u32> {
    match limit {
        Some(l) => {
            if l <= max {
                Ok(l)
            } else {
                Err(StdError::GenericErr {
                    msg: ContractError::OversizedRequest {
                        size: l as u64,
                        max: max as u64,
                    }
                    .to_string(),
                })
            }
        }
        None => Ok(default),
    }
}

pub fn get_deposit_message(
    env: &Env,
    info: &MessageInfo,
    amount: &Uint128,
    gov_token: &Addr,
) -> StdResult<Vec<CosmosMsg>> {
    if *amount == Uint128::zero() {
        return Ok(vec![]);
    }
    let transfer_cw20_msg = Cw20ExecuteMsg::TransferFrom {
        owner: info.sender.clone().into(),
        recipient: env.contract.address.clone().into(),
        amount: *amount,
    };
    let exec_cw20_transfer = WasmMsg::Execute {
        contract_addr: gov_token.into(),
        msg: to_binary(&transfer_cw20_msg)?,
        funds: vec![],
    };
    let cw20_transfer_cosmos_msg: CosmosMsg = exec_cw20_transfer.into();
    Ok(vec![cw20_transfer_cosmos_msg])
}

pub fn get_proposal_deposit_refund_message(
    proposer: &Addr,
    amount: &Uint128,
    gov_token: &Addr,
) -> StdResult<Vec<CosmosMsg>> {
    if *amount == Uint128::zero() {
        return Ok(vec![]);
    }
    let transfer_cw20_msg = Cw20ExecuteMsg::Transfer {
        recipient: proposer.into(),
        amount: *amount,
    };
    let exec_cw20_transfer = WasmMsg::Execute {
        contract_addr: gov_token.into(),
        msg: to_binary(&transfer_cw20_msg)?,
        funds: vec![],
    };
    let cw20_transfer_cosmos_msg: CosmosMsg = exec_cw20_transfer.into();
    Ok(vec![cw20_transfer_cosmos_msg])
}

pub fn get_total_staked_supply(deps: Deps, parent_dao_address: &Addr) -> StdResult<Uint128> {
    let res = query_parent_contract_storage::<Addr>(
        &deps.querier,
        STAKING_CONTRACT_KEY,
        parent_dao_address,
    );

    if res.is_err() {
        return Err(StdError::GenericErr {
            msg: res.err().unwrap().to_string(),
        });
    }

    let staking_contract = res.unwrap();

    // Get total supply
    let total: TotalStakedAtHeightResponse = deps.querier.query_wasm_smart(
        staking_contract,
        &StakingContractQueryMsg::TotalStakedAtHeight { height: None },
    )?;
    Ok(total.total)
}

pub fn get_staked_balance(
    deps: Deps,
    address: Addr,
    parent_dao_address: &Addr,
) -> StdResult<Uint128> {
    let res = query_parent_contract_storage::<Addr>(
        &deps.querier,
        STAKING_CONTRACT_KEY,
        parent_dao_address,
    );

    if res.is_err() {
        return Err(StdError::GenericErr {
            msg: res.err().unwrap().to_string(),
        });
    }

    let staking_contract = res.unwrap();

    // Get current staked balance
    let res: StakedBalanceAtHeightResponse = deps.querier.query_wasm_smart(
        staking_contract,
        &StakingContractQueryMsg::StakedBalanceAtHeight {
            address: address.to_string(),
            height: None,
        },
    )?;
    Ok(res.balance)
}

pub fn get_voting_power_at_height(
    deps: Deps,
    address: Addr,
    height: u64,
    parent_address: &Addr,
) -> StdResult<Uint128> {
    let res =
        query_parent_contract_storage::<Addr>(&deps.querier, STAKING_CONTRACT_KEY, parent_address);

    if res.is_err() {
        return Err(StdError::GenericErr {
            msg: res.err().unwrap().to_string(),
        });
    }

    let staking_contract = res.unwrap();

    // Get voting power at height
    let balance: StakedBalanceAtHeightResponse = deps.querier.query_wasm_smart(
        staking_contract,
        &StakingContractQueryMsg::StakedBalanceAtHeight {
            address: address.to_string(),
            height: Some(height),
        },
    )?;
    Ok(balance.balance)
}

// Query parent dao contract storage
pub fn query_parent_contract_storage<T>(
    querier: &QuerierWrapper,
    storage_key: &str,
    parent_contract_address: &Addr,
) -> Result<T, ContractError>
where
    T: DeserializeOwned,
{
    let res = querier.query_wasm_raw(parent_contract_address, storage_key.as_bytes());
    if res.is_err() {
        return Err(ContractError::QueryStorageFailed {});
    }

    let opt = res.unwrap();
    match opt {
        Some(bytes) => {
            let val = from_slice::<T>(&bytes)?;
            return Ok(val);
        }
        None => return Err(ContractError::QueryStorageFailed {}),
    }
}
