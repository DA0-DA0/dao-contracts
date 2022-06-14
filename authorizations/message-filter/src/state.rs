use cosmwasm_std::{Addr, Response};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ContractError;
use cw_auth_middleware::ContractError as AuthorizationError;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum Kind {
    Allow {},
    Reject {},
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema)]
pub struct Config {
    /// The address of the DAO that this authorization module is
    /// associated with.
    pub dao: Addr,
    /// The type of authorization this is. Kind::Allow means messages will only
    /// be authorized (allowed) if there is a matching Authorization in the
    /// contract. Kind::Reject means all messages will be authorized (allowed)
    /// by this contract unless explicitly rejected by one of the stored
    /// authorizations
    pub kind: Kind,
}

impl Config {
    pub fn default_response(&self) -> Result<Response, ContractError> {
        match self.kind {
            Kind::Allow {} => Err(AuthorizationError::Unauthorized {
                reason: Some("No authorizations allowed the request. Rejecting.".to_string()),
            }
            .into()),
            Kind::Reject {} => Ok(Response::default()
                .add_attribute("allowed", "true")
                .add_attribute(
                    "reason",
                    "No authorizations rejected the request. Allowing.",
                )),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Authorization {
    pub addr: Addr,
    /// A json representation of a CosmosMsg. Incomming messages will be
    /// recursively compared to the matcher to determine if they are authorized.
    ///
    /// To short-circuit the recursive comparison (i.e.: allow everything under
    /// an object key), you can use the empty object.
    ///
    /// For example:
    ///
    /// {"bank": {"to_address": "an_address", "amount":[{"denom": "juno", "amount": 1}]}}
    ///
    /// will match exactly that message but not a message where any of the fields are different.
    ///
    /// However, {"bank": {}} will match all bank messages, and
    /// {"bank": {"send": {"to_address": "an_address", "amount": {}}}} will match all bank messages to "an_address".
    ///
    pub matcher: String,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const ALLOWED: Map<Addr, Vec<Authorization>> = Map::new("allowed");
