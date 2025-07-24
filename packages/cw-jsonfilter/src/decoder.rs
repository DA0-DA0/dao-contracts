use serde_json::Value;

/// interface for decoding protobuf messages. This allows CwJsonFilter to
/// remain generic while enabling consumers to provide their own decoding logic.
pub trait ProtobufDecoder {
    fn decode(&self, message_name: String, value: Vec<u8>) -> Result<Value, String>;
}

/// default decoder used when you don't care about protobuf
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopDecoder;

impl ProtobufDecoder for NoopDecoder {
    fn decode(&self, _message_name: String, _value: Vec<u8>) -> Result<Value, String> {
        Err("protobuf decoder not provided".to_string())
    }
}
