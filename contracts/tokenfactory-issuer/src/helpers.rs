use cosmwasm_std::{Addr, Coin, Deps, MessageInfo, Uint128};
use cw_storage_plus::Map;

use crate::state::{BLACKLISTED_ADDRESSES, CONFIG};
use crate::ContractError;

pub fn build_denom(creator: &Addr, subdenom: &str) -> Result<String, ContractError> {
    // Minimum validation checks on the full denom.
    // https://github.com/cosmos/cosmos-sdk/blob/2646b474c7beb0c93d4fafd395ef345f41afc251/types/coin.go#L706-L711
    // https://github.com/cosmos/cosmos-sdk/blob/2646b474c7beb0c93d4fafd395ef345f41afc251/types/coin.go#L677
    let full_denom = format!("factory/{}/{}", creator, subdenom);
    if full_denom.len() < 3
        || full_denom.len() > 128
        || creator.as_str().contains('/')
        || subdenom.len() > 44
        || creator.as_str().len() > 75
    {
        return Err(ContractError::InvalidDenom {
            denom: full_denom,
            message: "".to_string(),
        });
    }
    Ok(full_denom)
}

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
    let config = CONFIG.load(deps.storage).unwrap();
    if config.owner != sender {
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
    let config = CONFIG.load(deps.storage)?;
    let is_denom_frozen = config.is_frozen && denom == config.denom;
    if is_denom_frozen {
        return Err(ContractError::ContractFrozen {
            denom: config.denom,
        });
    }

    Ok(())
}
