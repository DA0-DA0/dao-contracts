use cosmwasm_std::{
    coins, to_json_binary, Addr, BankMsg, BlockInfo, CosmosMsg, Deps, DepsMut, StdError, StdResult,
    Uint128, Uint256, WasmMsg,
};
use cw20::{Denom, Expiration};
use cw_utils::Duration;
use dao_interface::voting::{
    Query as VotingQueryMsg, TotalPowerAtHeightResponse, VotingPowerAtHeightResponse,
};

use crate::ContractError;

pub fn get_prev_block_total_vp(
    deps: Deps,
    block: &BlockInfo,
    contract_addr: &Addr,
) -> StdResult<Uint128> {
    let msg = VotingQueryMsg::TotalPowerAtHeight {
        height: Some(block.height.checked_sub(1).unwrap_or_default()),
    };
    let resp: TotalPowerAtHeightResponse = deps.querier.query_wasm_smart(contract_addr, &msg)?;
    Ok(resp.power)
}

pub fn get_voting_power_at_block(
    deps: Deps,
    block: &BlockInfo,
    contract_addr: &Addr,
    addr: &Addr,
) -> StdResult<Uint128> {
    let msg = VotingQueryMsg::VotingPowerAtHeight {
        address: addr.into(),
        height: Some(block.height),
    };
    let resp: VotingPowerAtHeightResponse = deps.querier.query_wasm_smart(contract_addr, &msg)?;
    Ok(resp.power)
}

/// returns underlying scalar value for a given duration.
/// if the duration is a block height, return the number of blocks.
/// if the duration is a time, return the number of seconds.
pub fn get_duration_scalar(duration: &Duration) -> u64 {
    match duration {
        Duration::Height(h) => *h,
        Duration::Time(t) => *t,
    }
}

/// Returns the appropriate CosmosMsg for transferring the reward token.
pub fn get_transfer_msg(recipient: Addr, amount: Uint128, denom: Denom) -> StdResult<CosmosMsg> {
    match denom {
        Denom::Native(denom) => Ok(BankMsg::Send {
            to_address: recipient.into_string(),
            amount: coins(amount.u128(), denom),
        }
        .into()),
        Denom::Cw20(addr) => {
            let cw20_msg = to_json_binary(&cw20::Cw20ExecuteMsg::Transfer {
                recipient: recipient.into_string(),
                amount,
            })?;
            Ok(WasmMsg::Execute {
                contract_addr: addr.into_string(),
                msg: cw20_msg,
                funds: vec![],
            }
            .into())
        }
    }
}

pub(crate) fn scale_factor() -> Uint256 {
    Uint256::from(10u8).pow(39)
}

/// Calculate the duration scalar value from start to end. If the end is at or
/// before the start, return 0. The first argument is end, and the second is
/// start. If start and end are block heights, this returns the number of
/// blocks. If they are times, this returns the number of seconds. If both are
/// never, this returns 0. If start and end have different units, it errors as
/// that should not be possible.
pub fn get_exp_diff_scalar(end: &Expiration, start: &Expiration) -> StdResult<u64> {
    match (end, start) {
        (Expiration::AtHeight(end), Expiration::AtHeight(start)) => {
            if end > start {
                Ok(end - start)
            } else {
                Ok(0)
            }
        }
        (Expiration::AtTime(end), Expiration::AtTime(start)) => {
            if end > start {
                Ok(end.seconds() - start.seconds())
            } else {
                Ok(0)
            }
        }
        (Expiration::Never {}, Expiration::Never {}) => Ok(0),
        _ => Err(StdError::generic_err(format!(
            "incompatible expirations: got end {:?}, start {:?}",
            end, start
        ))),
    }
}

pub fn validate_voting_power_contract(
    deps: &DepsMut,
    vp_contract: String,
) -> Result<Addr, ContractError> {
    let vp_contract = deps.api.addr_validate(&vp_contract)?;
    let _: TotalPowerAtHeightResponse = deps.querier.query_wasm_smart(
        &vp_contract,
        &VotingQueryMsg::TotalPowerAtHeight { height: None },
    )?;
    Ok(vp_contract)
}
