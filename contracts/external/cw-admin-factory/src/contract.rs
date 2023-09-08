#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Attribute, BankMsg, Binary, Coin, Coins, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdResult, SubMsg, WasmMsg,
};

use cw2::set_contract_version;
use cw_ownable::{assert_owner, get_ownership, initialize_owner, update_ownership};
use cw_utils::parse_reply_instantiate_data;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::FEE;

pub(crate) const CONTRACT_NAME: &str = "crates.io:cw-admin-factory";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const INSTANTIATE_CONTRACT_REPLY_ID: u64 = 0;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let ownership = initialize_owner(deps.storage, deps.api, msg.owner.as_deref())?;
    let attributes = update_fee_inner(deps, msg.fee)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("creator", info.sender)
        .add_attributes(attributes)
        .add_attributes(ownership.into_attributes()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::InstantiateContractWithSelfAdmin {
            instantiate_msg: msg,
            code_id,
            label,
        } => instantiate_contract(deps, env, info, msg, code_id, label),
        ExecuteMsg::UpdateFee { fee } => execute_update_fee(deps, info, fee),
        ExecuteMsg::UpdateOwnership(action) => {
            let ownership = update_ownership(deps, &env.block, &info.sender, action)?;
            Ok(Response::default().add_attributes(ownership.into_attributes()))
        }
    }
}

pub fn instantiate_contract(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    instantiate_msg: Binary,
    code_id: u64,
    label: String,
) -> Result<Response, ContractError> {
    // Validate and get coins struct
    let mut funds = Coins::try_from(info.funds)?;

    // Check for a fee and attach a bank send message if found
    let mut response = Response::default();
    match FEE.may_load(deps.storage)? {
        Some(fee) => {
            // Get the funds recipient
            let ownership = get_ownership(deps.storage)?;

            if ownership.owner.is_none() {
                return Err(ContractError::Ownership(
                    cw_ownable::OwnershipError::NoOwner {},
                ));
            }

            // Subtract the fee from the funds
            for coin in &fee {
                funds.sub(coin.clone())?;
            }

            let msg = BankMsg::Send {
                to_address: ownership.owner.unwrap().to_string(),
                amount: fee,
            };

            response = response.add_message(msg);
        }
        None => {}
    };

    // Instantiate the specified contract with factory as the admin.
    let instantiate = WasmMsg::Instantiate {
        admin: Some(env.contract.address.to_string()),
        code_id,
        msg: instantiate_msg,
        funds: funds.into_vec(),
        label,
    };

    let msg = SubMsg::reply_on_success(instantiate, INSTANTIATE_CONTRACT_REPLY_ID);

    Ok(response
        .add_attribute("action", "instantiate_cw_core")
        .add_submessage(msg))
}

pub fn execute_update_fee(
    deps: DepsMut,
    info: MessageInfo,
    fee: Option<Vec<Coin>>,
) -> Result<Response, ContractError> {
    assert_owner(deps.storage, &info.sender)?;

    let attributes = update_fee_inner(deps, fee)?;

    Ok(Response::default()
        .add_attribute("action", "execute_update_fee")
        .add_attribute("sender", info.sender)
        .add_attributes(attributes))
}

/// Updates the fee configuration and returns the fee attributes
fn update_fee_inner(
    deps: DepsMut,
    fee: Option<Vec<Coin>>,
) -> Result<Vec<Attribute>, ContractError> {
    let fee = fee.map(|x| Coins::try_from(x)).transpose()?;
    let fee_string = fee.as_ref().map_or("None".to_owned(), ToString::to_string);

    match fee {
        Some(fee) => FEE.save(deps.storage, &fee.into_vec())?,
        None => FEE.remove(deps.storage),
    }

    Ok(vec![Attribute {
        key: "fee".to_owned(),
        value: fee_string,
    }])
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_binary(&get_ownership(deps.storage)?),
        QueryMsg::Fee {} => to_binary(&FEE.may_load(deps.storage)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_CONTRACT_REPLY_ID => {
            let res = parse_reply_instantiate_data(msg)?;
            let contract_addr = deps.api.addr_validate(&res.contract_address)?;
            // Make the contract its own admin.
            let msg = WasmMsg::UpdateAdmin {
                contract_addr: contract_addr.to_string(),
                admin: contract_addr.to_string(),
            };

            Ok(Response::default()
                .add_attribute("set contract admin as itself", contract_addr)
                .add_message(msg))
        }
        _ => Err(ContractError::UnknownReplyID {}),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
