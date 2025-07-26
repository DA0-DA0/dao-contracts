use crate::{gt_json, lt_json, CwJsonFilter, FilterResult, ProtobufDecoder};

impl<D: ProtobufDecoder> CwJsonFilter<D> {
    pub fn handle_eq_op(
        &self,
        operator: &str,
        operator_arg: &serde_json::Value,
        value: &serde_json::Value,
        filter_path: &str,
        obj_path: &str,
    ) -> FilterResult {
        FilterResult::from_bool(
            value == operator_arg,
            operator,
            "value != filter",
            filter_path,
            obj_path,
        )
    }

    pub fn handle_neq_op(
        &self,
        operator: &str,
        operator_arg: &serde_json::Value,
        value: &serde_json::Value,
        filter_path: &str,
        obj_path: &str,
    ) -> FilterResult {
        FilterResult::from_bool(
            value != operator_arg,
            operator,
            "value == filter",
            filter_path,
            obj_path,
        )
    }

    pub fn handle_range_op(
        &self,
        operator: &str,
        operator_arg: &serde_json::Value,
        value: &serde_json::Value,
        filter_path: &str,
        obj_path: &str,
    ) -> FilterResult {
        match (operator_arg, operator_arg.as_array().map(|x| x.len())) {
            (serde_json::Value::Array(range_arg), Some(2)) => {
                let min = range_arg.first().unwrap();
                let max = range_arg.last().unwrap();

                // Ensure range is valid (same types and
                // ascending order).
                match lt_json(min, max) {
                    Some(true) => {}
                    Some(false) => {
                        return FilterResult::fatal_invalid_filter(
                            format!("{} args must be in ascending order", operator),
                            filter_path,
                            obj_path,
                        )
                    }
                    None => {
                        return FilterResult::fatal_invalid_filter(
                            format!("{} args must be both numbers or both strings", operator),
                            filter_path,
                            obj_path,
                        )
                    }
                };

                let inclusive = !operator.ends_with("_exclusive");

                let min_passes = match gt_json(value, min) {
                    Some(true) => true,
                    // If not greater than the min, check if
                    // inclusive and equal to the min.
                    Some(false) => inclusive && value == min,
                    // If the types are incompatible, fail.
                    None => {
                        return FilterResult::operator_failed(
                            operator,
                            "filter bounds and value are not all numbers or all strings",
                            filter_path,
                            obj_path,
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
                            operator,
                            "filter bounds and value are not all numbers or all strings",
                            filter_path,
                            obj_path,
                        )
                    }
                };

                FilterResult::from_bool(
                    min_passes && max_passes,
                    operator,
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
                    filter_path,
                    obj_path,
                )
            }
            _ => FilterResult::fatal_invalid_filter(
                format!(
                    "{} arg must be an array of two numbers or two strings",
                    operator
                ),
                filter_path,
                obj_path,
            ),
        }
    }

    pub fn handle_lt_check_op(
        &self,
        operator: &str,
        operator_arg: &serde_json::Value,
        value: &serde_json::Value,
        filter_path: &str,
        obj_path: &str,
    ) -> FilterResult {
        lt_json(value, operator_arg).map_or_else(
            || {
                FilterResult::operator_failed(
                    operator,
                    "filter and value are not both numbers or both strings",
                    filter_path,
                    obj_path,
                )
            },
            |lt| {
                FilterResult::from_bool(
                    lt || (operator == "$lte" && value == operator_arg),
                    operator,
                    if operator == "$lt" {
                        "value >= filter"
                    } else {
                        "value > filter"
                    },
                    filter_path,
                    obj_path,
                )
            },
        )
    }

    pub fn handle_gt_check_op(
        &self,
        operator: &str,
        operator_arg: &serde_json::Value,
        value: &serde_json::Value,
        filter_path: &str,
        obj_path: &str,
    ) -> FilterResult {
        gt_json(value, operator_arg).map_or_else(
            || {
                FilterResult::operator_failed(
                    operator,
                    "filter and value are not both numbers or both strings",
                    filter_path,
                    obj_path,
                )
            },
            |gt| {
                FilterResult::from_bool(
                    gt || (operator == "$gte" && value == operator_arg),
                    operator,
                    if operator == "$gt" {
                        "value <= filter"
                    } else {
                        "value < filter"
                    },
                    filter_path,
                    obj_path,
                )
            },
        )
    }
}
