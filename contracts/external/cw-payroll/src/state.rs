use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Response, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

use crate::ContractError;
use cw_denom::CheckedDenom;

#[cw_serde]
pub struct Config {
    pub admin: Addr,
}
pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Stream {
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

pub const STREAM_SEQ: Item<u64> = Item::new("stream_seq");
pub const STREAMS: Map<u64, Stream> = Map::new("stream");
