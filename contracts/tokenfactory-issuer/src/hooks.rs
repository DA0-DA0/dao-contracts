#[cfg(not(feature = "library"))]
use cosmwasm_std::{Coin, DepsMut, Response};

use crate::error::ContractError;
use crate::helpers::check_is_not_blacklisted;
use crate::state::{CONFIG, FREEZER_ALLOWANCES};

pub fn beforesend_hook(
    deps: DepsMut,
    from: String,
    to: String,
    coin: Coin,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let from_addr = deps.api.addr_validate(&from)?;

    // TODO: extract to: check_if_not_frozen
    let is_denom_frozen = config.is_frozen && coin.denom == config.denom;
    let is_from_frozen_address = FREEZER_ALLOWANCES
        .may_load(deps.storage, &from_addr)?
        .unwrap_or(false); // not frozen by default

    if is_denom_frozen || is_from_frozen_address {
        return Err(ContractError::ContractFrozen {
            denom: config.denom,
        });
    }

    // assert that neither 'from' or 'to' address is blacklisted
    check_is_not_blacklisted(deps.as_ref(), from)?;
    check_is_not_blacklisted(deps.as_ref(), to)?;

    Ok(Response::new().add_attribute("action", "before_send"))
}
