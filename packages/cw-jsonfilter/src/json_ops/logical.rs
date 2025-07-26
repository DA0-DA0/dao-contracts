// TODO: and, or, xor, not

use crate::{
    filter::append_array_path, json_ops::operator::OperatorContext, CwJsonFilter, FilterResult,
    ProtobufDecoder,
};

impl<D: ProtobufDecoder> CwJsonFilter<D> {
    pub fn handle_and_op(&self, op_ctx: OperatorContext) -> FilterResult {
        match op_ctx.operator_arg {
            serde_json::Value::Array(and_arg) => {
                for (i, sub_filter) in and_arg.iter().enumerate() {
                    let filter_path = &append_array_path(op_ctx.filter_path, i);
                    match self.inner_matches(sub_filter, op_ctx.value, filter_path, op_ctx.obj_path)
                    {
                        // Continue on success.
                        FilterResult::Pass => continue,
                        // Early return the first failure.
                        failure_result => return failure_result,
                    }
                }
                // Passes if all filters passed or there are no
                // filters.
                FilterResult::Pass
            }
            _ => FilterResult::fatal_invalid_filter(
                format!("{} arg must be an array", op_ctx.operator),
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
        }
    }

    pub fn handle_or_op(&self, op_ctx: OperatorContext) -> FilterResult {
        match op_ctx.operator_arg {
            serde_json::Value::Array(or_arg) => {
                for (i, sub_filter) in or_arg.iter().enumerate() {
                    let filter_path = &append_array_path(op_ctx.filter_path, i);
                    match self.inner_matches(sub_filter, op_ctx.value, filter_path, op_ctx.obj_path)
                    {
                        // Early return passed on first success.
                        FilterResult::Pass => return FilterResult::Pass,
                        // Ignore non-fatal errors.
                        FilterResult::Fail(_) => continue,
                        // Return fatal errors immediately.
                        FilterResult::Fatal(e) => return FilterResult::Fatal(e),
                    }
                }
                // Fails if all filters failed or there are no
                // filters.
                FilterResult::operator_failed(
                    op_ctx.operator,
                    "all filters failed",
                    op_ctx.filter_path,
                    op_ctx.obj_path,
                )
            }
            _ => FilterResult::fatal_invalid_filter(
                format!("{} arg must be an array", op_ctx.operator),
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
        }
    }

    pub fn handle_xor_op(&self, op_ctx: OperatorContext) -> FilterResult {
        match op_ctx.operator_arg {
            serde_json::Value::Array(xor_arg) => {
                let mut passed = 0;
                for (i, sub_filter) in xor_arg.iter().enumerate() {
                    let filter_path = &append_array_path(op_ctx.filter_path, i);
                    match self.inner_matches(sub_filter, op_ctx.value, filter_path, op_ctx.obj_path)
                    {
                        FilterResult::Pass => {
                            passed += 1;
                            // Early return failed on second
                            // success.
                            if passed > 1 {
                                return FilterResult::operator_failed(
                                    op_ctx.operator,
                                    "more than one filter passed",
                                    filter_path,
                                    op_ctx.obj_path,
                                );
                            }
                        }
                        // Ignore non-fatal errors.
                        FilterResult::Fail(_) => continue,
                        // Return fatal errors immediately.
                        FilterResult::Fatal(e) => return FilterResult::Fatal(e),
                    }
                }
                // Passes if exactly one filter passed.
                FilterResult::from_bool(
                    passed == 1,
                    op_ctx.operator,
                    format!("{} filters passed, expected exactly 1", passed),
                    op_ctx.filter_path,
                    op_ctx.obj_path,
                )
            }
            _ => FilterResult::fatal_invalid_filter(
                format!("{} arg must be an array", op_ctx.operator),
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
        }
    }

    pub fn handle_not_op(&self, op_ctx: OperatorContext) -> FilterResult {
        match self.inner_matches(
            op_ctx.operator_arg,
            op_ctx.value,
            op_ctx.filter_path,
            op_ctx.obj_path,
        ) {
            // Passes if the filter fails.
            FilterResult::Pass => FilterResult::operator_failed(
                op_ctx.operator,
                "filter needed to fail, but it passed",
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
            // Fails if the filter passes.
            FilterResult::Fail(_) => FilterResult::Pass,
            // Pass fatal errors through.
            FilterResult::Fatal(e) => FilterResult::Fatal(e),
        }
    }
}
