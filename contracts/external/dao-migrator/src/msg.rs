use cosmwasm_schema::{cw_serde, QueryResponses};
use dao_core::query::SubDao;

use crate::types::{MigrationParams, V1CodeIds, V2CodeIds};

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
        sub_daos: Option<Vec<SubDao>>,
        params: MigrationParams,
        v1_code_ids: V1CodeIds,
        v2_code_ids: V2CodeIds,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
