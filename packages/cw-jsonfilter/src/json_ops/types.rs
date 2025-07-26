use crate::{CwJsonFilter, FilterResult, ProtobufDecoder};

impl<D: ProtobufDecoder> CwJsonFilter<D> {
    pub fn handle_type_op(
        &self,
        operator: &str,
        operator_arg: &serde_json::Value,
        value: &serde_json::Value,
        filter_path: &str,
        obj_path: &str,
    ) -> FilterResult {
        match operator_arg {
            serde_json::Value::String(type_str) => {
                let (check, reason) = match type_str.to_lowercase().as_str() {
                    "null" => (value.is_null(), "value is not null"),
                    "boolean" => (value.is_boolean(), "value is not a boolean"),
                    "number" => (value.is_number(), "value is not a number"),
                    "string" => (value.is_string(), "value is not a string"),
                    "array" => (value.is_array(), "value is not an array"),
                    "object" => (value.is_object(), "value is not an object"),
                    _ => {
                        return FilterResult::fatal_invalid_filter(
                            format!("{} arg must be a valid type, got `{}`", operator, type_str),
                            filter_path,
                            obj_path,
                        );
                    }
                };

                FilterResult::from_bool(check, operator, reason, filter_path, obj_path)
            }
            _ => FilterResult::fatal_invalid_filter(
                format!("{} arg must be a string", operator),
                filter_path,
                obj_path,
            ),
        }
    }
}
