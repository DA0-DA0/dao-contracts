use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// NB: these are placeholders, set as structs
// so they don't result in invalid JSON. They should ofc be enums
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct ExecuteMsg {}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct QueryMsg {}
