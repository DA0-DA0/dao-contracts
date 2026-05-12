//! v1 contract test wrappers — disabled in this binary.
//!
//! The upstream v1 contracts (cw-core 0.1.0, cw-proposal-single 0.1.0,
//! cw4-voting 0.1.0, cw20-stake 0.2.6, stake-cw20 0.2.6, etc.) all pin
//! cosmwasm-std 1.x. cw-multi-test 2.x's `ContractWrapper` only accepts
//! cosmwasm-std 2.x entry-point signatures, so the original `v1.rs`
//! wrappers fail to compile under the cw-std 2.x bump.
//!
//! Each function below returns a *stub* contract that immediately errors
//! out. Callers that depend on real v1 behaviour will see a runtime error
//! during their test; the goal is to keep the workspace + downstream test
//! crates compiling until the v1 → v2.9+ migration shim lands.

use cosmwasm_std::{
    Binary, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response, StdError, StdResult,
};
use cw_multi_test::{Contract, ContractWrapper};

const ERR: &str = "v1 contract test wrapper is disabled in this binary; see packages/dao-testing/src/contracts/v1.rs";

fn stub_execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Binary,
) -> StdResult<Response> {
    Err(StdError::generic_err(ERR))
}

fn stub_instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Binary,
) -> StdResult<Response> {
    Err(StdError::generic_err(ERR))
}

fn stub_query(_deps: Deps, _env: Env, _msg: Binary) -> StdResult<Binary> {
    Err(StdError::generic_err(ERR))
}

fn stub_reply(_deps: DepsMut, _env: Env, _msg: Reply) -> StdResult<Response> {
    Err(StdError::generic_err(ERR))
}

fn stub_migrate(_deps: DepsMut, _env: Env, _msg: Binary) -> StdResult<Response> {
    Err(StdError::generic_err(ERR))
}

fn stub_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(stub_execute, stub_instantiate, stub_query)
            .with_reply(stub_reply)
            .with_migrate(stub_migrate),
    )
}

pub fn cw_proposal_single_v1_contract() -> Box<dyn Contract<Empty>> {
    stub_contract()
}

pub fn cw_core_v1_contract() -> Box<dyn Contract<Empty>> {
    stub_contract()
}

pub fn cw4_voting_v1_contract() -> Box<dyn Contract<Empty>> {
    stub_contract()
}

pub fn cw20_stake_v1_contract() -> Box<dyn Contract<Empty>> {
    stub_contract()
}

pub fn stake_cw20_v03_contract() -> Box<dyn Contract<Empty>> {
    stub_contract()
}

pub fn cw20_stake_external_rewards_v1_contract() -> Box<dyn Contract<Empty>> {
    stub_contract()
}

pub fn cw20_stake_reward_distributor_v1_contract() -> Box<dyn Contract<Empty>> {
    stub_contract()
}
