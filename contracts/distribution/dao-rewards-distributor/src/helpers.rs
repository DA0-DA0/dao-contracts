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

pub trait ExpirationExt {
    /// Compute the duration since the start, flooring at 0 if the current
    /// expiration is before the start. If either is never, or if they have
    /// different units, returns an error as those cannot be compared.
    fn duration_since(&self, start: &Self) -> StdResult<Duration>;
}

impl ExpirationExt for Expiration {
    fn duration_since(&self, start: &Self) -> StdResult<Duration> {
        match (self, start) {
            (Expiration::AtHeight(end), Expiration::AtHeight(start)) => {
                if end > start {
                    Ok(Duration::Height(end - start))
                } else {
                    Ok(Duration::Height(0))
                }
            }
            (Expiration::AtTime(end), Expiration::AtTime(start)) => {
                if end > start {
                    Ok(Duration::Time(end.seconds() - start.seconds()))
                } else {
                    Ok(Duration::Time(0))
                }
            }
            (Expiration::Never {}, _) | (_, Expiration::Never {}) => {
                Err(StdError::generic_err(format!(
                "can't compute diff between expirations with never: got end {:?} and start {:?}",
                self, start
            )))
            }
            _ => Err(StdError::generic_err(format!(
                "incompatible expirations: got end {:?} and start {:?}",
                self, start
            ))),
        }
    }
}

pub trait DurationExt {
    /// Returns true if the duration is 0 blocks or 0 seconds.
    fn is_zero(&self) -> bool;

    /// Perform checked integer division between two durations, erroring if the
    /// units do not match or denominator is 0.
    fn checked_div(&self, denominator: &Self) -> Result<Uint128, ContractError>;
}

impl DurationExt for Duration {
    fn is_zero(&self) -> bool {
        match self {
            Duration::Height(h) => *h == 0,
            Duration::Time(t) => *t == 0,
        }
    }

    fn checked_div(&self, denominator: &Self) -> Result<Uint128, ContractError> {
        match (self, denominator) {
            (Duration::Height(numerator), Duration::Height(denominator)) => {
                Ok(Uint128::from(*numerator).checked_div(Uint128::from(*denominator))?)
            }
            (Duration::Time(numerator), Duration::Time(denominator)) => {
                Ok(Uint128::from(*numerator).checked_div(Uint128::from(*denominator))?)
            }
            _ => Err(ContractError::Std(StdError::generic_err(format!(
                "incompatible durations: got numerator {:?} and denominator {:?}",
                self, denominator
            )))),
        }
    }
}
