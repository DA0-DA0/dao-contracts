use crate::{json_ops::operator::OperatorContext, CwJsonFilter, FilterResult, ProtobufDecoder};

impl<D: ProtobufDecoder> CwJsonFilter<D> {
    pub fn handle_eq_op(&self, op_ctx: OperatorContext) -> FilterResult {
        let value = match op_ctx.value {
            Some(v) => v,
            None => return FilterResult::key_not_found(op_ctx.filter_path, op_ctx.obj_path),
        };

        FilterResult::from_bool(
            value == op_ctx.operator_arg,
            op_ctx.operator,
            "value != filter",
            op_ctx.filter_path,
            op_ctx.obj_path,
        )
    }

    pub fn handle_neq_op(&self, op_ctx: OperatorContext) -> FilterResult {
        let value = match op_ctx.value {
            Some(v) => v,
            None => return FilterResult::key_not_found(op_ctx.filter_path, op_ctx.obj_path),
        };

        FilterResult::from_bool(
            value != op_ctx.operator_arg,
            op_ctx.operator,
            "value == filter",
            op_ctx.filter_path,
            op_ctx.obj_path,
        )
    }

    pub fn handle_range_op(&self, op_ctx: OperatorContext) -> FilterResult {
        let value = match op_ctx.value {
            Some(v) => v,
            None => return FilterResult::key_not_found(op_ctx.filter_path, op_ctx.obj_path),
        };

        match (
            op_ctx.operator_arg,
            op_ctx.operator_arg.as_array().map(|x| x.len()),
        ) {
            (serde_json::Value::Array(range_arg), Some(2)) => {
                let min = range_arg.first().unwrap();
                let max = range_arg.last().unwrap();

                // Ensure range is valid (same types and
                // ascending order).
                match lt_json(min, max) {
                    Some(true) => {}
                    Some(false) => {
                        return FilterResult::fatal_invalid_filter(
                            format!("{} args must be in ascending order", op_ctx.operator),
                            op_ctx.filter_path,
                            op_ctx.obj_path,
                        )
                    }
                    None => {
                        return FilterResult::fatal_invalid_filter(
                            format!(
                                "{} args must be both numbers or both strings",
                                op_ctx.operator
                            ),
                            op_ctx.filter_path,
                            op_ctx.obj_path,
                        )
                    }
                };

                let inclusive = !op_ctx.operator.ends_with("_exclusive");

                let min_passes = match gt_json(value, min) {
                    Some(true) => true,
                    // If not greater than the min, check if
                    // inclusive and equal to the min.
                    Some(false) => inclusive && value == min,
                    // If the types are incompatible, fail.
                    None => {
                        return FilterResult::operator_failed(
                            op_ctx.operator,
                            "filter bounds and value are not all numbers or all strings",
                            op_ctx.filter_path,
                            op_ctx.obj_path,
                        )
                    }
                };

                let max_passes = match lt_json(value, max) {
                    Some(true) => true,
                    // If not less than the max, check if inclusive
                    // and equal to the max.
                    Some(false) => inclusive && value == max,
                    // If the types are incompatible, fail.
                    None => {
                        return FilterResult::operator_failed(
                            op_ctx.operator,
                            "filter bounds and value are not all numbers or all strings",
                            op_ctx.filter_path,
                            op_ctx.obj_path,
                        )
                    }
                };

                FilterResult::from_bool(
                    min_passes && max_passes,
                    op_ctx.operator,
                    format!(
                        "value ({}) not between {} min ({}) and max ({})",
                        value,
                        match inclusive {
                            true => "(inclusive)",
                            false => "(exclusive)",
                        },
                        min,
                        max,
                    ),
                    op_ctx.filter_path,
                    op_ctx.obj_path,
                )
            }
            _ => FilterResult::fatal_invalid_filter(
                format!(
                    "{} arg must be an array of two numbers or two strings",
                    op_ctx.operator
                ),
                op_ctx.filter_path,
                op_ctx.obj_path,
            ),
        }
    }

