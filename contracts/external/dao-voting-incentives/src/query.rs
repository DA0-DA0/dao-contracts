use cosmwasm_std::{Deps, Env, StdError, StdResult};

use crate::{
    msg::RewardResponse,
    state::{self, Config, CONFIG},
};

pub fn rewards(deps: Deps, address: String) -> StdResult<RewardResponse> {
    let address = deps.api.addr_validate(&address)?;

    state::reward(deps, &address).map_err(|x| StdError::GenericErr { msg: x.to_string() })
}

pub fn expected_rewards(deps: Deps, env: Env, address: String) -> StdResult<RewardResponse> {
    let address = deps.api.addr_validate(&address)?;

    state::expected_reward(deps, env, &address)
        .map_err(|x| StdError::GenericErr { msg: x.to_string() })
}

pub fn config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}
