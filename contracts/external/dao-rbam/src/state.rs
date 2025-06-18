use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, CosmosMsg};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, KeyDeserialize, Map, MultiIndex};

/// Whether or not actions can be performed.
pub const ENABLED: Item<bool> = Item::new("enabled");

/// The next ID to use for a role, authorization, or action. Default = 1
pub const NEXT_ID: Item<u64> = Item::new("next_id");

/// Map role_id -> role
pub const ROLES: Map<u64, Role> = Map::new("roles");

/// Map authorization_id -> authorization. Secondary index on role_id to look
/// up/iterate by role. This supports the following queries:
/// - get a specific authorization by ID (map lookup)
/// - list all authorizations for a specific role (secondary index range query)
pub const AUTHORIZATIONS: IndexedMap<u64, Authorization, AuthorizationsIndexes<'_>> =
    IndexedMap::new(
        "authorizations",
        AuthorizationsIndexes {
            role_id: MultiIndex::new(
                |_pk: &[u8], a: &Authorization| a.role_id,
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
pub const ASSIGNMENTS: IndexedMap<AssignmentPairKey, Addr, AssignmentsIndexes<'_>> =
    IndexedMap::new(
        "assignments",
        AssignmentsIndexes {
            role_id: MultiIndex::new(
                |pk: &[u8], _d: &Addr| AssignmentPairKey::from_slice(pk).unwrap().1,
                "assignments",
                "assignments__role_id",
            ),
        },
    );

/// Map (address, action_id) -> action. Secondary indexes on role_id and
/// authorization_id to look up/iterate over all actions by role and by
/// authorization. This supports the following queries:
/// - list all actions performed by a specific address (prefixed range query)
/// - list all actions performed by a specific role (secondary index range
///   query)
/// - list all actions performed by a specific authorization (secondary index
///   range query)
pub const LOG: IndexedMap<(Addr, u64), Action, LogIndexes<'_>> = IndexedMap::new(
    "log",
    LogIndexes {
        role_id: MultiIndex::new(|_pk: &[u8], a: &Action| a.role_id, "log", "log__role_id"),
        authorization_id: MultiIndex::new(
            |_pk: &[u8], a: &Action| a.authorization_id,
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
pub type AssignmentPairKey = (Addr, u64);

/// Secondary indexes for assignments to look up/iterate over all addresses
/// assigned to a role as well as all addresses with a role assigned at all.
pub struct AssignmentsIndexes<'a> {
    pub role_id: MultiIndex<'a, u64, Addr, AssignmentPairKey>,
}
impl IndexList<Addr> for AssignmentsIndexes<'_> {
    fn get_indexes(&self) -> Box<dyn Iterator<Item = &dyn Index<Addr>> + '_> {
        let v: Vec<&dyn Index<Addr>> = vec![&self.role_id];
        Box::new(v.into_iter())
    }
}

/// Secondary indexes for log to look up/iterate over all actions by role and by
/// authorization.
pub struct LogIndexes<'a> {
    pub role_id: MultiIndex<'a, u64, Action, (Addr, u64)>,
    pub authorization_id: MultiIndex<'a, u64, Action, (Addr, u64)>,
}
impl IndexList<Action> for LogIndexes<'_> {
    fn get_indexes(&self) -> Box<dyn Iterator<Item = &dyn Index<Action>> + '_> {
        let v: Vec<&dyn Index<Action>> = vec![&self.role_id, &self.authorization_id];
        Box::new(v.into_iter())
    }
}

#[cw_serde]
pub struct Role {
    /// Autoincrementing ID.
    pub id: u64,
    /// Name for the role (ideally unique, but not enforced).
    pub name: String,
    /// Optional metadata for the role. This should either be a JSON object or
    /// IPFS hash.
    pub metadata: Option<String>,
    /// Whether or not the role and all its authorizations are enabled.
    pub enabled: bool,
}

#[cw_serde]
pub struct Authorization {
    /// Autoincrementing ID.
    pub id: u64,
    /// Role ID.
    pub role_id: u64,
    /// Name for the authorization (ideally unique, but not enforced).
    pub name: String,
    /// Optional metadata for the authorization. This should either be a JSON
    /// object or IPFS hash.
    pub metadata: Option<String>,
    /// Optional `jsonfilter` filter. If no filter is provided, this is simply a
    /// symbolic authorization, which may be useful for external systems.
    pub filter: Option<serde_json::value::Value>,
    /// Whether or not the authorization is enabled.
    pub enabled: bool,
}

#[cw_serde]
pub struct Action {
    /// Autoincrementing ID.
    pub id: u64,
    /// Address that executed the message.
    pub addr: Addr,
    /// Message that was executed.
    pub msg: CosmosMsg,
    /// Role ID used to perform this action.
    pub role_id: u64,
    /// Authorization ID used to perform this action.
    pub authorization_id: u64,
}
