#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_json_binary, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env,
    MessageInfo, Order, Response, StdError, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    msg::{
        AdapterQueryMsg, AllOptionsResponse, CheckOptionResponse, ExecuteMsg, InstantiateMsg,
        MigrateMsg, QueryMsg, SampleGaugeMsgsResponse,
    },
    state::{Config, CONFIG, OPTIONS},
};

const CONTRACT_NAME: &str = "crates.io:gauge-budget-allocator";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    if msg.options.is_empty() {
        return Err(ContractError::NoOptions {});
    }

    cw_ownable::initialize_owner(deps.storage, deps.api, Some(&msg.owner))?;

    CONFIG.save(
        deps.storage,
        &Config {
            epoch_budget: msg.epoch_budget,
        },
    )?;

    for option in msg.options {
        OPTIONS.save(deps.storage, option.as_str(), &())?;
    }

    Ok(Response::new().add_attribute("action", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // UpdateOwnership runs its own auth — gate everything else here.
    if !matches!(msg, ExecuteMsg::UpdateOwnership(_)) {
        cw_ownable::assert_owner(deps.storage, &info.sender)?;
    }

    match msg {
        ExecuteMsg::AddOption { option } => {
            if OPTIONS.has(deps.storage, option.as_str()) {
                return Err(ContractError::OptionAlreadyExists(option));
            }
            OPTIONS.save(deps.storage, option.as_str(), &())?;
            Ok(Response::new()
                .add_attribute("action", "add_option")
                .add_attribute("option", option))
        }
        ExecuteMsg::RemoveOption { option } => {
            if !OPTIONS.has(deps.storage, option.as_str()) {
                return Err(ContractError::OptionDoesNotExist(option));
            }
            OPTIONS.remove(deps.storage, option.as_str());
            Ok(Response::new()
                .add_attribute("action", "remove_option")
                .add_attribute("option", option))
        }
        ExecuteMsg::UpdateBudget { epoch_budget } => {
            CONFIG.update(deps.storage, |mut c| -> StdResult<_> {
                c.epoch_budget = epoch_budget.clone();
                Ok(c)
            })?;
            Ok(Response::new()
                .add_attribute("action", "update_budget")
                .add_attribute("denom", &epoch_budget.denom)
                .add_attribute("amount", epoch_budget.amount.to_string()))
        }
        ExecuteMsg::UpdateOwnership(action) => {
            let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
            Ok(Response::new().add_attributes(ownership.into_attributes()))
        }
    }
}

/// Native (non-orchestrator) query entrypoint. Accepts `QueryMsg`, which is
/// a superset of `AdapterQueryMsg` (it adds `Config {}`). The orchestrator
/// only ever sends `AdapterQueryMsg` variants, which we translate below.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::AllOptions {} => to_json_binary(&all_options(deps)?),
        QueryMsg::CheckOption { option } => to_json_binary(&check_option(deps, option)),
        QueryMsg::SampleGaugeMsgs { selected } => {
            to_json_binary(&sample_gauge_msgs(deps, selected)?)
        }
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
    }
}

/// Convenience: dispatch a raw `AdapterQueryMsg` (what the orchestrator
/// sends) through the same handlers. Useful for integration tests that
/// want to confirm orchestrator-compat without writing two variants.
pub fn answer_adapter(deps: Deps, msg: AdapterQueryMsg) -> StdResult<Binary> {
    match msg {
        AdapterQueryMsg::AllOptions {} => to_json_binary(&all_options(deps)?),
        AdapterQueryMsg::CheckOption { option } => to_json_binary(&check_option(deps, option)),
        AdapterQueryMsg::SampleGaugeMsgs { selected } => {
            to_json_binary(&sample_gauge_msgs(deps, selected)?)
        }
        // The orchestrator never sends these to a non-marketing adapter; if
        // it ever does, fail loudly rather than silently returning empty.
        AdapterQueryMsg::Config {}
        | AdapterQueryMsg::Submission { .. }
        | AdapterQueryMsg::AllSubmissions {}
        | AdapterQueryMsg::SubmissionsBySender { .. } => Err(StdError::generic_err(
            "gauge-budget-allocator does not implement registry-style queries",
        )),
        AdapterQueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
    }
}

fn all_options(deps: Deps) -> StdResult<AllOptionsResponse> {
    Ok(AllOptionsResponse {
        options: OPTIONS
            .keys(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?,
    })
}

fn check_option(deps: Deps, option: String) -> CheckOptionResponse {
    CheckOptionResponse {
        valid: OPTIONS.has(deps.storage, option.as_str()),
    }
}

fn sample_gauge_msgs(
    deps: Deps,
    selected: Vec<(String, Decimal)>,
) -> StdResult<SampleGaugeMsgsResponse> {
    let Config { epoch_budget, .. } = CONFIG.load(deps.storage)?;
    let execute = selected
        .into_iter()
        .map(|(to_address, weight)| -> StdResult<CosmosMsg> {
            let amount = epoch_budget
                .amount
                .checked_mul_floor(weight)
                .map_err(|e| StdError::generic_err(e.to_string()))?;
            Ok(send_message(to_address, &epoch_budget, amount))
        })
        .collect::<StdResult<Vec<CosmosMsg>>>()?;
    Ok(SampleGaugeMsgsResponse { execute })
}

fn send_message(to: String, budget: &Coin, amount: Uint128) -> CosmosMsg {
    CosmosMsg::Bank(BankMsg::Send {
        to_address: to,
        amount: vec![coin(amount.u128(), budget.denom.clone())],
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::new())
}
