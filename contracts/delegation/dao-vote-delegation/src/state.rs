use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_snapshot_vector_map::SnapshotVectorMap;
use cw_storage_plus::{Item, Map, SnapshotMap, Strategy};

/// the configuration of the delegation system.
pub const CONFIG: Item<Config> = Item::new("config");

/// the DAO this delegation system is connected to.
pub const DAO: Item<Addr> = Item::new("dao");

/// the delegates.
pub const DELEGATES: SnapshotMap<Addr, Delegate> = SnapshotMap::new(
    "delegates",
    "delegates__checkpoints",
    "delegates__changelog",
    Strategy::EveryBlock,
);

/// map (delegate, proposal_module, proposal_id) -> the VP delegated to the
/// delegate that has not yet been used in votes cast by delegators in a
/// specific proposal.
pub const UNVOTED_DELEGATED_VP: Map<(&Addr, &Addr, u64), Uint128> = Map::new("udvp");

/// the VP delegated to a delegate by height.
pub const DELEGATED_VP: SnapshotMap<&Addr, Uint128> = SnapshotMap::new(
    "dvp",
    "dvp__checkpoints",
    "dvp__changelog",
    Strategy::EveryBlock,
);

/// the delegations of a delegator.
pub const DELEGATIONS: SnapshotVectorMap<Addr, Delegation> = SnapshotVectorMap::new(
    "d__items",
    "d__next_ids",
    "d__active",
    "d__active__checkpoints",
    "d__active__changelog",
);

/// map (delegator, delegate) -> ID of the delegation in the vector map. this is
/// useful for quickly checking if a delegation already exists, and for
/// undelegating.
pub const DELEGATION_IDS: Map<(&Addr, &Addr), u64> = Map::new("dids");

/// map (delegator, delegate) -> calculated absolute delegated VP.
pub const DELEGATED_VP_AMOUNTS: Map<(&Addr, &Addr), Uint128> = Map::new("dvp_amounts");

/// map delegator -> percent delegated to all delegates.
pub const PERCENT_DELEGATED: Map<&Addr, Decimal> = Map::new("pd");

#[cw_serde]
pub struct Config {
    /// the maximum percent of voting power that a single delegate can wield.
    /// they can be delegated any amount of voting power—this cap is only
    /// applied when casting votes.
    pub vp_cap_percent: Option<Decimal>,
    // /// the duration a delegation is valid for, after which it must be renewed
    // /// by the delegator.
    // pub delegation_validity: Option<Duration>,
}

#[cw_serde]
pub struct Delegate {}

#[cw_serde]
pub struct Delegation {
    /// the delegate that can vote on behalf of the delegator.
    pub delegate: Addr,
    /// the percent of the delegator's voting power that is delegated to the
    /// delegate.
    pub percent: Decimal,
}
