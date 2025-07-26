use crate::{
    filter::append_array_path, json_ops::operator::OperatorContext, CwJsonFilter, FilterResult,
    ProtobufDecoder,
};

impl<D: ProtobufDecoder> CwJsonFilter<D> {
    pub fn handle_contains_op(&self, op_ctx: OperatorContext) -> FilterResult {
        let value = match op_ctx.value {
            Some(v) => v,
            None => return FilterResult::key_not_found(op_ctx.filter_path, op_ctx.obj_path),
        };

        match (op_ctx.operator_arg, value) {
            // when value is a string, operator_arg must be a string
            (serde_json::Value::String(op_arg), serde_json::Value::String(value_str)) => {
                FilterResult::from_bool(
                    value_str.contains(op_arg),
                    op_ctx.operator,
                    "string value does not contain filter value",
                    op_ctx.filter_path,
                    op_ctx.obj_path,
                )
            }
            (_, serde_json::Value::String(_)) => FilterResult::operator_failed(
                op_ctx.operator,
                format!(
                    "{} arg must be a string when applied to a string value",
                    op_ctx.operator,
                ),
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
            // when value is an array, operator_arg can be anything
            (_, serde_json::Value::Array(value_list)) => FilterResult::from_bool(
                value_list.iter().any(|x| x == op_ctx.operator_arg),
                op_ctx.operator,
                "array value does not contain filter value",
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
            // value is incorrect type
            _ => FilterResult::operator_failed(
                op_ctx.operator,
                "value is not a string or an array",
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
        }
    }

    pub fn handle_overlaps_op(&self, op_ctx: OperatorContext) -> FilterResult {
        let value = match op_ctx.value {
            Some(v) => v,
            None => return FilterResult::key_not_found(op_ctx.filter_path, op_ctx.obj_path),
        };

        match (op_ctx.operator_arg, value) {
            (serde_json::Value::Array(op_arg), serde_json::Value::Array(value_list)) => {
                FilterResult::from_bool(
                    value_list.iter().any(|x| op_arg.contains(x)),
                    op_ctx.operator,
                    "array value does not overlap with filter array",
                    op_ctx.filter_path,
                    op_ctx.obj_path,
                )
            }
            (serde_json::Value::Array(_), _) => FilterResult::operator_failed(
                op_ctx.operator,
                "value is not an array",
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
            _ => FilterResult::fatal_invalid_filter(
                format!("{} arg must be an array", op_ctx.operator),
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
        }
    }

    pub fn handle_any_op(&self, op_ctx: OperatorContext) -> FilterResult {
        let value = match op_ctx.value {
            Some(v) => v,
            None => return FilterResult::key_not_found(op_ctx.filter_path, op_ctx.obj_path),
        };

        match value {
            serde_json::Value::Array(value_list) => {
                for (i, item) in value_list.iter().enumerate() {
                    let obj_path = &append_array_path(op_ctx.obj_path, i);
                    match self.inner_matches(
                        op_ctx.operator_arg,
                        Some(item),
                        op_ctx.filter_path,
                        obj_path,
                    ) {
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
                    op_ctx.operator,
                    "no values passed the filter",
                    op_ctx.filter_path,
                    op_ctx.obj_path,
                )
            }
            _ => FilterResult::operator_failed(
                op_ctx.operator,
                "value is not an array",
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
        }
    }

    pub fn handle_all_op(&self, op_ctx: OperatorContext) -> FilterResult {
        let value = match op_ctx.value {
            Some(v) => v,
            None => return FilterResult::key_not_found(op_ctx.filter_path, op_ctx.obj_path),
        };

        match value {
            serde_json::Value::Array(value_list) => {
                for (i, item) in value_list.iter().enumerate() {
                    let obj_path = &append_array_path(op_ctx.obj_path, i);
                    match self.inner_matches(
                        op_ctx.operator_arg,
                        Some(item),
                        op_ctx.filter_path,
                        obj_path,
                    ) {
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
                op_ctx.operator,
                "value is not an array",
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
        }
    }

    pub fn handle_starts_ends_with_op(&self, op_ctx: OperatorContext) -> FilterResult {
        let value = match op_ctx.value {
            Some(v) => v,
            None => return FilterResult::key_not_found(op_ctx.filter_path, op_ctx.obj_path),
        };

        match (op_ctx.operator_arg, value) {
            (serde_json::Value::String(op_arg), serde_json::Value::String(value_str)) => {
                FilterResult::from_bool(
                    if op_ctx.operator == "$startsWith" {
                        value_str.starts_with(op_arg)
                    } else {
                        value_str.ends_with(op_arg)
                    },
                    op_ctx.operator,
                    if op_ctx.operator == "$startsWith" {
                        "value does not start with filter value"
                    } else {
                        "value does not end with filter value"
                    },
                    op_ctx.filter_path,
                    op_ctx.obj_path,
                )
            }
            (serde_json::Value::String(_), _) => FilterResult::operator_failed(
                op_ctx.operator,
                "value is not a string",
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
            _ => FilterResult::fatal_invalid_filter(
                format!("{} arg must be a string", op_ctx.operator),
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
        }
    }
}
