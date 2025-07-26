use crate::{filter::append_array_path, CwJsonFilter, FilterResult, ProtobufDecoder};

impl<D: ProtobufDecoder> CwJsonFilter<D> {
    pub fn handle_contains_op(
        &self,
        operator: &str,
        operator_arg: &serde_json::Value,
        value: &serde_json::Value,
        filter_path: &str,
        obj_path: &str,
    ) -> FilterResult {
        match (operator_arg, value) {
            // when value is a string, operator_arg must be a string
            (serde_json::Value::String(op_arg), serde_json::Value::String(value_str)) => {
                FilterResult::from_bool(
                    value_str.contains(op_arg),
                    operator,
                    "string value does not contain filter value",
                    filter_path,
                    obj_path,
                )
            }
            (_, serde_json::Value::String(_)) => FilterResult::operator_failed(
                operator,
                format!(
                    "{} arg must be a string when applied to a string value",
                    operator,
                ),
                filter_path,
                obj_path,
            ),
            // when value is an array, operator_arg can be anything
            (_, serde_json::Value::Array(value_list)) => FilterResult::from_bool(
                value_list.iter().any(|x| x == operator_arg),
                operator,
                "array value does not contain filter value",
                filter_path,
                obj_path,
            ),
            // value is incorrect type
            _ => FilterResult::operator_failed(
                operator,
                "value is not a string or an array",
                filter_path,
                obj_path,
            ),
        }
    }

    pub fn handle_overlaps_op(
        &self,
        operator: &str,
        operator_arg: &serde_json::Value,
        value: &serde_json::Value,
        filter_path: &str,
        obj_path: &str,
    ) -> FilterResult {
        match (operator_arg, value) {
            (serde_json::Value::Array(op_arg), serde_json::Value::Array(value_list)) => {
                FilterResult::from_bool(
                    value_list.iter().any(|x| op_arg.contains(x)),
                    operator,
                    "array value does not overlap with filter array",
                    filter_path,
                    obj_path,
                )
            }
            (serde_json::Value::Array(_), _) => FilterResult::operator_failed(
                operator,
                "value is not an array",
                filter_path,
                obj_path,
            ),
            _ => FilterResult::fatal_invalid_filter(
                format!("{} arg must be an array", operator),
                filter_path,
                obj_path,
            ),
        }
    }

    pub fn handle_any_op(
        &self,
        operator: &str,
        operator_arg: &serde_json::Value,
        value: &serde_json::Value,
        filter_path: &str,
        obj_path: &str,
    ) -> FilterResult {
        match value {
            serde_json::Value::Array(value_list) => {
                for (i, item) in value_list.iter().enumerate() {
                    let obj_path = &append_array_path(obj_path, i);
                    match self.inner_matches(operator_arg, Some(item), filter_path, obj_path) {
                        // Early return passed on first success.
                        FilterResult::Pass => return FilterResult::Pass,
                        // Ignore non-fatal errors.
                        FilterResult::Fail(_) => continue,
                        // Return fatal errors immediately.
                        FilterResult::Fatal(e) => return FilterResult::Fatal(e),
                    }
                }
                // Fails if no values passed the filter or there are
                // no values.
                FilterResult::operator_failed(
                    operator,
                    "no values passed the filter",
                    filter_path,
                    obj_path,
                )
            }
            _ => FilterResult::operator_failed(
                operator,
                "value is not an array",
                filter_path,
                obj_path,
            ),
        }
    }

    pub fn handle_all_op(
        &self,
        operator: &str,
        operator_arg: &serde_json::Value,
        value: &serde_json::Value,
        filter_path: &str,
        obj_path: &str,
    ) -> FilterResult {
        match value {
            serde_json::Value::Array(value_list) => {
                for (i, item) in value_list.iter().enumerate() {
                    let obj_path = &append_array_path(obj_path, i);
                    match self.inner_matches(operator_arg, Some(item), filter_path, obj_path) {
                        // Continue on success.
                        FilterResult::Pass => continue,
                        // Early return the first failure.
                        failure_result => return failure_result,
                    }
                }
                // Passes if all values passed the filter or there
                // are no values.
                FilterResult::Pass
            }
            _ => FilterResult::operator_failed(
                operator,
                "value is not an array",
                filter_path,
                obj_path,
            ),
        }
    }

    pub fn handle_starts_ends_with_op(
        &self,
        operator: &str,
        operator_arg: &serde_json::Value,
        value: &serde_json::Value,
        filter_path: &str,
        obj_path: &str,
    ) -> FilterResult {
        match (operator_arg, value) {
            (serde_json::Value::String(op_arg), serde_json::Value::String(value_str)) => {
                FilterResult::from_bool(
                    if operator == "$startsWith" {
                        value_str.starts_with(op_arg)
                    } else {
                        value_str.ends_with(op_arg)
                    },
                    operator,
                    if operator == "$startsWith" {
                        "value does not start with filter value"
                    } else {
                        "value does not end with filter value"
                    },
                    filter_path,
                    obj_path,
                )
            }
            (serde_json::Value::String(_), _) => FilterResult::operator_failed(
                operator,
                "value is not a string",
                filter_path,
                obj_path,
            ),
            _ => FilterResult::fatal_invalid_filter(
                format!("{} arg must be a string", operator),
                filter_path,
                obj_path,
            ),
        }
    }
}
