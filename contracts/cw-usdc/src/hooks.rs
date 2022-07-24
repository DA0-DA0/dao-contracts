#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;

use cw_storage_plus::Map;
use osmo_bindings::OsmosisMsg;

// use osmo_bindings_test::OsmosisModule;

use crate::error::ContractError;
use crate::helpers::build_denom;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, SudoMsg};
use crate::queries;
use crate::state::{
    Config, BLACKLISTED_ADDRESSES, BLACKLISTER_ALLOWANCES, BURNER_ALLOWANCES, CONFIG,
    FREEZER_ALLOWANCES, MINTER_ALLOWANCES,
};

pub fn beforesend_hook(
    deps: DepsMut,
    from: String,
    to: String,
    amount: Vec<Coin>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if config.is_frozen {
        for coin in amount.clone() {
            if coin.denom == config.denom {
                return Err(ContractError::ContractFrozen {
                    denom: config.denom,
                });
            }
        }
    }

    // Check if 'from' address is blacklisted
    let from_address = deps.api.addr_validate(from.as_str())?;
    if let Some(is_blacklisted) = BLACKLISTED_ADDRESSES.may_load(deps.storage, &from_address)? {
        if is_blacklisted {
            return Err(ContractError::Blacklisted { address: from });
        }
    };

    // Check if 'to' address is blacklisted
    let to_address = deps.api.addr_validate(to.as_str())?;
    if let Some(is_blacklisted) = BLACKLISTED_ADDRESSES.may_load(deps.storage, &to_address)? {
        if is_blacklisted {
            return Err(ContractError::Blacklisted { address: to });
        }
    };

    Ok(Response::new().add_attribute("method", "before_send"))
}
