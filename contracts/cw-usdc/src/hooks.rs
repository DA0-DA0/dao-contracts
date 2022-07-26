#[cfg(not(feature = "library"))]
use cosmwasm_std::{Coin, DepsMut, Response};

use crate::error::ContractError;
use crate::state::{BLACKLISTED_ADDRESSES, CONFIG};

pub fn beforesend_hook(
    deps: DepsMut,
    from: String,
    to: String,
    amount: Vec<Coin>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if config.is_frozen {
        for coin in amount {
            if coin.denom == config.denom {
                return Err(ContractError::ContractFrozen {
                    denom: config.denom,
                });
            }
        }
    }

    // Check if 'from' address is blacklisted
    let from_address = deps.api.addr_validate(&from)?;
    if let Some(is_blacklisted) = BLACKLISTED_ADDRESSES.may_load(deps.storage, &from_address)? {
        if is_blacklisted {
            return Err(ContractError::Blacklisted { address: from });
        }
    };

    // Check if 'to' address is blacklisted
    let to_address = deps.api.addr_validate(&to)?;
    if let Some(is_blacklisted) = BLACKLISTED_ADDRESSES.may_load(deps.storage, &to_address)? {
        if is_blacklisted {
            return Err(ContractError::Blacklisted { address: to });
        }
    };

    Ok(Response::new().add_attribute("method", "before_send"))
}
