use cosmwasm_std::{
    to_binary, Addr, BlockInfo, CosmosMsg, Deps, Env, MessageInfo, StdResult, Uint128, WasmMsg,
};
use cw20::{BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg};
use cw20_base::state::TokenInfo;
use cw20_gov::msg::{QueryMsg as Cw20GovQueryMsg, VotingPowerAtHeightResponse};

use crate::{
    query::ProposalResponse,
    state::{parse_id, Proposal, GOV_TOKEN},
};

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

pub fn get_total_supply(deps: Deps) -> StdResult<Uint128> {
    let gov_token = GOV_TOKEN.load(deps.storage)?;

    // Get total supply
    let token_info: TokenInfo = deps
        .querier
        .query_wasm_smart(gov_token, &Cw20QueryMsg::TokenInfo {})?;
    Ok(token_info.total_supply)
}

pub fn get_balance(deps: Deps, address: Addr) -> StdResult<Uint128> {
    let gov_token = GOV_TOKEN.load(deps.storage)?;

    // Get total supply
    let balance: BalanceResponse = deps
        .querier
        .query_wasm_smart(
            gov_token,
            &Cw20QueryMsg::Balance {
                address: address.to_string(),
            },
        )
        .unwrap_or(BalanceResponse {
            balance: Uint128::zero(),
        });
    Ok(balance.balance)
}

pub fn get_voting_power_at_height(deps: Deps, address: Addr, height: u64) -> StdResult<Uint128> {
    let gov_token = GOV_TOKEN.load(deps.storage)?;

    // Get total supply
    let balance: VotingPowerAtHeightResponse = deps
        .querier
        .query_wasm_smart(
            gov_token,
            &Cw20GovQueryMsg::VotingPowerAtHeight {
                address: address.to_string(),
                height,
            },
        )
        .unwrap_or(VotingPowerAtHeightResponse {
            balance: Uint128::zero(),
            height: 0,
        });
    Ok(balance.balance)
}

pub fn map_proposal(
    block: &BlockInfo,
    item: StdResult<(Vec<u8>, Proposal)>,
) -> StdResult<ProposalResponse> {
    let (key, prop) = item?;
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
    })
}
