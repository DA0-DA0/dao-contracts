use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Expiration;
use cw_storage_plus::{Item, Map, SnapshotMap, Strategy};
use cw_utils::Duration;

/// the configuration of the delegation system.
pub const CONFIG: Item<Config> = Item::new("config");

/// the DAO this delegation system is connected to.
pub const DAO: Item<Addr> = Item::new("dao");

/// the VP delegated to a delegate that has not yet been used in votes cast by
/// delegators in a specific proposal.
pub const UNVOTED_DELEGATED_VP: Map<(&Addr, u64), Uint128> = Map::new("udvp");

/// the VP delegated to a delegate by height.
pub const DELEGATED_VP: SnapshotMap<&Addr, Uint128> = SnapshotMap::new(
    "dvp",
    "dvp__checkpoints",
    "dvp__changelog",
    Strategy::EveryBlock,
);

#[cw_serde]
pub struct Config {
    /// the maximum percent of voting power that a single delegate can wield.
    /// they can be delegated any amount of voting power—this cap is only
    /// applied when casting votes.
    pub vp_cap_percent: Option<Decimal>,
    /// the duration a delegation is valid for, after which it must be renewed
    /// by the delegator.
    pub delegation_validity: Option<Duration>,
}

#[cw_serde]
pub struct Delegation {
    /// the delegator.
    pub delegator: Addr,
    /// the delegate that can vote on behalf of the delegator.
    pub delegate: Addr,
    /// the percent of the delegator's voting power that is delegated to the
    /// delegate.
    pub percent: Decimal,
    /// when the delegation expires.
    pub expiration: Expiration,
}
