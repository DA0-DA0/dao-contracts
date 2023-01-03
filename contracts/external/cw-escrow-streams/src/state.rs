use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, Env, Response, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

use crate::{
    msg::{CheckedStreamData, UncheckedStreamData},
    ContractError,
};
use cw_denom::CheckedDenom;

#[cw_serde]
pub struct Config {
    pub admin: Addr,
}
pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Stream {
    pub owner: Addr,
    pub recipient: Addr,
    /// Balance in Native and Cw20 tokens
    pub balance: Uint128,
    pub claimed_balance: Uint128,
    pub denom: CheckedDenom,
    pub start_time: u64,
    pub end_time: u64,
    pub paused_time: Option<u64>,
    pub paused_duration: Option<u64>,
    pub paused: bool,

    // METADATA
    /// Title of the payroll item, for example for a bug bounty "Fix issue in contract.rs"
    pub title: Option<String>,
    /// Description of the payroll item, a more in depth description of how to meet the payroll conditions
    pub description: Option<String>,
    /// Link to stream attached for sync
    pub link_id: Option<StreamId>,
    /// Making a stream detachable will only affect linked streams.
    /// A linked stream that detaches in the future will pause both streams.
    /// Each stream must then resume on their own, or be fully removed to re-link.
    pub is_detachable: bool,
}
impl From<CheckedStreamData> for Stream {
    fn from(data: CheckedStreamData) -> Stream {
        Stream {
            owner: data.owner,
            recipient: data.recipient,
            balance: data.balance,
            claimed_balance: Uint128::zero(),
            denom: data.denom,
            start_time: data.start_time,
            end_time: data.end_time,
            paused_time: None,
            paused_duration: None,
            paused: false,
            title: data.title,
            description: data.description,
            link_id: None,
            // TODO Should not be an option type
            is_detachable: data.is_detachable,
        }
    }
}

impl Stream {
    pub(crate) fn can_distribute_more(&self) -> bool {
        if self.balance == Uint128::zero() {
            return false;
        }
        self.claimed_balance < self.balance
    }

    pub(crate) fn calc_distribution_rate_core(
        start_time: u64,
        end_time: u64,
        block_time: Timestamp,
        paused_duration: Option<u64>,
        balance: Uint128,
        claimed_balance: Uint128,
    ) -> Result<(Uint128, Uint128), ContractError> {
        // TODO rename? This is not strictly speaking block time
        let block_time = std::cmp::min(block_time.seconds(), end_time);
        // TODO NO UNWRAP OR DEFAULT, throw real errors
        let duration: u64 = end_time.checked_sub(start_time).unwrap_or_default();
        if duration > 0 {
            let diff = block_time.checked_sub(start_time).unwrap_or_default();

            let passed: u128 = diff
                .checked_sub(paused_duration.unwrap_or_default())
                .unwrap_or_default()
                .into();

            let rate_per_second = balance.checked_div(duration.into()).unwrap_or_default();

            return Ok((
                (Uint128::from(passed) * rate_per_second)
                    .checked_sub(claimed_balance)
                    .unwrap_or_default(),
                rate_per_second,
            ));
        }
        Ok((Uint128::new(0), Uint128::new(0)))
    }

    pub(crate) fn calc_distribution_rate(
        &self,
        block_time: Timestamp,
    ) -> Result<(Uint128, Uint128), ContractError> {
        Stream::calc_distribution_rate_core(
            self.start_time,
            self.end_time,
            block_time,
            self.paused_duration,
            self.balance,
            self.claimed_balance,
        )
    }

    pub(crate) fn calc_pause_duration(&self, block_time: Timestamp) -> Option<u64> {
        let end = std::cmp::min(block_time.seconds(), self.end_time);
        self.paused_duration.unwrap_or_default().checked_add(
            end.checked_sub(self.paused_time.unwrap_or_default())
                .unwrap_or_default(),
        )
    }
}

pub type StreamId = u64;
pub type StreamIds = Vec<StreamId>;
pub type ContractResult = Result<Response, ContractError>;

pub(crate) trait StreamIdsExtensions {
    fn second(&self) -> Option<&StreamId>;
    fn validate(&self) -> Result<(), ContractError>;
}
impl StreamIdsExtensions for StreamIds {
    fn second(&self) -> Option<&StreamId> {
        self.get(1)
    }
    fn validate(&self) -> Result<(), ContractError> {
        if self.len() != 2 {
            return Err(ContractError::InvalidStreamIds {});
        }
        if self.first() == self.second() {
            return Err(ContractError::StreamsShouldNotBeEqual {});
        }
        Ok(())
    }
}

pub const STREAM_SEQ: Item<u64> = Item::new("stream_seq");
pub const STREAMS: Map<StreamId, Stream> = Map::new("stream");

impl UncheckedStreamData {
    pub fn into_checked(self, env: Env, deps: Deps) -> Result<CheckedStreamData, ContractError> {
        let owner = deps.api.addr_validate(&self.owner)?;
        let recipient = deps.api.addr_validate(&self.recipient)?;

        if self.start_time >= self.end_time {
            return Err(ContractError::InvalidStartTime {});
        }

        let block_time = env.block.time.seconds();

        if self.end_time <= block_time {
            return Err(ContractError::InvalidEndTime {});
        }

        Ok(CheckedStreamData {
            owner,
            recipient,
            balance: self.balance,
            denom: self.denom.into_checked(deps)?,
            start_time: self.start_time,
            end_time: self.end_time,
            title: self.title,
            description: self.description,
            is_detachable: self.is_detachable,
        })
    }
}

