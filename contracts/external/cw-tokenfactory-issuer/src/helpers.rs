use crate::state::{
    ALLOWLIST, BEFORE_SEND_HOOK_FEATURES_ENABLED, DENOM, DENYLIST, IS_FROZEN, OWNER,
};
use crate::ContractError;
use cosmwasm_std::{Addr, Deps};

pub fn check_is_contract_owner(deps: Deps, sender: Addr) -> Result<(), ContractError> {
    let owner = OWNER.load(deps.storage)?;
    if owner != sender {
        Err(ContractError::Unauthorized {})
    } else {
        Ok(())
    }
}

pub fn check_before_send_hook_features_enabled(deps: Deps) -> Result<(), ContractError> {
    let enabled = BEFORE_SEND_HOOK_FEATURES_ENABLED.load(deps.storage)?;
    if !enabled {
        Err(ContractError::BeforeSendHookFeaturesDisabled {})
    } else {
        Ok(())
    }
}

pub fn check_is_not_denied(deps: Deps, address: String) -> Result<(), ContractError> {
    let addr = deps.api.addr_validate(&address)?;
    if let Some(is_denied) = DENYLIST.may_load(deps.storage, &addr)? {
        if is_denied {
            return Err(ContractError::Denied { address });
        }
    };
    Ok(())
}

pub fn check_is_not_frozen(deps: Deps, address: &str, denom: &str) -> Result<(), ContractError> {
    let is_frozen = IS_FROZEN.load(deps.storage)?;
    let contract_denom = DENOM.load(deps.storage)?;

    // check if issuer is configured to be frozen and the arriving denom is the same
    // as this contract denom.
    // Denom can be different since setting beforesend listener doesn't check
    // contract's denom.
    let is_denom_frozen = is_frozen && denom == contract_denom;
    if is_denom_frozen {
        let addr = deps.api.addr_validate(address)?;
        if let Some(is_allowed) = ALLOWLIST.may_load(deps.storage, &addr)? {
            if is_allowed {
                return Ok(());
            }
        };

        return Err(ContractError::ContractFrozen {
            denom: contract_denom,
        });
    }

    Ok(())
}
