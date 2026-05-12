//! cw-utils 2.0 dropped `parse_reply_instantiate_data` and `parse_reply_execute_data`
//! in favor of the `parse_instantiate_response_data` / `parse_execute_response_data`
//! functions that take `&[u8]` directly. These shims restore the v1 signatures so
//! existing contract reply handlers compile without rewriting each call site.
//!
//! In cosmwasm-std 2.x the `SubMsgResponse::data` field is marked deprecated and may
//! be empty on chains running CosmWasm 2.0+ — those chains populate `msg_responses`
//! instead. The shim still reads `data` because that is what wasmd 1.x writes; once
//! Stage 3 lands the contract substrate runs on wasmvm 3.x and the equivalent
//! migration to `msg_responses` parsing can happen.

use cosmwasm_std::Reply;
use cw_utils::{
    parse_execute_response_data, parse_instantiate_response_data, MsgExecuteContractResponse,
    MsgInstantiateContractResponse, ParseReplyError,
};

#[allow(deprecated)]
pub fn parse_reply_instantiate_data(
    msg: Reply,
) -> Result<MsgInstantiateContractResponse, ParseReplyError> {
    let data = msg
        .result
        .into_result()
        .map_err(ParseReplyError::SubMsgFailure)?
        .data
        .ok_or_else(|| ParseReplyError::ParseFailure("Missing reply data".to_string()))?;
    parse_instantiate_response_data(&data)
}

#[allow(deprecated)]
pub fn parse_reply_execute_data(msg: Reply) -> Result<MsgExecuteContractResponse, ParseReplyError> {
    let data = msg
        .result
        .into_result()
        .map_err(ParseReplyError::SubMsgFailure)?
        .data
        .ok_or_else(|| ParseReplyError::ParseFailure("Missing reply data".to_string()))?;
    parse_execute_response_data(&data)
}
