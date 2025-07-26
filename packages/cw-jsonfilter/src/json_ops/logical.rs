// TODO: and, or, xor, not

use crate::{filter::append_array_path, CwJsonFilter, FilterResult, ProtobufDecoder};

impl<D: ProtobufDecoder> CwJsonFilter<D> {
    pub fn handle_and_op(
        &self,
        operator: &str,
        operator_arg: &serde_json::Value,
        value: Option<&serde_json::Value>,
        filter_path: &str,
        obj_path: &str,
    ) -> FilterResult {
        match operator_arg {
            serde_json::Value::Array(and_arg) => {
                for (i, sub_filter) in and_arg.iter().enumerate() {
                    let filter_path = &append_array_path(filter_path, i);
                    match self.inner_matches(sub_filter, value, filter_path, obj_path) {
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
                format!("{} arg must be an array", operator),
                filter_path,
                obj_path,
            ),
        }
    }

    pub fn handle_or_op(
        &self,
        operator: &str,
        operator_arg: &serde_json::Value,
        value: Option<&serde_json::Value>,
        filter_path: &str,
        obj_path: &str,
    ) -> FilterResult {
        match operator_arg {
            serde_json::Value::Array(or_arg) => {
                for (i, sub_filter) in or_arg.iter().enumerate() {
                    let filter_path = &append_array_path(filter_path, i);
                    match self.inner_matches(sub_filter, value, filter_path, obj_path) {
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
                FilterResult::operator_failed(operator, "all filters failed", filter_path, obj_path)
            }
            _ => FilterResult::fatal_invalid_filter(
                format!("{} arg must be an array", operator),
                filter_path,
                obj_path,
            ),
        }
    }

    pub fn handle_xor_op(
        &self,
        operator: &str,
        operator_arg: &serde_json::Value,
        value: Option<&serde_json::Value>,
        filter_path: &str,
        obj_path: &str,
    ) -> FilterResult {
        match operator_arg {
            serde_json::Value::Array(xor_arg) => {
                let mut passed = 0;
                for (i, sub_filter) in xor_arg.iter().enumerate() {
                    let filter_path = &append_array_path(filter_path, i);
                    match self.inner_matches(sub_filter, value, filter_path, obj_path) {
                        FilterResult::Pass => {
                            passed += 1;
                            // Early return failed on second
                            // success.
                            if passed > 1 {
                                return FilterResult::operator_failed(
                                    operator,
                                    "more than one filter passed",
                                    filter_path,
                                    obj_path,
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
                    operator,
                    format!("{} filters passed, expected exactly 1", passed),
                    filter_path,
                    obj_path,
                )
            }
            _ => FilterResult::fatal_invalid_filter(
                format!("{} arg must be an array", operator),
                filter_path,
                obj_path,
            ),
        }
    }

    pub fn handle_not_op(
        &self,
        operator: &str,
        operator_arg: &serde_json::Value,
        value: Option<&serde_json::Value>,
        filter_path: &str,
        obj_path: &str,
    ) -> FilterResult {
        match self.inner_matches(operator_arg, value, filter_path, obj_path) {
            // Passes if the filter fails.
            FilterResult::Pass => FilterResult::operator_failed(
                operator,
                "filter needed to fail, but it passed",
                filter_path,
                obj_path,
            ),
            // Fails if the filter passes.
            FilterResult::Fail(_) => FilterResult::Pass,
            // Pass fatal errors through.
            FilterResult::Fatal(e) => FilterResult::Fatal(e),
        }
    }
}
