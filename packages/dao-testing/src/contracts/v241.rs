//! v2.4.1 contract test wrappers — disabled in this binary.
//!
//! dao-dao-core 2.4.1 (and the matching v2.4.1 friends) pin cosmwasm-std
//! 1.5.5. cw-multi-test 2.x's `ContractWrapper` only accepts cosmwasm-std
//! 2.x entry-point signatures, so the original v241 wrappers fail to
//! compile under the cw-std 2.x bump.
//!
//! Each function below returns a *stub* contract that errors out at
//! runtime. The goal is keeping the workspace + downstream test crates
//! compiling until the v2.4.1 -> v2.9+ migration shim lands.

use cosmwasm_std::{
    Binary, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response, StdError, StdResult,
};
use cw_multi_test::{Contract, ContractWrapper};

const ERR: &str = "v2.4.1 contract test wrapper is disabled in this binary; see packages/dao-testing/src/contracts/v241.rs";

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

pub fn dao_dao_core_v241_contract() -> Box<dyn Contract<Empty>> {
    stub_contract()
}

pub fn dao_voting_cw4_v241_contract() -> Box<dyn Contract<Empty>> {
    stub_contract()
}

pub fn dao_proposal_single_v241_contract() -> Box<dyn Contract<Empty>> {
    stub_contract()
}

pub fn dao_proposal_multiple_v241_contract() -> Box<dyn Contract<Empty>> {
    stub_contract()
}

pub fn dao_pre_propose_single_v241_contract() -> Box<dyn Contract<Empty>> {
    stub_contract()
}

pub fn dao_pre_propose_approval_single_v241_contract() -> Box<dyn Contract<Empty>> {
    stub_contract()
}

pub fn dao_pre_propose_multiple_v241_contract() -> Box<dyn Contract<Empty>> {
    stub_contract()
}
