use dao_cw721_extensions::roles::{ExecuteExt, MetadataExt, QueryExt};

pub type InstantiateMsg = cw721_base::InstantiateMsg;
pub type ExecuteMsg = cw721_base::ExecuteMsg<MetadataExt, ExecuteExt>;
pub type QueryMsg = cw721_base::QueryMsg<QueryExt>;
