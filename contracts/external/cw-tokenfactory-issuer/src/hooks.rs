use cosmwasm_std::{Coin, DepsMut, Response};

use crate::error::ContractError;
use crate::helpers::{check_is_not_denied, check_is_not_frozen};

pub fn beforesend_hook(
    deps: DepsMut,
    from: String,
    to: String,
    coin: Coin,
) -> Result<Response, ContractError> {
    // assert that denom of this contract is not frozen
    check_is_not_frozen(deps.as_ref(), &from, &coin.denom)?;

    // assert that neither 'from' or 'to' address is denylist
    check_is_not_denied(deps.as_ref(), from)?;
    check_is_not_denied(deps.as_ref(), to)?;

    Ok(Response::new().add_attribute("action", "before_send"))
}
