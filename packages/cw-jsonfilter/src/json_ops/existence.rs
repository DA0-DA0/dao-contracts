use crate::{json_ops::operator::OperatorContext, CwJsonFilter, FilterResult, ProtobufDecoder};

impl<D: ProtobufDecoder> CwJsonFilter<D> {
    pub fn handle_exists_op(&self, op_ctx: OperatorContext) -> FilterResult {
        match op_ctx.operator_arg {
            serde_json::Value::Bool(exists) => FilterResult::from_bool(
                *exists == op_ctx.value.is_some(),
                op_ctx.operator,
                match op_ctx.value.is_some() {
                    true => "value exists",
                    false => "value does not exist",
                },
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
            _ => FilterResult::fatal_invalid_filter(
                format!("{} arg must be a boolean", op_ctx.operator),
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
        }
    }
}
