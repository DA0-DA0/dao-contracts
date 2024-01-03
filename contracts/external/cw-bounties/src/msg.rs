use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Coin;
use cw_ownable::cw_ownable_execute;

#[cw_serde]
pub struct InstantiateMsg {
    /// Contract owner with the ability to create, pay out, close
    /// and update bounties. Must be a valid account address.
    pub owner: String,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// Creates a bounty (only owner)
    Create {
        /// The amount the bounty is claimable for
        amount: Coin,
        /// The title of the bounty
        title: String,
        /// Bounty description and details
        description: Option<String>,
    },
    /// Closes a bounty (only owner)
    Close {
        /// The ID of the bounty to close
        id: u64,
    },
    /// Claims a bounty (only owner)
    PayOut {
        /// Bounty id to claim
        id: u64,
        /// Recipient address where funds from bounty are claimed
        recipient: String,
    },
    /// Updates a bounty (only owner)
    Update {
        /// The ID of the bounty
        id: u64,
        /// The amount the bounty is claimable for
        amount: Coin,
        /// The title of the bounty
        title: String,
        /// Bounty description and details
        description: Option<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns a single bounty by ID
    #[returns(crate::state::Bounty)]
    Bounty { id: u64 },
    /// List bounties
    #[returns(Vec<crate::state::Bounty>)]
    Bounties {
        /// Used for pagination
        start_after: Option<u64>,
        /// The number of bounties to return
        limit: Option<u32>,
    },
    /// Returns the number of bounties
    #[returns(u64)]
    Count {},
    /// Returns information about the current contract owner
    #[returns(::cw_ownable::Ownership<::cosmwasm_std::Addr>)]
    Ownership {},
}

#[cw_serde]
pub struct MigrateMsg {}
