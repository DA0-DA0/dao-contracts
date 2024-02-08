use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;
use cw_denom::{CheckedDenom, UncheckedDenom};
use cw_ownable::cw_ownable_query;
use cw_utils::Expiration;
use dao_hooks::vote::VoteHookMsg;
use dao_interface::state::Config;

#[cw_serde]
pub struct InstantiateMsg {
    /// The contract's owner using cw-ownable
    pub owner: String,
    /// The denom to distribute
    pub denom: UncheckedDenom,
    /// The expiration of the voting incentives
    pub expiration: Expiration,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Fires when a new vote is cast.
    VoteHook(VoteHookMsg),
    /// Claim rewards
    Claim {},
    /// Expire the voting incentives period
    Expire {},
    UpdateOwnership(cw_ownable::Action),
    Receive(Cw20ReceiveMsg),
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the config
    #[returns(Config)]
    Config {},
    /// Returns the claimable rewards for the given address.
    #[returns(RewardResponse)]
    Rewards { address: String },
    /// Returns the expected rewards for the given address
    #[returns(RewardResponse)]
    ExpectedRewards { address: String },
    /// Returns the votes count for the given address
    #[returns(Uint128)]
    Votes { address: String },
}

#[cw_serde]
pub enum MigrateMsg {
    FromCompatible {},
}

#[cw_serde]
pub struct RewardResponse {
    pub denom: CheckedDenom,
    pub amount: Uint128,
}
