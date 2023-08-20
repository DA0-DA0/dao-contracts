use crate::state::{BLACKLISTED_ADDRESSES, DENOM, IS_FROZEN, OWNER};
use crate::ContractError;
use cosmwasm_std::{Addr, Coin, Deps, MessageInfo, Uint128};
use cw_storage_plus::Map;

pub fn check_contract_has_funds(
    denom: String,
    funds: &[Coin],
    amount: Uint128,
) -> Result<(), ContractError> {
    if let Some(c) = funds.iter().find(|c| c.denom == denom) {
        if c.amount < amount {
            Err(ContractError::NotEnoughFunds {
                denom,
                funds: c.amount.u128(),
                needed: amount.u128(),
            })
        } else {
            Ok(())
        }
    } else {
        Err(ContractError::NotEnoughFunds {
            denom,
            funds: 0u128,
            needed: amount.u128(),
        })
    }
}

pub fn check_is_contract_owner(deps: Deps, sender: Addr) -> Result<(), ContractError> {
    let owner = OWNER.load(deps.storage)?;
    if owner != sender {
        Err(ContractError::Unauthorized {})
    } else {
        Ok(())
    }
}

pub fn check_bool_allowance(
    deps: Deps,
    info: MessageInfo,
    allowances: Map<&Addr, bool>,
) -> Result<(), ContractError> {
    let res = allowances.load(deps.storage, &info.sender);
    match res {
        Ok(authorized) => {
            if !authorized {
                return Err(ContractError::Unauthorized {});
            }
        }
        Err(error) => {
            if let cosmwasm_std::StdError::NotFound { .. } = error {
                return Err(ContractError::Unauthorized {});
            } else {
                return Err(ContractError::Std(error));
            }
        }
    }
    Ok(())
}

pub fn check_is_not_blacklisted(deps: Deps, address: String) -> Result<(), ContractError> {
    let addr = deps.api.addr_validate(&address)?;
    if let Some(is_blacklisted) = BLACKLISTED_ADDRESSES.may_load(deps.storage, &addr)? {
        if is_blacklisted {
            return Err(ContractError::Blacklisted { address });
        }
    };
    Ok(())
}

pub fn check_is_not_frozen(deps: Deps, denom: &str) -> Result<(), ContractError> {
    let is_frozen = IS_FROZEN.load(deps.storage)?;
    let contract_denom = DENOM.load(deps.storage)?;

    // check if issuer is configured to be frozen and the arriving denom is the same
    // as this contract denom.
    // Denom can be different since setting beforesend listener doesn't check
    // contract's denom.
    let is_denom_frozen = is_frozen && denom == contract_denom;
    if is_denom_frozen {
        return Err(ContractError::ContractFrozen {
            denom: contract_denom,
        });
    }

    Ok(())
}
