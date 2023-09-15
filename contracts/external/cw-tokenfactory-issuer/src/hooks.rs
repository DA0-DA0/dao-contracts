use cosmwasm_std::{Coin, DepsMut, Response};

use crate::error::ContractError;
use crate::helpers::{check_is_not_denied, check_is_not_frozen};

/// The before send hook is called before every token transfer on chains that
/// support MsgSetBeforeSendHook.
///
/// It is called by the bank module.
pub fn beforesend_hook(
    deps: DepsMut,
    from: String,
    to: String,
    coin: Coin,
) -> Result<Response, ContractError> {
    // Assert that denom of this contract is not frozen
    // If it is frozen, check whether either 'from' or 'to' address is allowed
    check_is_not_frozen(deps.as_ref(), &from, &to, &coin.denom)?;

    // Assert that neither 'from' or 'to' address is denylist
    check_is_not_denied(deps.as_ref(), from)?;
    check_is_not_denied(deps.as_ref(), to)?;

    Ok(Response::new().add_attribute("action", "before_send"))
}
