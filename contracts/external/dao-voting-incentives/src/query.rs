use cosmwasm_std::{Deps, Env, StdError, StdResult, Uint128};

use crate::{
    msg::RewardResponse,
    state::{self, Config, CONFIG, USER_VOTE_COUNT},
};

pub fn rewards(deps: Deps, env: Env, address: String) -> StdResult<RewardResponse> {
    let address = deps.api.addr_validate(&address)?;

    state::reward(deps, &env.contract.address, &address)
        .map_err(|x| StdError::GenericErr { msg: x.to_string() })
}

pub fn config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

pub fn votes(deps: Deps, address: String) -> StdResult<Uint128> {
    let address = deps.api.addr_validate(&address)?;

    Ok(USER_VOTE_COUNT
        .may_load(deps.storage, &address)?
        .unwrap_or_default())
}
