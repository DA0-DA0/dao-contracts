use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    /// Address that is allowed to return deposits.
    pub admin: Addr,
    /// Deposit required for valid submission.
    pub required_deposit: Option<Asset>,
    /// Address of contract where each deposit is transferred.
    pub community_pool: Addr,
    /// Total reward amount.
    pub reward: Asset,
}

pub const CONFIG: Item<Config> = Item::new("config");

#[cw_serde]
pub enum AssetType {
    Native(String),
    Cw20(String),
}

#[cw_serde]
pub struct Asset {
    pub denom: AssetType,
    pub amount: Uint128,
}

impl Asset {
    pub fn new_native(denom: &str, amount: u128) -> Self {
        Self {
            denom: AssetType::Native(denom.to_owned()),
            amount: amount.into(),
        }
    }

    pub fn new_cw20(denom: &str, amount: u128) -> Self {
        Self {
            denom: AssetType::Cw20(denom.to_owned()),
            amount: amount.into(),
        }
    }
}

#[cw_serde]
pub struct Submission {
    pub sender: Addr,
    pub name: String,
    pub url: String,
}

// All submissions indexed by submition's fund destination address.
pub const SUBMISSIONS: Map<Addr, Submission> = Map::new("submissions");
