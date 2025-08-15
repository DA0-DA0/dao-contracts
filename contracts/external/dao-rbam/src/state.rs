use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary};
use cw_storage_plus::{
    Index, IndexList, IndexedMap, Item, KeyDeserialize, Map, MultiIndex, UniqueIndex,
};

use crate::{
    action::Action,
    role::{Authorization, Role},
};

/// The address of the DAO.
pub const DAO: Item<Addr> = Item::new("dao");

/// Filter setup helper type used during the instantiation.
#[cw_serde]
pub struct PendingFilterInstall {
    /// The code ID of the filter contract, stored temporarily if creating the
    /// protobuf registry first.
    pub filter_code_id: u64,
    /// The salt of the filter contract, stored temporarily if creating the
    /// protobuf registry first.
    pub filter_salt: Option<Binary>,
}

pub const PENDING_FILTER_INSTALL: Item<PendingFilterInstall> = Item::new("pending_filter_install");

/// The address of the filter contract.
pub const FILTER: Item<Addr> = Item::new("filter");

/// The address of the protobuf registry, if any.
pub const PROTOBUF_REGISTRY: Item<Addr> = Item::new("protobuf_registry");

/// Whether or not actions can be performed.
pub const ENABLED: Item<bool> = Item::new("enabled");

/// The next ID to use for a role, authorization, or action. Default = 1
pub const NEXT_ID: Item<u64> = Item::new("next_id");

/// Map role_id -> role
pub const ROLES: Map<u64, Role> = Map::new("roles");

/// Map authorization_id -> authorization. Secondary index on role_id to look
/// up/iterate by role. This supports the following queries:
/// - get a specific authorization by ID (map lookup)
/// - list all authorizations (range query)
/// - list all authorizations for a specific role (secondary index range query)
pub const AUTHORIZATIONS: IndexedMap<u64, Authorization, AuthorizationsIndexes<'_>> =
    IndexedMap::new(
        "authorizations",
        AuthorizationsIndexes {
            role_id: MultiIndex::new(
                |_pk, a| a.role_id,
                "authorizations",
                "authorizations__role_id",
            ),
        },
    );

/// Map (address, role_id) -> address. Secondary index on role_id to look
/// up/iterate over addresses by role. Indexes point to values stored in the
/// map, so we must redundantly store the address in both the key and the map.
/// No need to create an assignment ID since assignments are ephemeral and can
/// be deleted (the action audit log is sufficient history). This supports the
/// following queries:
/// - check if an address has a specific role (map lookup)
/// - list all roles assigned to a specific address (prefixed range query)
/// - list all addresses assigned a specific role (secondary index range query)
pub const ASSIGNMENTS: IndexedMap<AssignmentPair, Addr, AssignmentsIndexes<'_>> = IndexedMap::new(
    "assignments",
    AssignmentsIndexes {
        role_id: MultiIndex::new(
            |pk: &[u8], _d| AssignmentPair::from_slice(pk).unwrap().1,
            "assignments",
            "assignments__role_id",
        ),
    },
);

/// Map (address, action_id) -> action. Secondary indexes on action_id, role_id,
/// and authorization_id to look up/iterate over all actions by role and by
/// authorization. This supports the following queries:
/// - get a specific action by ID (map lookup)
/// - list all actions (range query)
/// - list all actions performed by a specific address (prefixed range query)
/// - list all actions performed by a specific role (secondary index range
///   query)
/// - list all actions performed by a specific authorization (secondary index
///   range query)
pub const LOG: IndexedMap<LogPairKey, Action, LogIndexes<'_>> = IndexedMap::new(
    "log",
    LogIndexes {
        action_id: UniqueIndex::new(|a| a.id, "log__action_id"),
        role_id: MultiIndex::new(|_pk, a| a.role_id, "log", "log__role_id"),
        authorization_id: MultiIndex::new(
            |_pk, a| a.authorization_id,
            "log",
            "log__authorization_id",
        ),
    },
);

/// Secondary index for authorizations to look up/iterate by role.
pub struct AuthorizationsIndexes<'a> {
    pub role_id: MultiIndex<'a, u64, Authorization, u64>,
}
impl IndexList<Authorization> for AuthorizationsIndexes<'_> {
    fn get_indexes(&self) -> Box<dyn Iterator<Item = &dyn Index<Authorization>> + '_> {
        let v: Vec<&dyn Index<Authorization>> = vec![&self.role_id];
        Box::new(v.into_iter())
    }
}

/// A pair of (address, role_id), the key for the ASSIGNMENTS map.
type AssignmentPair = (Addr, u64);

/// Secondary indexes for assignments to look up/iterate over all addresses
/// assigned to a role as well as all addresses with a role assigned at all.
pub struct AssignmentsIndexes<'a> {
    pub role_id: MultiIndex<'a, u64, Addr, AssignmentPair>,
}
impl IndexList<Addr> for AssignmentsIndexes<'_> {
    fn get_indexes(&self) -> Box<dyn Iterator<Item = &dyn Index<Addr>> + '_> {
        let v: Vec<&dyn Index<Addr>> = vec![&self.role_id];
        Box::new(v.into_iter())
    }
}

/// A pair of (address, action_id), the key for the LOG map.
pub type LogPairKey = (Addr, u64);

/// Secondary indexes for log to look up/iterate over all actions by role and by
/// authorization.
pub struct LogIndexes<'a> {
    pub action_id: UniqueIndex<'a, u64, Action, LogPairKey>,
    pub role_id: MultiIndex<'a, u64, Action, LogPairKey>,
    pub authorization_id: MultiIndex<'a, u64, Action, LogPairKey>,
}
impl IndexList<Action> for LogIndexes<'_> {
    fn get_indexes(&self) -> Box<dyn Iterator<Item = &dyn Index<Action>> + '_> {
        let v: Vec<&dyn Index<Action>> =
            vec![&self.action_id, &self.role_id, &self.authorization_id];
        Box::new(v.into_iter())
    }
}
