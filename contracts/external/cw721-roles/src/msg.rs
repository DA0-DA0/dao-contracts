use cosmwasm_std::Empty;
use cw721::msg::{Cw721ExecuteMsg, Cw721InstantiateMsg, Cw721QueryMsg};
use dao_cw721_extensions::roles::{ExecuteExt, MetadataExt, QueryExt};

/// cw721-roles uses no collection-info extension, so the collection extension msg is `Empty`.
pub type InstantiateMsg = Cw721InstantiateMsg<Empty>;
/// `MetadataExt` doubles as the NFT extension state and the mint-time extension msg.
pub type ExecuteMsg = Cw721ExecuteMsg<MetadataExt, Empty, ExecuteExt>;
pub type QueryMsg = Cw721QueryMsg<MetadataExt, Empty, QueryExt>;
