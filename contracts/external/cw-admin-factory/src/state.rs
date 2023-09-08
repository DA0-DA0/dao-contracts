use cosmwasm_std::Coin;
use cw_storage_plus::Item;

pub const FEE: Item<Vec<Coin>> = Item::new("fee");