    pub fn handle_lt_check_op(&self, op_ctx: OperatorContext) -> FilterResult {
        let value = match op_ctx.value {
            Some(v) => v,
            None => return FilterResult::key_not_found(op_ctx.filter_path, op_ctx.obj_path),
        };

        lt_json(value, op_ctx.operator_arg).map_or_else(
            || {
                FilterResult::operator_failed(
                    op_ctx.operator,
                    "filter and value are not both numbers or both strings",
                    op_ctx.filter_path,
                    op_ctx.obj_path,
                )
            },
            |lt| {
                FilterResult::from_bool(
                    lt || (op_ctx.operator == "$lte" && value == op_ctx.operator_arg),
                    op_ctx.operator,
                    if op_ctx.operator == "$lt" {
                        "value >= filter"
                    } else {
                        "value > filter"
                    },
                    op_ctx.filter_path,
                    op_ctx.obj_path,
                )
            },
        )
    }

    pub fn handle_gt_check_op(&self, op_ctx: OperatorContext) -> FilterResult {
        let value = match op_ctx.value {
            Some(v) => v,
            None => return FilterResult::key_not_found(op_ctx.filter_path, op_ctx.obj_path),
        };

        gt_json(value, op_ctx.operator_arg).map_or_else(
            || {
                FilterResult::operator_failed(
                    op_ctx.operator,
                    "filter and value are not both numbers or both strings",
                    op_ctx.filter_path,
                    op_ctx.obj_path,
                )
            },
            |gt| {
                FilterResult::from_bool(
                    gt || (op_ctx.operator == "$gte" && value == op_ctx.operator_arg),
                    op_ctx.operator,
                    if op_ctx.operator == "$gt" {
                        "value <= filter"
                    } else {
                        "value < filter"
                    },
                    op_ctx.filter_path,
                    op_ctx.obj_path,
                )
            },
        )
    }
}

/// Compares two `serde_json::Value` objects and determines if the first value is less than the second value.
///
/// # Arguments
///
/// * `a` - A reference to the first `serde_json::Value` to be compared. This can be a floating-point number, integer, unsigned integer, or a string.
/// * `b` - A reference to the second `serde_json::Value` to be compared. This should be of the same type as `a`.
///
/// # Returns
///
/// * `Some(true)` if `a` is less than `b`.
/// * `Some(false)` if `a` is greater than or equal to `b`.
/// * `None` if `a` and `b` are not of the same type or if they are of an unsupported type.
fn lt_json(a: &serde_json::Value, b: &serde_json::Value) -> Option<bool> {
    if let (Some(a), Some(b)) = (a.as_u64(), b.as_u64()) {
        Some(a < b)
    } else if let (Some(a), Some(b)) = (a.as_i64(), b.as_i64()) {
        Some(a < b)
    } else if let (Some(a), Some(b)) = (a.as_str(), b.as_str()) {
        Some(a < b)
    } else if let (Some(a), Some(b)) = (a.as_f64(), b.as_f64()) {
        // if both values being compared are not of the same numeric type
        // (u64/i64/f64), fallback both to f64.
        // this deliberately casts the original serde_json numbers into f64
        // and compares them based on that instead of the original types.
        Some(a < b)
    } else {
        None
    }
}

