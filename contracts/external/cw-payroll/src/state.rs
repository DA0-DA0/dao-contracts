
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, DepsMut, StdResult, Uint128, Timestamp};
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

use crate::{balance::WrappedBalance, msg::StreamId};

#[cw_serde]
pub struct Config {
    pub admin: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Stream {
    pub admin: Addr,
    pub recipient: Addr,
    /// Balance in Native and Cw20 tokens
    pub balance: WrappedBalance,
    pub claimed_balance: WrappedBalance,
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

impl Stream {
    pub(crate) fn can_ditribute_more(&self) -> bool {
        if self.balance.amount() == 0 {
            return false;
        }
        self.claimed_balance.amount() < self.balance.amount()
    }
    pub (crate) fn calc_distribution_rate_core(
        start_time:u64,
        end_time:u64,
        paused_time:Option<u64>,
        balance:&WrappedBalance,
        block_time:Timestamp
        )->Uint128{
        let duration: u128 = (end_time.checked_sub(start_time).unwrap_or_default()).into();
        if duration > 0 {
            let paused_duration: u128 = (block_time.seconds().checked_sub(paused_time.unwrap_or_default()).unwrap_or_default()).into();
            let duration_diff=duration.checked_sub(paused_duration).unwrap_or_default();
            let rate_per_second = Uint128::from(balance.amount().checked_div(duration_diff).unwrap_or_default());
            return rate_per_second
        }
        Uint128::new(0)
    }
    pub (crate) fn calc_distribution_rate(
        &self,
        block_time:Timestamp
        )->Uint128{
        Stream::calc_distribution_rate_core(self.start_time,self.end_time,self.paused_time,&self.balance,block_time)
    }
}
pub const STREAM_SEQ: Item<u64> = Item::new("stream_seq");
pub const STREAMS: Map<StreamId, Stream> = Map::new("stream");

pub fn add_stream(deps: DepsMut, stream: &Stream) -> StdResult<StreamId> {
    let id = STREAM_SEQ.load(deps.storage)?;
    let id = id.checked_add(1).unwrap();
    STREAM_SEQ.save(deps.storage, &id)?;
    STREAMS.save(deps.storage, id, stream)?;
    Ok(id)
}
pub fn save_stream(deps: DepsMut, id: StreamId, stream: &Stream) -> StdResult<StreamId> {
    STREAMS.save(deps.storage, id, stream)?;
    Ok(id)
}

pub fn remove_stream(deps: DepsMut, stream_id: StreamId) -> StdResult<StreamId> {
    STREAMS.remove(deps.storage, stream_id);
    Ok(stream_id)
}
