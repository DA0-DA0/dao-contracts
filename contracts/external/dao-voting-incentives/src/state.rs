use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, CheckedMultiplyFractionError, Deps, Uint128};
use cw_denom::CheckedDenom;
use cw_storage_plus::{Item, Map};
use cw_utils::Expiration;
use dao_interface::proposal::GenericProposalInfo;

use crate::{msg::RewardResponse, ContractError};

/// Incentives for voting
#[cw_serde]
pub struct Config {
    /// The start height of the voting incentives
    pub start_height: u64,
    /// The expiration of these voting incentives
    pub expiration: Expiration,
    /// The total rewards to be distributed
    pub denom: CheckedDenom,
    /// The total number of votes
    pub total_votes: Uint128,
    /// The balance at expiration
    pub expiration_balance: Option<Uint128>,
}

/// A map of user address to vote count
pub const USER_VOTE_COUNT: Map<&Addr, Uint128> = Map::new("user_vote_count");
/// A map of user address with proposal id to has voted value
/// This map is useful for cases where a proposal module allows revoting, so users cannot spam votes for more rewards
pub const USER_PROPOSAL_HAS_VOTED: Map<(&Addr, u64), bool> = Map::new("user_proposal_has_voted");
/// The voting incentives config
pub const CONFIG: Item<Config> = Item::new("config");
/// A cache of generic proposal information (proposal_module, proposal_id)
pub const GENERIC_PROPOSAL_INFO: Map<(&Addr, u64), GenericProposalInfo> =
    Map::new("generic_proposal_info");

/// A method to load reward information
pub fn reward(deps: Deps, contract: &Addr, addr: &Addr) -> Result<RewardResponse, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Get the user's votes
    let user_votes = USER_VOTE_COUNT
        .may_load(deps.storage, addr)?
        .unwrap_or_default();

    match config.expiration_balance {
        Some(balance) => {
            // Calculate the reward
            Ok(RewardResponse {
                denom: config.denom,
                amount: calculate_reward(config.total_votes, user_votes, balance)?,
                is_claimable: true,
            })
        }
        None => {
            // Get the current voting incentives balance
            let balance = config.denom.query_balance(&deps.querier, contract)?;

            // Calculate the reward
            Ok(RewardResponse {
                denom: config.denom,
                amount: calculate_reward(config.total_votes, user_votes, balance)?,
                is_claimable: false,
            })
        }
    }
}

fn calculate_reward(
    total_votes: Uint128,
    user_votes: Uint128,
    balance: Uint128,
) -> Result<Uint128, CheckedMultiplyFractionError> {
    if balance.is_zero() || user_votes.is_zero() || total_votes.is_zero() {
        return Ok(Uint128::zero());
    }

    balance.checked_mul_floor((user_votes, total_votes))
}
