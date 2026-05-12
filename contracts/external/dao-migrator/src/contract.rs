//! dao-migrator entry points (v2.9+ stub).
//!
//! The original dao-migrator contract bridged v1 DAOs (cosmwasm-std 1.x) to v2.
//! With the workspace on cosmwasm-std 2.x, the v1 type universe (`cw-core-v1`,
//! `cw-proposal-single-v1`, `cw-utils-v1`, `voting-v1`) compiles against a
//! distinct cosmwasm-std crate; types like `Addr`, `Uint128`, `Decimal`,
//! `Timestamp`, and `CosmosMsg` are no longer cross-compatible at the Rust
//! type level. The v1 -> v2.9+ migration story is deferred to a Stage 3
//! migration shim that will read raw storage bytes instead of relying on the
//! v1 typed storage handles.
//!
//! Until that shim lands, every entry point here returns
//! [`ContractError::V1MigrationUnsupported`]. The crate stays in the workspace
//! so downstream consumers (`dao-testing`, schema tooling) keep compiling.

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError, StdResult};
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
};

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-migrator";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Err(ContractError::V1MigrationUnsupported {})
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    Err(ContractError::V1MigrationUnsupported {})
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    Err(StdError::generic_err(
        "dao-migrator v1 -> v2.9+ query path is disabled; see contract docstring",
    ))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, _reply: Reply) -> Result<Response, ContractError> {
    Err(ContractError::V1MigrationUnsupported {})
}
