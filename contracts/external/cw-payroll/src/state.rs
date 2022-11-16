use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, DepsMut, StdResult, Uint128};
use cw20::{Balance, Cw20CoinVerified};
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

#[cw_serde]
pub struct Config {
    pub admin: Addr,
}

#[cw_serde]
#[derive(Default)]
pub struct GenericBalance {
    pub native: Vec<Coin>,
    pub cw20: Vec<Cw20CoinVerified>,
}

impl GenericBalance {
    pub fn add_tokens(&mut self, add: Balance) {
        match add {
            Balance::Native(balance) => {
                for token in balance.0 {
                    let index = self.native.iter().enumerate().find_map(|(i, exist)| {
                        if exist.denom == token.denom {
                            Some(i)
                        } else {
                            None
                        }
                    });
                    match index {
                        Some(idx) => self.native[idx].amount += token.amount,
                        None => self.native.push(token),
                    }
                }
            }
            Balance::Cw20(token) => {
                let index = self.cw20.iter().enumerate().find_map(|(i, exist)| {
                    if exist.address == token.address {
                        Some(i)
                    } else {
                        None
                    }
                });
                match index {
                    Some(idx) => self.cw20[idx].amount += token.amount,
                    None => self.cw20.push(token),
                }
            }
        };
    }
}

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Stream {
    pub admin: Addr,
    pub recipient: Addr,
    /// Balance in Native and Cw20 tokens
    pub balance: GenericBalance,
    pub claimed_balance: GenericBalance,
    pub start_time: u64,
    pub end_time: u64,
    pub paused_time: Option<u64>,
    pub paused: bool,

    // TODO: Does this need to be stored? seems read only fine
    pub rate_per_second: Uint128,

    // METADATA
    /// Title of the payroll item, for example for a bug bounty "Fix issue in contract.rs"
    pub title: Option<String>,
    /// Description of the payroll item, a more in depth description of how to meet the payroll conditions
    pub description: Option<String>,
}

pub const STREAM_SEQ: Item<u64> = Item::new("stream_seq");
pub const STREAMS: Map<u64, Stream> = Map::new("stream");

pub fn save_stream(deps: DepsMut, stream: &Stream) -> StdResult<u64> {
    let id = STREAM_SEQ.load(deps.storage)?;
    let id = id.checked_add(1).unwrap();
    STREAM_SEQ.save(deps.storage, &id)?;
    STREAMS.save(deps.storage, id, stream)?;
    Ok(id)
}
