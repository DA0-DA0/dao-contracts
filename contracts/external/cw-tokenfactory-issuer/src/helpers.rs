use crate::state::{ALLOWLIST, BEFORE_SEND_HOOK_INFO, DENOM, DENYLIST, IS_FROZEN};
use crate::ContractError;
use cosmwasm_std::Deps;

/// Checks wether the BeforeSendHookFeatures gated features are enabled
pub fn check_before_send_hook_features_enabled(deps: Deps) -> Result<(), ContractError> {
    let info = BEFORE_SEND_HOOK_INFO.load(deps.storage)?;
    if !info.advanced_features_enabled {
        Err(ContractError::BeforeSendHookFeaturesDisabled {})
    } else {
        Ok(())
    }
}

/// Checks wether the given address is on the denylist
pub fn check_is_not_denied(deps: Deps, address: String) -> Result<(), ContractError> {
    let addr = deps.api.addr_validate(&address)?;
    if let Some(is_denied) = DENYLIST.may_load(deps.storage, &addr)? {
        if is_denied {
            return Err(ContractError::Denied { address });
        }
    };
    Ok(())
}

/// Checks wether the contract is frozen for the given denom, in which case
/// token transfers will not be allowed unless the to or from address is on
/// the allowlist
pub fn check_is_not_frozen(
    deps: Deps,
    from_address: &str,
    to_address: &str,
    denom: &str,
) -> Result<(), ContractError> {
    let is_frozen = IS_FROZEN.load(deps.storage)?;
    let contract_denom = DENOM.load(deps.storage)?;

    // Check if issuer is configured to be frozen and the arriving denom is the same
    // as this contract denom.
    //
    // Denom can be different since setting beforesend listener doesn't check
    // contract's denom.
    let is_denom_frozen = is_frozen && denom == contract_denom;
    if is_denom_frozen {
        let from = deps.api.addr_validate(from_address)?;
        let to = deps.api.addr_validate(to_address)?;

        // If either the from address or the to_address is allowed, then transaction proceeds
        let is_from_allowed = ALLOWLIST.may_load(deps.storage, &from)?;
        let is_to_allowed = ALLOWLIST.may_load(deps.storage, &to)?;
        match (is_from_allowed, is_to_allowed) {
            (Some(true), _) => return Ok(()),
            (_, Some(true)) => return Ok(()),
            _ => {
                return Err(ContractError::ContractFrozen {
                    denom: contract_denom,
                })
            }
        }
    }

    Ok(())
}
