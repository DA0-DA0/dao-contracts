use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Timestamp;
use dao_hooks::vote::VoteHookMsg;

#[cw_serde]
pub struct InstantiateMsg {
    /// DAO address
    pub dao: String,
    /// Epoch duration in seconds. Used for reward calculation.
    pub epoch_duration: Timestamp,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Fires when a new vote is cast.
    VoteHook(VoteHookMsg),
    /// Claim rewards.
    Claim {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the rewards for the given address.
    #[returns(cosmwasm_std::Uint128)]
    Rewards { address: String },
}

#[cw_serde]
pub struct MigrateMsg {}
