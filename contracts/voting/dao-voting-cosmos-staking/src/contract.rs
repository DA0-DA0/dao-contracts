#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;
use dao_interface::voting::{TotalPowerAtHeightResponse, VotingPowerAtHeightResponse};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, GetHooksResponse, InstantiateMsg, QueryMsg, SudoMsg};
use crate::state::{CONFIG, DAO, HOOKS, STAKED_BALANCES, STAKED_TOTAL};

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-voting-cosmos-staking";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    Err(ContractError::NoExecution {})
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::VotingPowerAtHeight { address, height } => {
            to_json_binary(&query_voting_power_at_height(deps, env, address, height)?)
        }
        QueryMsg::TotalPowerAtHeight { height } => {
            to_json_binary(&query_total_power_at_height(deps, env, height)?)
        }
        QueryMsg::Info {} => query_info(deps),
        QueryMsg::Dao {} => query_dao(deps),
        QueryMsg::GetConfig {} => to_json_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::GetHooks {} => to_json_binary(&query_hooks(deps)?),
    }
}

pub fn query_voting_power_at_height(
    deps: Deps,
    env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<VotingPowerAtHeightResponse> {
    let height = height.unwrap_or(env.block.height);
    let delegator_addr = deps.api.addr_validate(&address)?;
    let power = STAKED_BALANCES.may_load_at_height(deps.storage, &delegator_addr, height)?;

    // If there is voting power history, we use that. If not, we try manually
    // querying delegations, as the user has not changed delegations since we
    // started capturing changes with the hooks.
    match power {
        Some(power) => Ok(VotingPowerAtHeightResponse { power, height }),
        None => {
            let power = get_total_delegations(deps, address)?;
            Ok(VotingPowerAtHeightResponse { power, height })
        }
    }
}

pub fn query_total_power_at_height(
    deps: Deps,
    env: Env,
    height: Option<u64>,
) -> StdResult<TotalPowerAtHeightResponse> {
    let height = height.unwrap_or(env.block.height);
    let power = STAKED_TOTAL.may_load_at_height(deps.storage, height)?;

    match power {
        // If we have history, which should be the case 99.9% of the time
        // we return the total power
        Some(power) => Ok(TotalPowerAtHeightResponse { power, height }),
        // TODO: fallback to getting the total stake
        // We may not have history if no one has staked or unstaked since this
        // contract was instantiated, in that case we fall back to a manual query
        None => unimplemented!(),
    }
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_json_binary(&dao_interface::voting::InfoResponse { info })
}

pub fn query_dao(deps: Deps) -> StdResult<Binary> {
    let dao = DAO.load(deps.storage)?;
    to_json_binary(&dao)
}

pub fn query_hooks(deps: Deps) -> StdResult<GetHooksResponse> {
    Ok(GetHooksResponse {
        hooks: HOOKS.query_hooks(deps)?.hooks,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn sudo(deps: DepsMut, env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        // TODO: a validator falls out of the active set. Should we check after a validator is slashed / unbonded from the active set ie. AfterValidatorBeginUnbonding
        // SudoMsg::BeforeDelegationRemoved {
        //     validator_address,
        //     delegator_address,
        //     shares,
        // } => delegation_change(deps, delegator_address, env.block.height),
        SudoMsg::AfterDelegationModified {
            delegator_address, ..
        } => delegation_change(deps, delegator_address, env.block.height),
    }
}

/// delegation_change is called any time the chain has delegation events emited.
/// On the change, we update this for
fn delegation_change(
    deps: DepsMut,
    delegator: String,
    block_height: u64,
) -> Result<Response, ContractError> {
    let delegator_addr = deps.api.addr_validate(&delegator)?;

    let amount_staked = get_total_delegations(deps.as_ref(), delegator)?;

    // TODO:
    STAKED_BALANCES.update(
        deps.storage,
        &delegator_addr,
        block_height,
        |balance| -> StdResult<Uint128> {
            Ok(balance.unwrap_or_default().checked_add(amount_staked)?)
        },
    )?;

    // TODO get all the total stake at the current height (pool.bonded_tokens)
    // "/cosmos.staking.v1beta1.Query/Pool": &stakingtypes.QueryPoolResponse{}, // https://lcd.juno.strange.love/cosmos/staking/v1beta1/pool

    STAKED_TOTAL.update(deps.storage, block_height, |total| -> StdResult<Uint128> {
        Ok(total.unwrap_or_default().checked_add(amount_staked)?)
    })?;

    // TODO add attributes
    Ok(Response::new())
}

fn get_total_delegations(deps: Deps, delegator: String) -> StdResult<Uint128> {
    let delegations = deps.querier.query_all_delegations(delegator)?;

    let mut amount_staked = Uint128::zero();

    // iter delegations
    delegations.iter().for_each(|delegation| {
        amount_staked += delegation.amount.amount;
    });

    Ok(amount_staked)
}
