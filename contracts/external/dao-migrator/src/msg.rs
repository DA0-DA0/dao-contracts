use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::WasmMsg;
use dao_core::query::SubDao;

use crate::types::{MigrationParams, V1CodeIds, V2CodeIds};

// TODO: Maybe we can unwrap `MigrationParams`, and include it in initMsg directly,
// and just save the whole init msg into storage? seems unneeded to add extra layer
#[cw_serde]
pub struct InstantiateMsg {
    pub migration_params: MigrationParams,
    pub sub_daos: Option<Vec<SubDao>>,
    pub v1_code_ids: V1CodeIds,
    pub v2_code_ids: V2CodeIds,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Detects the current DAO configuration and performs a migration
    /// checking state before and after to smoke test the migration's
    /// success. This module will remove itself on this message's
    /// completion regardless of the migration's success.
    MigrateV1ToV2 {
        params: MigrationParams,
        v1_code_ids: V1CodeIds,
        v2_code_ids: V2CodeIds,
    },
    /// Callable only by this contract.
    ///
    /// In submessage terms, say a message that results in an error
    /// "returns false" and one that succedes "returns true". Returns
    /// the logical conjunction (&&) of all the messages in operands.
    ///
    /// Under the hood this just executes them in order. We use this
    /// to respond with a single ACK when a message calls for the
    /// execution of both `CreateVouchers` and `RedeemVouchers`.
    Conjunction { operands: Vec<WasmMsg> },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
