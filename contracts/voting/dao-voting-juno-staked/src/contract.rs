#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::{ensure_from_older_version, set_contract_version};
use dao_interface::voting::{
    InfoResponse, TotalPowerAtHeightResponse, VotingPowerAtHeightResponse,
};

use crate::bindings::{JunoQuerier, JunoQuery};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::DAO;

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-voting-juno-staked";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<JunoQuery>,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    DAO.save(deps.storage, &info.sender)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("dao", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut<JunoQuery>,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    Err(ContractError::NoExecute {})
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<JunoQuery>, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::VotingPowerAtHeight { address, height } => {
            to_json_binary(&query_voting_power_at_height(deps, env, address, height)?)
        }
        QueryMsg::TotalPowerAtHeight { height } => {
            to_json_binary(&query_total_power_at_height(deps, env, height)?)
        }
        QueryMsg::Dao {} => to_json_binary(&DAO.load(deps.storage)?),
        QueryMsg::Info {} => to_json_binary(&InfoResponse {
            info: cw2::get_contract_version(deps.storage)?,
        }),
    }
}

fn query_voting_power_at_height(
    deps: Deps<JunoQuery>,
    env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<VotingPowerAtHeightResponse> {
    let height = height.unwrap_or(env.block.height);
    let address = deps.api.addr_validate(&address)?;
    let power = deps
        .querier
        .voting_power_at(address.to_string(), snapshot_height(height)?)?;
    Ok(VotingPowerAtHeightResponse { power, height })
}

fn query_total_power_at_height(
    deps: Deps<JunoQuery>,
    env: Env,
    height: Option<u64>,
) -> StdResult<TotalPowerAtHeightResponse> {
    let height = height.unwrap_or(env.block.height);
    let power = deps
        .querier
        .total_voting_power_at(snapshot_height(height)?)?;
    Ok(TotalPowerAtHeightResponse { power, height })
}

/// DAO DAO fixes proposal power at the beginning of proposal block `h`.
/// Juno records settled staking state at the end of each block, so the
/// equivalent chain snapshot is `h - 1`. Height zero is retained as zero for
/// defensive genesis-boundary queries.
fn snapshot_height(dao_height: u64) -> StdResult<u64> {
    let snapshot_height = dao_height.saturating_sub(1);
    i64::try_from(snapshot_height).map_err(|_| {
        cosmwasm_std::StdError::generic_err(format!(
            "DAO query height {dao_height} translates to snapshot height \
             {snapshot_height}, which exceeds i64::MAX"
        ))
    })?;
    Ok(snapshot_height)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(
    deps: DepsMut<JunoQuery>,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response, ContractError> {
    let previous = ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    if previous.to_string() == CONTRACT_VERSION {
        return Err(cosmwasm_std::StdError::generic_err(format!(
            "cannot migrate from the current version {CONTRACT_VERSION}"
        ))
        .into());
    }
    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", previous.to_string())
        .add_attribute("to_version", CONTRACT_VERSION))
}
