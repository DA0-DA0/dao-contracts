use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_controllers::Hooks;
use cw_storage_plus::Item;

use crate::msg::NftMintMsg;

#[cw_serde]
pub struct Config {
    pub nft_address: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const DAO: Item<Addr> = Item::new("dao");

// Hooks to contracts that will receive staking and unstaking
// messages.
pub const HOOKS: Hooks = Hooks::new("hooks");

// Holds initial NFTs messages during instantiation.
pub const INITITIAL_NFTS: Item<Vec<NftMintMsg>> = Item::new("initial_nfts");
