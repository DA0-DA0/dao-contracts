#[cfg(not(feature = "library"))]
use cosmwasm_std::{Coin, DepsMut, Response};

use crate::error::ContractError;
use crate::state::CONFIG;
use crate::helpers::check_is_not_blacklisted;

pub fn blockbeforesend_hook(
    deps: DepsMut,
    from: String,
    to: String,
    amount: Coin,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if config.is_frozen {
        if amount.denom == config.denom {
            return Err(ContractError::ContractFrozen {
                denom: config.denom,
            });
        }
    }

    // assert that neither 'from' or 'to' address is blacklisted
    check_is_not_blacklisted(deps.as_ref(), from)?;
    check_is_not_blacklisted(deps.as_ref(), to)?;

    Ok(Response::new().add_attribute("action", "blcok_before_send"))
}
