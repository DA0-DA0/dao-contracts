use cosmwasm_schema::{cw_serde, QueryResponses};
use dao_interface::query::SubDao;

use crate::types::{MigrationParams, V1CodeIds, V2CodeIds};

#[cw_serde]
pub struct MigrateV1ToV2 {
    pub sub_daos: Vec<SubDao>,
    pub migration_params: MigrationParams,
    pub v1_code_ids: V1CodeIds,
    pub v2_code_ids: V2CodeIds,
}

pub type InstantiateMsg = MigrateV1ToV2;

pub type ExecuteMsg = MigrateV1ToV2;

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
