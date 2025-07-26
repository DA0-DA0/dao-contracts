use crate::{
    gt_json, json_ops::operator::OperatorContext, lt_json, CwJsonFilter, FilterResult,
    ProtobufDecoder,
};

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
