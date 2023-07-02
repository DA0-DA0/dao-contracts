use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;
use cw_storage_plus::{Item, Map};

// TODO add timestamps
// A struct representing a bounty
#[cw_serde]
pub struct Bounty {
    /// The ID for the bounty
    pub id: u64,
    /// The amount the bounty is claimable for
    pub amount: Coin,
    /// The title of the bounty
    pub title: String,
    /// Bounty description and details
    pub description: Option<String>,
    /// The bounty status
    pub status: BountyStatus,
    /// The timestamp when the bounty was created
    pub created_at: u64,
    /// The timestamp when the bounty was last updated
    pub updated_at: Option<u64>,
}

/// The status of the bounty
#[cw_serde]
pub enum BountyStatus {
    /// The bounty has been closed by the owner without being claimed
    Closed { closed_at: u64 },
    /// The bounty has been claimed
    Claimed { claimed_by: String, claimed_at: u64 },
    /// The bounty is open and available to be claimed
    Open,
}

pub const BOUNTIES: Map<u64, Bounty> = Map::new("bounties");
pub const ID: Item<u64> = Item::new("id");
