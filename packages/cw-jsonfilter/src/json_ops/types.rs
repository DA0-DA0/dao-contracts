use crate::{json_ops::operator::OperatorContext, CwJsonFilter, FilterResult, ProtobufDecoder};

impl<D: ProtobufDecoder> CwJsonFilter<D> {
    pub fn handle_type_op(&self, op_ctx: OperatorContext) -> FilterResult {
        let value = match op_ctx.value {
            Some(v) => v,
            None => return FilterResult::key_not_found(op_ctx.filter_path, op_ctx.obj_path),
        };

        match op_ctx.operator_arg {
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
                            format!(
                                "{} arg must be a valid type, got `{}`",
                                op_ctx.operator, type_str
                            ),
                            op_ctx.filter_path,
                            op_ctx.obj_path,
                        );
                    }
                };

                FilterResult::from_bool(
                    check,
                    op_ctx.operator,
                    reason,
                    op_ctx.filter_path,
                    op_ctx.obj_path,
                )
            }
            _ => FilterResult::fatal_invalid_filter(
                format!("{} arg must be a string", op_ctx.operator),
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
        }
    }
}
