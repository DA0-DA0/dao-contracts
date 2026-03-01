use cw721::EmptyOptionalCollectionExtension;
use dao_cw721_extensions::roles::{ExecuteExt, MetadataExt, QueryExt};

pub type InstantiateMsg = cw721::msg::Cw721InstantiateMsg<EmptyOptionalCollectionExtension>;
pub type ExecuteMsg =
    cw721::msg::Cw721ExecuteMsg<MetadataExt, EmptyOptionalCollectionExtension, ExecuteExt>;
pub type QueryMsg =
    cw721::msg::Cw721QueryMsg<MetadataExt, EmptyOptionalCollectionExtension, QueryExt>;
