use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    Addr, BlockInfo, DivideByZeroError, StdError, StdResult, Timestamp, Uint128, Uint256,
};
use cw20::Denom;
use cw_storage_plus::{Item, Map};
use cw_utils::{Duration, Expiration};

#[cw_serde]
pub struct Config {
    /// The address of a DAO DAO voting power module contract.
    pub vp_contract: Addr,
    /// An optional contract that is allowed to call the StakeChangedHook in
    /// place of the voting power contract.
    pub hook_caller: Option<Addr>,
    /// The Denom in which rewards are paid out.
    pub reward_denom: Denom,
}
pub const CONFIG: Item<Config> = Item::new("config");

#[cw_serde]
pub struct RewardConfig {
    pub period_finish_expiration: Expiration,
    pub reward_rate: Uint128,
    pub reward_duration: Duration,
}

impl RewardConfig {
    pub fn get_reward_rate(&self, amount: Uint128) -> StdResult<Uint128> {
        // depending on whether the reward duration is specified in blocks
        // or in time, we calculate reward rate differently
        let duration_units = match self.reward_duration {
            Duration::Height(h) => Uint128::from(h),
            Duration::Time(t) => Uint128::from(t),
        };

        amount
            .checked_div(duration_units)
            .map_err(|e| StdError::divide_by_zero(e))
    }

    /// return the minimum of the current block and the period finish block,
    /// depending on the `reward_duration` configuration
    pub fn get_last_time_reward_applicable(&self, block: BlockInfo) -> BlockInfo {
        if self.period_finish_expiration.is_expired(&block) {
            block
        } else {
            let mut expiration_block = block.clone();
            match self.reward_duration {
                Duration::Height(h) => {
                    expiration_block.height = h;
                }
                Duration::Time(t) => {
                    expiration_block.time = Timestamp::from_seconds(t);
                }
            };
            expiration_block
        }
    }
}

pub const REWARD_CONFIG: Item<RewardConfig> = Item::new("reward_config");

pub const REWARD_PER_TOKEN: Item<Uint256> = Item::new("reward_per_token");

pub const LAST_UPDATE_BLOCK: Item<BlockInfo> = Item::new("last_update_block");

/// A map of user addresses to their pending rewards.
pub const PENDING_REWARDS: Map<Addr, Uint128> = Map::new("pending_rewards");

/// A map of user addresses to their rewards per token. In other words, it is the
/// reward per share of voting power that the user has.
pub const USER_REWARD_PER_TOKEN: Map<Addr, Uint256> = Map::new("user_reward_per_token");