/// Compares two `serde_json::Value` objects and determines if the first value is greater than the second value.
///
/// # Arguments
///
/// * `a` - A reference to the first `serde_json::Value` to be compared. This can be a floating-point number, integer, unsigned integer, or a string.
/// * `b` - A reference to the second `serde_json::Value` to be compared. This should be of the same type as `a`.
///
/// # Returns
///
/// * `Some(true)` if `a` is greater than `b`.
/// * `Some(false)` if `a` is less than or equal to `b`.
/// * `None` if `a` and `b` are not of the same type or if they are of an unsupported type.
fn gt_json(a: &serde_json::Value, b: &serde_json::Value) -> Option<bool> {
    if let (Some(a), Some(b)) = (a.as_u64(), b.as_u64()) {
        Some(a > b)
    } else if let (Some(a), Some(b)) = (a.as_i64(), b.as_i64()) {
        Some(a > b)
    } else if let (Some(a), Some(b)) = (a.as_str(), b.as_str()) {
        Some(a > b)
    } else if let (Some(a), Some(b)) = (a.as_f64(), b.as_f64()) {
        // if both values being compared are not of the same numeric type
        // (u64/i64/f64), fallback both to f64.
        // this deliberately casts the original serde_json numbers into f64
        // and compares them based on that instead of the original types.
        Some(a > b)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn lt_json_f64_f64_happy() {
        let filter_val: f64 = 86.0;
        let object_val: f64 = 85.0;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$lt": filter_val } }),
            &json!({ "number": object_val }),
        );

        assert!(check_result.is_pass());
    }

    #[test]
    fn lt_json_i64_i64_happy() {
        let filter_val: i64 = -20;
        let object_val: i64 = -21;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$lt": filter_val } }),
            &json!({ "number": object_val }),
        );

        assert!(check_result.is_pass());
    }

    #[test]
    fn lt_json_u64_u64_happy() {
        let filter_val: u64 = 21;
        let object_val: u64 = 20;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$lt": filter_val } }),
            &json!({ "number": object_val }),
        );

        assert!(check_result.is_pass());
    }

    #[test]
    fn lt_json_f64_u64_err() {
        let filter_val: f64 = 85.0;
        let object_val: u64 = 86;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$lt": filter_val } }),
            &json!({ "number": object_val }),
        );

        assert!(check_result.is_fail());
    }

    #[test]
    fn lt_json_u64_f64_err() {
        let filter_val: u64 = 85;
        let object_val: f64 = 86.0;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$lt": filter_val } }),
            &json!({ "number": object_val }),
        );

        assert!(check_result.is_fail());
    }

    #[test]
    fn lt_json_i64_f64_err() {
        let filter_val: i64 = 1;
        let object_val: f64 = 2.0;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$lt": filter_val } }),
            &json!({ "number": object_val }),
        );

        assert!(check_result.is_fail());
    }

    #[test]
    fn lt_json_f64_i64_err() {
        let filter_val: f64 = 1.0;
        let object_val: i64 = 2;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$lt": filter_val } }),
            &json!({ "number": object_val }),
        );

        assert!(check_result.is_fail());
    }

    #[test]
    fn gt_json_f64_f64_happy() {
        let filter_val: f64 = 85.0;
        let object_val: f64 = 86.0;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$gt": filter_val } }),
            &json!({ "number": object_val }),
        );

        assert!(check_result.is_pass());
    }

    #[test]
    fn gt_json_i64_i64_happy() {
        let filter_val: i64 = -21;
        let object_val: i64 = -20;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$gt": filter_val } }),
            &json!({ "number": object_val }),
        );

        assert!(check_result.is_pass());
    }

    #[test]
    fn gt_json_u64_u64_happy() {
        let filter_val: u64 = 20;
        let object_val: u64 = 21;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$gt": filter_val } }),
            &json!({ "number": object_val }),
        );

        assert!(check_result.is_pass());
    }

    #[test]
    fn gt_json_f64_u64_err() {
        let filter_val: f64 = 85.0;
        let object_val: u64 = 86;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$lt": filter_val } }),
            &json!({ "number": object_val }),
        );

        assert!(check_result.is_fail());
    }

    #[test]
    fn gt_json_u64_f64_err() {
        let filter_val: u64 = 85;
        let object_val: f64 = 86.0;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$lt": filter_val } }),
            &json!({ "number": object_val }),
        );

        assert!(check_result.is_fail());
    }

    #[test]
    fn gt_json_i64_f64_err() {
        let filter_val: i64 = 2;
        let object_val: f64 = 1.0;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$gt": filter_val } }),
            &json!({ "number": object_val }),
        );

        assert!(check_result.is_fail());
    }

    #[test]
    fn gt_json_f64_i64_err() {
        let filter_val: i64 = 2;
        let object_val: f64 = 1.0;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$gt": filter_val } }),
            &json!({ "number": object_val }),
        );

        assert!(check_result.is_fail());
    }
}
