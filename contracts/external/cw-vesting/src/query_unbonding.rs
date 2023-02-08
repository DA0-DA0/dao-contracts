use cosmos_sdk_proto::{cosmos::staking::v1beta1 as staking_proto, prost::Message};
use cosmwasm_std::{to_vec, Binary, Empty, QuerierWrapper, QueryRequest, StdError};

use crate::ContractError;

pub(crate) fn query_unbonding_duration_seconds(
    querier: QuerierWrapper,
) -> Result<u64, ContractError> {
    let resp = querier
        .raw_query(&to_vec(&QueryRequest::<Empty>::Stargate {
            path: "custom/cosmos_sdk.x.staking.v1.Query/Params".to_string(),
            data: Binary::from(staking_proto::QueryParamsRequest {}.encode_to_vec()),
        })?)
        .into_result()
        .map_err(|e| StdError::generic_err(format!("querier system error: {e}")))?
        .into_result()
        .map_err(|e| StdError::generic_err(format!("querier contract error: {e}")))?;
    let unbonding_duration = staking_proto::QueryParamsResponse::decode(resp.as_slice())
        .expect("decodable response")
        .params
        .expect("staking module to have params")
        .unbonding_time
        .expect("staking module to have unbonding duration");
    Ok(unbonding_duration.seconds as u64 + if unbonding_duration.nanos > 0 { 1 } else { 0 })
}
