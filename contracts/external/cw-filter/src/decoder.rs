use cosmwasm_std::{Addr, QuerierWrapper};
use cw_jsonfilter::ProtobufDecoder;
use serde_json::Value;

/// ProtobufDecoder implementing helper type that wraps around a
/// cw querier and the registry address to enable cw-protobuf-registry
/// decoding queries.
pub struct WasmQuerierProtobufDecoder<'a> {
    querier: QuerierWrapper<'a>,
    registry_address: Addr,
}

impl<'a> WasmQuerierProtobufDecoder<'a> {
    pub fn new(querier: QuerierWrapper<'a>, registry_address: Addr) -> Self {
        Self {
            querier,
            registry_address,
        }
    }
}

impl ProtobufDecoder for WasmQuerierProtobufDecoder<'_> {
    fn decode(&self, message_name: String, value: Vec<u8>) -> Result<Value, String> {
        self.querier
            .query_wasm_smart::<cw_protobuf_registry::msg::DecodeResponse>(
                &self.registry_address,
                &cw_protobuf_registry::msg::QueryMsg::Decode {
                    message_name,
                    value,
                },
            )
            .map(|r| r.value)
            .map_err(|e| e.to_string())
    }
}
