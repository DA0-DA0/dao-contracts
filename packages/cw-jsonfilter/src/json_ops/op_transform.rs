use base64::Engine;
use serde_json::json;

use crate::{
    json_ops::operator::OperatorContext, CwJsonFilter, FilterResult, ProtobufDecoder, BASE64_ENGINE,
};

impl<D: ProtobufDecoder> CwJsonFilter<D> {
    pub fn handle_size_op(&self, op_ctx: OperatorContext) -> FilterResult {
        let value = match op_ctx.value {
            Some(v) => v,
            None => return FilterResult::key_not_found(op_ctx.filter_path, op_ctx.obj_path),
        };

        match value.as_array().map_or_else(
            || {
                value.as_str().map_or_else(
                    || value.as_object().map(|value_obj| value_obj.len()),
                    |value_str| Some(value_str.len()),
                )
            },
            |value_array| Some(value_array.len()),
        ) {
            Some(len) => self.inner_matches(
                op_ctx.operator_arg,
                Some(&serde_json::Value::Number(len.into())),
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
            _ => FilterResult::operator_failed(
                op_ctx.operator,
                "value is not a string, array, or object",
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
        }
    }

    pub fn handle_to_string_op(&self, op_ctx: OperatorContext) -> FilterResult {
        let value = match op_ctx.value {
            Some(v) => v,
            None => return FilterResult::key_not_found(op_ctx.filter_path, op_ctx.obj_path),
        };

        match value {
            // pass through if value is already a string
            serde_json::Value::String(_) => self.inner_matches(
                op_ctx.operator_arg,
                Some(value),
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
            _ => self.inner_matches(
                op_ctx.operator_arg,
                Some(&serde_json::Value::String(value.to_string())),
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
        }
    }

    pub fn handle_to_number_op(&self, op_ctx: OperatorContext) -> FilterResult {
        let value = match op_ctx.value {
            Some(v) => v,
            None => return FilterResult::key_not_found(op_ctx.filter_path, op_ctx.obj_path),
        };

        match value {
            // pass through if value is already a number
            serde_json::Value::Number(_) => self.inner_matches(
                op_ctx.operator_arg,
                Some(value),
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
            serde_json::Value::String(value_str) => self.inner_matches(
                op_ctx.operator_arg,
                Some(&serde_json::Value::Number(match value_str.parse() {
                    Ok(num) => num,
                    Err(e) => {
                        return FilterResult::operator_failed(
                            op_ctx.operator,
                            format!("failed to convert string to number: {}", e),
                            op_ctx.filter_path,
                            op_ctx.obj_path,
                        )
                    }
                })),
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
            _ => FilterResult::operator_failed(
                op_ctx.operator,
                "value is not a string or number",
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
        }
    }

    pub fn handle_to_lower_op(&self, op_ctx: OperatorContext) -> FilterResult {
        let value = match op_ctx.value {
            Some(v) => v,
            None => return FilterResult::key_not_found(op_ctx.filter_path, op_ctx.obj_path),
        };

        match value {
            serde_json::Value::String(value_str) => self.inner_matches(
                op_ctx.operator_arg,
                Some(&serde_json::Value::String(value_str.to_lowercase())),
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
            _ => FilterResult::operator_failed(
                op_ctx.operator,
                "value is not a string",
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
        }
    }

    pub fn handle_to_upper_op(&self, op_ctx: OperatorContext) -> FilterResult {
        let value = match op_ctx.value {
            Some(v) => v,
            None => return FilterResult::key_not_found(op_ctx.filter_path, op_ctx.obj_path),
        };

        match value {
            serde_json::Value::String(value_str) => self.inner_matches(
                op_ctx.operator_arg,
                Some(&serde_json::Value::String(value_str.to_uppercase())),
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
            _ => FilterResult::operator_failed(
                op_ctx.operator,
                "value is not a string",
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
        }
    }

    pub fn handle_to_keys_op(&self, op_ctx: OperatorContext) -> FilterResult {
        let value = match op_ctx.value {
            Some(v) => v,
            None => return FilterResult::key_not_found(op_ctx.filter_path, op_ctx.obj_path),
        };

        match value {
            serde_json::Value::Object(value_obj) => self.inner_matches(
                op_ctx.operator_arg,
                Some(&serde_json::Value::Array(
                    value_obj
                        .keys()
                        .map(|k| serde_json::Value::String(k.clone()))
                        .collect(),
                )),
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
            _ => FilterResult::operator_failed(
                op_ctx.operator,
                "value is not an object",
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
        }
    }

    pub fn handle_to_values_op(&self, op_ctx: OperatorContext) -> FilterResult {
        let value = match op_ctx.value {
            Some(v) => v,
            None => return FilterResult::key_not_found(op_ctx.filter_path, op_ctx.obj_path),
        };

        match value {
            serde_json::Value::Object(value_obj) => self.inner_matches(
                op_ctx.operator_arg,
                Some(&serde_json::Value::Array(
                    value_obj.values().cloned().collect(),
                )),
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
            _ => FilterResult::operator_failed(
                op_ctx.operator,
                "value is not an object",
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
        }
    }
    pub fn handle_replace_op(&self, op_ctx: OperatorContext) -> FilterResult {
        let value = match op_ctx.value {
            Some(v) => v,
            None => return FilterResult::key_not_found(op_ctx.filter_path, op_ctx.obj_path),
        };

        match (op_ctx.operator_arg, value) {
            (serde_json::Value::Object(op_arg), serde_json::Value::String(value_str)) => {
                let substring = match op_arg.get("find") {
                    Some(serde_json::Value::String(str)) => str,
                    _ => {
                        return FilterResult::fatal_invalid_filter(
                            format!("{} arg `find` must be a string", op_ctx.operator),
                            op_ctx.filter_path,
                            op_ctx.obj_path,
                        )
                    }
                };

                let replace = match op_arg.get("replace") {
                    Some(serde_json::Value::String(str)) => str,
                    _ => {
                        return FilterResult::fatal_invalid_filter(
                            format!("{} arg `replace` must be a string", op_ctx.operator),
                            op_ctx.filter_path,
                            op_ctx.obj_path,
                        )
                    }
                };

                let filter = match op_arg.get("filter") {
                    Some(v) => v,
                    None => {
                        return FilterResult::fatal_invalid_filter(
                            format!("{} arg `filter` must be provided", op_ctx.operator),
                            op_ctx.filter_path,
                            op_ctx.obj_path,
                        )
                    }
                };

                let replaced = value_str.replace(substring, replace.as_str());

                self.inner_matches(
                    filter,
                    Some(&serde_json::Value::String(replaced)),
                    op_ctx.filter_path,
                    op_ctx.obj_path,
                )
            }
            (serde_json::Value::Object(_), _) => FilterResult::operator_failed(
                op_ctx.operator,
                "value is not a string",
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
            _ => FilterResult::fatal_invalid_filter(
                format!("{} arg must be an object", op_ctx.operator),
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
        }
    }
    pub fn handle_base64_op(&self, op_ctx: OperatorContext) -> FilterResult {
        let value = match op_ctx.value {
            Some(v) => v,
            None => return FilterResult::key_not_found(op_ctx.filter_path, op_ctx.obj_path),
        };

        match value {
            serde_json::Value::String(value_str) => {
                // Decode the base64 string.
                let decoded_value = match BASE64_ENGINE.decode(value_str) {
                    Ok(decoded_value) => decoded_value,
                    Err(e) => {
                        return FilterResult::operator_failed(
                            op_ctx.operator,
                            format!("failed to decode base64: {}", e),
                            op_ctx.filter_path,
                            op_ctx.obj_path,
                        )
                    }
                };

                // Convert the decoded value to a string.
                let decoded_value_str = match String::from_utf8(decoded_value) {
                    Ok(str) => str,
                    Err(e) => {
                        return FilterResult::operator_failed(
                            op_ctx.operator,
                            format!(
                                "failed to parse decoded base64 value as utf-8 string: {}",
                                e
                            ),
                            op_ctx.filter_path,
                            op_ctx.obj_path,
                        )
                    }
                };

                // Attempt to parse the decoded value as a JSON
                // value. If this fails, assume the value is
                // supposed to be a string.
                let decoded_value_json = match serde_json::from_str(&decoded_value_str) {
                    Ok(json) => json,
                    Err(_) => serde_json::Value::String(decoded_value_str),
                };

                self.inner_matches(
                    op_ctx.operator_arg,
                    Some(&decoded_value_json),
                    op_ctx.filter_path,
                    op_ctx.obj_path,
                )
            }
            _ => FilterResult::operator_failed(
                op_ctx.operator,
                "value is not a string",
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
        }
    }
    pub fn handle_proto_op(&self, op_ctx: OperatorContext) -> FilterResult {
        let value = match op_ctx.value {
            Some(v) => v,
            None => return FilterResult::key_not_found(op_ctx.filter_path, op_ctx.obj_path),
        };

        match (op_ctx.operator_arg, value) {
            (serde_json::Value::Object(op_arg), serde_json::Value::String(value_str)) => {
                // Extract `type` and `value` from the operator
                // argument. Both are required.
                let proto_type = match op_arg.get("type").and_then(|v| v.as_str()) {
                    Some(proto_type) => proto_type,
                    None => {
                        return FilterResult::fatal_invalid_filter(
                            format!("{} arg `type` not specified", op_ctx.operator),
                            op_ctx.filter_path,
                            op_ctx.obj_path,
                        )
                    }
                };
                let proto_value = match op_arg.get("value") {
                    Some(v) => v,
                    None => {
                        return FilterResult::fatal_invalid_filter(
                            format!("{} arg `value` not specified", op_ctx.operator),
                            op_ctx.filter_path,
                            op_ctx.obj_path,
                        )
                    }
                };

                // Ensure the protobuf decoder is provided.
                let decode_protobuf = match &self.decode_protobuf {
                    Some(decode_protobuf) => decode_protobuf,
                    None => {
                        return FilterResult::fatal_invalid_filter(
                            format!("{} protobuf decoder not provided", op_ctx.operator),
                            op_ctx.filter_path,
                            op_ctx.obj_path,
                        )
                    }
                };

                // Decode the base64 protobuf value string.
                let proto_value_encoded = match BASE64_ENGINE.decode(value_str) {
                    Ok(decoded_value) => decoded_value,
                    Err(e) => {
                        return FilterResult::operator_failed(
                            op_ctx.operator,
                            format!("failed to decode protobuf base64 value: {}", e),
                            op_ctx.filter_path,
                            op_ctx.obj_path,
                        )
                    }
                };

                // Decode the protobuf value.
                let proto_value_json =
                    match decode_protobuf.decode(proto_type.to_string(), proto_value_encoded) {
                        Ok(json) => json,
                        Err(e) => {
                            return FilterResult::operator_failed(
                                op_ctx.operator,
                                e,
                                op_ctx.filter_path,
                                op_ctx.obj_path,
                            )
                        }
                    };

                self.inner_matches(
                    proto_value,
                    Some(&proto_value_json),
                    op_ctx.filter_path,
                    op_ctx.obj_path,
                )
            }
            (serde_json::Value::Object(_), _) => FilterResult::operator_failed(
                op_ctx.operator,
                "value is not a string",
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
            _ => FilterResult::fatal_invalid_filter(
                format!("{} arg must be an object", op_ctx.operator),
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
        }
    }
    pub fn handle_stargate_op(&self, op_ctx: OperatorContext) -> FilterResult {
        let value = match op_ctx.value {
            Some(v) => v,
            None => return FilterResult::key_not_found(op_ctx.filter_path, op_ctx.obj_path),
        };

        match op_ctx.operator_arg {
            serde_json::Value::Object(op_arg) => {
                let type_url = match op_arg.get("type_url") {
                    Some(serde_json::Value::String(type_url)) => type_url,
                    _ => {
                        return FilterResult::fatal_invalid_filter(
                            format!("{} arg `type_url` not specified", op_ctx.operator),
                            op_ctx.filter_path,
                            op_ctx.obj_path,
                        )
                    }
                };

                let type_without_prefix = match type_url.strip_prefix('/') {
                    Some(t) => t,
                    None => {
                        return FilterResult::fatal_invalid_filter(
                            format!("{} arg `type_url` must start with `/`", op_ctx.operator),
                            op_ctx.filter_path,
                            op_ctx.obj_path,
                        )
                    }
                };

                let filter_value = match op_arg.get("value") {
                    Some(v) => v,
                    None => {
                        return FilterResult::fatal_invalid_filter(
                            format!("{} arg `value` not specified", op_ctx.operator),
                            op_ctx.filter_path,
                            op_ctx.obj_path,
                        )
                    }
                };

                self.inner_matches(
                    &json!({
                        "stargate": {
                            "type_url": type_url,
                            "value": {
                                "#proto": {
                                    "type": type_without_prefix,
                                    "value": filter_value,
                                }
                            }
                        }
                    }),
                    Some(value),
                    op_ctx.filter_path,
                    op_ctx.obj_path,
                )
            }
            _ => FilterResult::fatal_invalid_filter(
                format!("{} arg must be an object", op_ctx.operator),
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
        }
    }
}
