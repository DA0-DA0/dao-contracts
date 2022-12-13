#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, to_binary};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, TotalPowerResponse, VotingContractResponse};
use crate::state::{DISTRIBUTION_HEIGHT, TOTAL_POWER, VOTING_CONTRACT};

use dao_interface::voting;

const CONTRACT_NAME: &str = "crates.io:cw-fund-distributor";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // store the height
    DISTRIBUTION_HEIGHT.save(deps.storage, &env.block.height)?;

    // validate the contract and save it
    let voting_contract = deps.api.addr_validate(&msg.voting_contract)?;
    VOTING_CONTRACT.save(deps.storage, &voting_contract)?;

    let total_power: voting::TotalPowerAtHeightResponse = deps.querier.query_wasm_smart(
        voting_contract.clone(),
        &voting::Query::TotalPowerAtHeight {
            height: Some(env.block.height),
        },
    )?;
    // validate the total power and store it
    if total_power.power.is_zero() {
        return Err(ContractError::ZeroVotingPower {});
    }
    TOTAL_POWER.save(deps.storage, &total_power.power)?;

    Ok(Response::default()
        .add_attribute(
            "distribution_height",
            format!("{}", env.block.height),
        )
        .add_attribute("voting_contract", voting_contract)
        .add_attribute("total_power", total_power.power)
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    unimplemented!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::VotingContract {} => query_voting_contract(deps),
        QueryMsg::TotalPower {} => query_total_power(deps),
    }
}

pub fn query_voting_contract(deps: Deps) -> StdResult<Binary> {
    let contract = VOTING_CONTRACT.load(deps.storage)?;
    let distribution_height = DISTRIBUTION_HEIGHT.load(deps.storage)?;
    to_binary(&VotingContractResponse {
        contract,
        distribution_height,
    })
}

pub fn query_total_power(deps: Deps) -> StdResult<Binary> {
    let total_power  = TOTAL_POWER.load(deps.storage)?;
    to_binary(&TotalPowerResponse {
        total_power,
    })
}
