#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use cw_ownable::get_ownership;

use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::PROPOSAL_INCENTIVES;
use crate::{execute, query, ContractError};

pub(crate) const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Save ownership
    let ownership = cw_ownable::initialize_owner(deps.storage, deps.api, Some(&msg.owner))?;

    // Validate proposal incentives
    let proposal_incentives = msg.proposal_incentives.into_checked(deps.as_ref())?;

    // Save proposal incentives config
    PROPOSAL_INCENTIVES.save(deps.storage, &proposal_incentives, env.block.height)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("creator", info.sender)
        .add_attributes(ownership.into_attributes())
        .add_attributes(proposal_incentives.into_attributes()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ProposalHook(msg) => execute::proposal_hook(deps, env, info, msg),
        ExecuteMsg::UpdateOwnership(action) => execute::update_ownership(deps, env, info, action),
        ExecuteMsg::UpdateProposalIncentives {
            proposal_incentives,
        } => execute::update_proposal_incentives(deps, env, info, proposal_incentives),
        ExecuteMsg::Receive(cw20_receive_msg) => {
            execute::receive_cw20(deps, env, info, cw20_receive_msg)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ProposalIncentives { height } => {
            to_json_binary(&query::proposal_incentives(deps, height)?)
        }
        QueryMsg::Ownership {} => to_json_binary(&get_ownership(deps.storage)?),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
