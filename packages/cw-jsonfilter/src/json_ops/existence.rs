use crate::{CwJsonFilter, FilterResult, ProtobufDecoder};

impl<D: ProtobufDecoder> CwJsonFilter<D> {
    pub fn handle_exists_op(
        &self,
        operator: &str,
        operator_arg: &serde_json::Value,
        value: Option<&serde_json::Value>,
        filter_path: &str,
        obj_path: &str,
    ) -> FilterResult {
        match operator_arg {
            serde_json::Value::Bool(exists) => FilterResult::from_bool(
                *exists == value.is_some(),
                operator,
                match value.is_some() {
                    true => "value exists",
                    false => "value does not exist",
                },
                filter_path,
                obj_path,
            ),
            _ => FilterResult::fatal_invalid_filter(
                format!("{} arg must be a boolean", operator),
                filter_path,
                obj_path,
            ),
        }
    }
}
