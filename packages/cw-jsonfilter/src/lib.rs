pub mod math;
pub mod result;
mod test;

use base64::{
    alphabet,
    engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig},
    Engine as _,
};
pub use math::{gt_json, lt_json};
pub use result::{FilterFailure, FilterFatalError, FilterResult};

/// Base64 decoding engine
const BASE64_ENGINE: GeneralPurpose = GeneralPurpose::new(
    &alphabet::STANDARD,
    GeneralPurposeConfig::new()
        .with_decode_allow_trailing_bits(true)
        .with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

/// Tests an object against a filter and returns whether the object passes the
/// filter.
///
/// # Arguments
///
/// * `filter` - A reference to a `serde_json::Value` representing the filter to
///   apply. Must be an object.
/// * `obj` - A reference to a `serde_json::Value` representing the object to
///   match against. Must be an object.
///
/// # Returns
///
/// * `FilterResult::Pass` if the filter matches the object.
/// * `FilterResult::Fail(FilterError)` if the filter does not match the object.
///
/// # Examples
///
/// ```
/// use serde_json::json;
/// use cw_jsonfilter::test;
///
/// let filter = json!({"name": "John", "age": 30});
/// let obj = json!({"name": "John", "age": 30, "city": "New York"});
///
/// assert!(test(&filter, &obj).is_pass());
/// ```
#[must_use]
pub fn test(filter: &serde_json::Value, obj: &serde_json::Value) -> FilterResult {
    inner_test(filter, obj, "@", "@")
}

fn inner_test(
    filter: &serde_json::Value,
    obj: &serde_json::Value,
    filter_path: &str,
    obj_path: &str,
) -> FilterResult {
    match filter {
        // Arrays are implicitly an $and operation.
        serde_json::Value::Array(filter_list) => {
            for (i, sub_filter) in filter_list.iter().enumerate() {
                let filter_path = &format!("{}[{}]", filter_path, i);
                match inner_test(sub_filter, obj, filter_path, obj_path) {
                    // If success, continue to next filter.
                    FilterResult::Pass => continue,
                    // If failure, return the error.
                    failure_result => return failure_result,
                }
            }

            // Passes if all filters matched or there are no filters.
            FilterResult::Pass
        }
        // Objects match each key recursively, also implicitly an $and
        // operation.
        serde_json::Value::Object(filter_obj) => {
            for (filter_key, filter_val) in filter_obj {
                let filter_path = &format!("{}.{}", filter_path, filter_key);
                let is_operator = filter_key.starts_with('$');
                let is_transformer = filter_key.starts_with('#');
                if is_operator || is_transformer {
                    // only apply transformers to the object path—not operators
                    let obj_path = match is_transformer {
                        true => format!("{}.{}", obj_path, filter_key),
                        false => obj_path.to_string(),
                    };

                    match test_operator(filter_key, filter_val, Some(obj), filter_path, &obj_path) {
                        // If success, continue to next key.
                        FilterResult::Pass => continue,
                        // If failure, return the error.
                        failure_result => return failure_result,
                    }
                } else {
                    let obj_path = &format!("{}.{}", obj_path, filter_key);
                    match obj.get(filter_key) {
                        Some(value) => match inner_test(filter_val, value, filter_path, obj_path) {
                            // If success, continue to next key.
                            FilterResult::Pass => continue,
                            // If failure, return the error.
                            failure_result => return failure_result,
                        },
                        // If value not found in object, return error.
                        None => return FilterResult::key_not_found(filter_path, obj_path),
                    };
                }
            }

            // Passes if all keys matched or there are no keys.
            FilterResult::Pass
        }
        // Match primitive values directly.
        _ => {
            if filter == obj {
                FilterResult::Pass
            } else {
                FilterResult::operator_failed(
                    "implicit equality check",
                    "value does not match filter",
                    filter_path,
                    obj_path,
                )
            }
        }
    }
}

/// Matches a filter operator against a value from the object and determines if
/// the condition is met.
///
/// # Arguments
///
/// * `operator` - The operator to apply to the value.
/// * `operator_arg` - The argument associated with the operator.
/// * `value` - The value to apply the operator filter to. If None, the value
///   does not exist in the object at the path specified by `obj_path`.
/// * `filter_path` - The path to the filter operator being applied.
/// * `obj_path` - The path to the value from the object bei.
///
/// # Returns
///
/// * `Ok(true)` if the condition specified by the filter operator is met.
/// * `Ok(false)` if the condition specified by the filter operator is not met.
/// * `Err(FilterError)` if there is an error in the filtering process.
///
/// # Errors
///
/// This function will return an error if:
/// * An unknown operator is used.
/// * The operator argument or value is not the expected type based on context.
/// * The filter format is invalid.
/// * The value is not found in the object when it is needed.
pub fn test_operator(
    operator: &str,
    operator_arg: &serde_json::Value,
    value: Option<&serde_json::Value>,
    filter_path: &str,
    obj_path: &str,
) -> FilterResult {
    // It is the caller's responsibility to pass a valid operator.
    if !operator.starts_with('$') && !operator.starts_with('#') {
        return FilterResult::fatal_unknown_operator(operator, filter_path, obj_path);
    }

    match operator {
        // Existence operator (does not require a value)
        "$exists" => match operator_arg {
            serde_json::Value::Bool(exists) => FilterResult::from_bool(
                *exists == value.is_some(),
                operator,
                match value.is_some() {
                    true => "value exists",
                    false => "value does not exist",
                },
                filter_path,
                obj_path,
            ),
            _ => FilterResult::fatal_invalid_filter(
                format!("{} arg must be a boolean", operator),
                filter_path,
                obj_path,
            ),
        },
        // The rest of the operators require a value.
        _ => {
            let value = match value {
                Some(v) => v,
                None => return FilterResult::key_not_found(filter_path, obj_path),
            };

            match operator {
                // Logical operators
                "$and" => match operator_arg {
                    serde_json::Value::Array(and_arg) => {
                        for (i, sub_filter) in and_arg.iter().enumerate() {
                            let filter_path = &format!("{}[{}]", filter_path, i);
                            match inner_test(sub_filter, value, filter_path, obj_path) {
                                // Continue on success.
                                FilterResult::Pass => continue,
                                // Early return the first failure.
                                failure_result => return failure_result,
                            }
                        }
                        // Passes if all filters passed or there are no filters.
                        FilterResult::Pass
                    }
                    _ => FilterResult::fatal_invalid_filter(
                        format!("{} arg must be an array", operator),
                        filter_path,
                        obj_path,
                    ),
                },
                "$or" => match operator_arg {
                    serde_json::Value::Array(or_arg) => {
                        for (i, sub_filter) in or_arg.iter().enumerate() {
                            let filter_path = &format!("{}[{}]", filter_path, i);
                            match inner_test(sub_filter, value, filter_path, obj_path) {
                                // Early return passed on first success.
                                FilterResult::Pass => return FilterResult::Pass,
                                // Ignore non-fatal errors.
                                FilterResult::Fail(_) => continue,
                                // Return fatal errors immediately.
                                FilterResult::Fatal(e) => return FilterResult::Fatal(e),
                            }
                        }
                        // Fails if all filters failed or there are no filters.
                        FilterResult::operator_failed(
                            operator,
                            "all filters failed",
                            filter_path,
                            obj_path,
                        )
                    }
                    _ => FilterResult::fatal_invalid_filter(
                        format!("{} arg must be an array", operator),
                        filter_path,
                        obj_path,
                    ),
                },
                "$nor" => match operator_arg {
                    serde_json::Value::Array(nor_arg) => {
                        for (i, sub_filter) in nor_arg.iter().enumerate() {
                            let filter_path = &format!("{}[{}]", filter_path, i);
                            match inner_test(sub_filter, value, filter_path, obj_path) {
                                // Early return failed on first success.
                                FilterResult::Pass => {
                                    return FilterResult::operator_failed(
                                        operator,
                                        "a filter passed",
                                        filter_path,
                                        obj_path,
                                    )
                                }
                                // Ignore non-fatal errors.
                                FilterResult::Fail(_) => continue,
                                // Return fatal errors immediately.
                                FilterResult::Fatal(e) => return FilterResult::Fatal(e),
                            }
                        }
                        // Passes if all filters failed or there are no filters.
                        FilterResult::Pass
                    }
                    _ => FilterResult::fatal_invalid_filter(
                        format!("{} arg must be an array", operator),
                        filter_path,
                        obj_path,
                    ),
                },
                "$xor" => match operator_arg {
                    serde_json::Value::Array(xor_arg) => {
                        let mut passed = 0;
                        for (i, sub_filter) in xor_arg.iter().enumerate() {
                            let filter_path = &format!("{}[{}]", filter_path, i);
                            match inner_test(sub_filter, value, filter_path, obj_path) {
                                FilterResult::Pass => {
                                    passed += 1;
                                    // Early return failed on second success.
                                    if passed > 1 {
                                        return FilterResult::operator_failed(
                                            operator,
                                            "a filter passed",
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
                },
                "$not" => match inner_test(operator_arg, value, filter_path, obj_path) {
                    // Passes if the filter fails.
                    FilterResult::Pass => FilterResult::operator_failed(
                        operator,
                        "filter passed",
                        filter_path,
                        obj_path,
                    ),
                    // Fails if the filter passes.
                    FilterResult::Fail(_) => FilterResult::Pass,
                    // Pass fatal errors through.
                    FilterResult::Fatal(e) => FilterResult::Fatal(e),
                },

                // Comparison operators
                "$eq" => FilterResult::from_bool(
                    value == operator_arg,
                    operator,
                    "value != filter",
                    filter_path,
                    obj_path,
                ),
                "$ne" | "$neq" => FilterResult::from_bool(
                    value != operator_arg,
                    operator,
                    "value == filter",
                    filter_path,
                    obj_path,
                ),
                "$lt" => lt_json(value, operator_arg).map_or_else(
                    || {
                        FilterResult::operator_failed(
                            operator,
                            format!(
                                "{} arg and value are not both numbers or both strings",
                                operator
                            ),
                            filter_path,
                            obj_path,
                        )
                    },
                    |lt| {
                        FilterResult::from_bool(
                            lt,
                            operator,
                            "value >= filter",
                            filter_path,
                            obj_path,
                        )
                    },
                ),
                // Check less than before equality to ensure JSON types are
                // compatible.
                "$lte" => lt_json(value, operator_arg).map_or_else(
                    || {
                        FilterResult::operator_failed(
                            operator,
                            format!(
                                "{} arg and value are not both numbers or both strings",
                                operator
                            ),
                            filter_path,
                            obj_path,
                        )
                    },
                    |lt| {
                        FilterResult::from_bool(
                            lt || value == operator_arg,
                            operator,
                            "value > filter",
                            filter_path,
                            obj_path,
                        )
                    },
                ),
                "$gt" => gt_json(value, operator_arg).map_or_else(
                    || {
                        FilterResult::operator_failed(
                            operator,
                            format!(
                                "{} arg and value are not both numbers or both strings",
                                operator
                            ),
                            filter_path,
                            obj_path,
                        )
                    },
                    |gt| {
                        FilterResult::from_bool(
                            gt,
                            operator,
                            "value <= filter",
                            filter_path,
                            obj_path,
                        )
                    },
                ),
                // Check greater than before equality to ensure JSON types are
                // compatible.
                "$gte" => gt_json(value, operator_arg).map_or_else(
                    || {
                        FilterResult::operator_failed(
                            operator,
                            format!(
                                "{} arg and value are not both numbers or both strings",
                                operator
                            ),
                            filter_path,
                            obj_path,
                        )
                    },
                    |gt| {
                        FilterResult::from_bool(
                            gt || value == operator_arg,
                            operator,
                            "value < filter",
                            filter_path,
                            obj_path,
                        )
                    },
                ),
                "$range" | "$range_exclusive" | "$between" | "$between_exclusive" => {
                    match (operator_arg, operator_arg.as_array().map(|x| x.len())) {
                        (serde_json::Value::Array(range_arg), Some(2)) => {
                            let min = range_arg.first().unwrap();
                            let max = range_arg.last().unwrap();

                            // Ensure range is valid (same types and ascending
                            // order).
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
                                        format!(
                                            "{} args must be both numbers or both strings",
                                            operator
                                        ),
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
                                        format!(
                                        "{} arg minimum and value are not both numbers or both strings",
                                        operator
                                    ),
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
                                        format!(
                                        "{} arg maximum and value are not both numbers or both strings",
                                        operator
                                    ),
                                        filter_path,
                                        obj_path,
                                    )
                                }
                            };

                            FilterResult::from_bool(
                                min_passes && max_passes,
                                operator,
                                format!(
                                    "value not {} min and max",
                                    match inclusive {
                                        true => "between (inclusive)",
                                        false => "between (exclusive)",
                                    }
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
                "$type" => match operator_arg {
                    serde_json::Value::String(type_str) => match type_str.to_lowercase().as_str() {
                        "null" => FilterResult::from_bool(
                            value.is_null(),
                            operator,
                            "value is not null",
                            filter_path,
                            obj_path,
                        ),
                        "boolean" => FilterResult::from_bool(
                            value.is_boolean(),
                            operator,
                            "value is not a boolean",
                            filter_path,
                            obj_path,
                        ),
                        "number" => FilterResult::from_bool(
                            value.is_number(),
                            operator,
                            "value is not a number",
                            filter_path,
                            obj_path,
                        ),
                        "string" => FilterResult::from_bool(
                            value.is_string(),
                            operator,
                            "value is not a string",
                            filter_path,
                            obj_path,
                        ),
                        "array" => FilterResult::from_bool(
                            value.is_array(),
                            operator,
                            "value is not an array",
                            filter_path,
                            obj_path,
                        ),
                        "object" => FilterResult::from_bool(
                            value.is_object(),
                            operator,
                            "value is not an object",
                            filter_path,
                            obj_path,
                        ),
                        _ => FilterResult::fatal_invalid_filter(
                            format!("invalid type: `{}`", type_str),
                            filter_path,
                            obj_path,
                        ),
                    },
                    _ => FilterResult::fatal_invalid_filter(
                        format!("{} arg must be a string", operator),
                        filter_path,
                        obj_path,
                    ),
                },

                // Array/String operators
                "$in" | "$contains" => match (operator_arg, value) {
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
                    // value is neither a string nor an array
                    _ => FilterResult::operator_failed(
                        operator,
                        "value not a string nor array",
                        filter_path,
                        obj_path,
                    ),
                },
                "$empty" => match (operator_arg, value) {
                    (serde_json::Value::Bool(empty), serde_json::Value::Array(value_list)) => {
                        FilterResult::from_bool(
                            *empty == value_list.is_empty(),
                            operator,
                            match value_list.is_empty() {
                                true => "value is empty",
                                false => "value is not empty",
                            },
                            filter_path,
                            obj_path,
                        )
                    }
                    (serde_json::Value::Bool(empty), serde_json::Value::Object(value_map)) => {
                        FilterResult::from_bool(
                            *empty == value_map.is_empty(),
                            operator,
                            match value_map.is_empty() {
                                true => "value is empty",
                                false => "value is not empty",
                            },
                            filter_path,
                            obj_path,
                        )
                    }
                    (serde_json::Value::Bool(empty), serde_json::Value::String(value_str)) => {
                        FilterResult::from_bool(
                            *empty == value_str.is_empty(),
                            operator,
                            match value_str.is_empty() {
                                true => "value is empty",
                                false => "value is not empty",
                            },
                            filter_path,
                            obj_path,
                        )
                    }
                    (serde_json::Value::Bool(_), _) => FilterResult::operator_failed(
                        operator,
                        "value not a string, array, or object",
                        filter_path,
                        obj_path,
                    ),
                    (_, _) => FilterResult::fatal_invalid_filter(
                        format!("{} arg must be a boolean", operator),
                        filter_path,
                        obj_path,
                    ),
                },

                // Array operators
                "$overlap" => match (operator_arg, value) {
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
                },
                "$any" => match value {
                    serde_json::Value::Array(value_list) => {
                        for (i, item) in value_list.iter().enumerate() {
                            let obj_path = &format!("{}[{}]", obj_path, i);
                            match inner_test(operator_arg, item, filter_path, obj_path) {
                                // Early return passed on first success.
                                FilterResult::Pass => return FilterResult::Pass,
                                // Ignore non-fatal errors.
                                FilterResult::Fail(_) => continue,
                                // Return fatal errors immediately.
                                FilterResult::Fatal(e) => return FilterResult::Fatal(e),
                            }
                        }
                        // Fails if no values passed the filter or there are no
                        // values.
                        FilterResult::operator_failed(
                            operator,
                            "no values passed the filter",
                            filter_path,
                            obj_path,
                        )
                    }
                    _ => FilterResult::fatal_invalid_filter(
                        format!("{} arg must be an array", operator),
                        filter_path,
                        obj_path,
                    ),
                },
                "$all" => match value {
                    serde_json::Value::Array(value_list) => {
                        for (i, item) in value_list.iter().enumerate() {
                            let obj_path = &format!("{}[{}]", obj_path, i);
                            match inner_test(operator_arg, item, filter_path, obj_path) {
                                // Continue on success.
                                FilterResult::Pass => continue,
                                // Early return the first failure.
                                failure_result => return failure_result,
                            }
                        }
                        // Passes if all values passed the filter or there are
                        // no values.
                        FilterResult::Pass
                    }
                    _ => FilterResult::fatal_invalid_filter(
                        format!("{} arg must be an array", operator),
                        filter_path,
                        obj_path,
                    ),
                },

                // String operators
                "$regex" | "$match" => match (operator_arg, value) {
                    (
                        serde_json::Value::String(regex_pattern),
                        serde_json::Value::String(value_str),
                    ) => {
                        let pattern = match regex::Regex::new(regex_pattern) {
                            Ok(pattern) => pattern,
                            Err(e) => {
                                return FilterResult::fatal_invalid_filter(
                                    format!("invalid regex: {}", e),
                                    filter_path,
                                    obj_path,
                                )
                            }
                        };

                        FilterResult::from_bool(
                            pattern.is_match(value_str),
                            operator,
                            "value does not match regex",
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
                },
                "$startsWith" => match (operator_arg, value) {
                    (serde_json::Value::String(op_arg), serde_json::Value::String(value_str)) => {
                        FilterResult::from_bool(
                            value_str.starts_with(op_arg),
                            operator,
                            "value does not start with filter value",
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
                },
                "$endsWith" => match (operator_arg, value) {
                    (serde_json::Value::String(op_arg), serde_json::Value::String(value_str)) => {
                        FilterResult::from_bool(
                            value_str.ends_with(op_arg),
                            operator,
                            "value does not end with filter value",
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
                },

                // Value transformers
                "#len" | "#size" => match value.as_array().map_or_else(
                    || value.as_str().map(|value_str| value_str.len()),
                    |value_array| Some(value_array.len()),
                ) {
                    Some(len) => inner_test(
                        operator_arg,
                        &serde_json::Value::Number(len.into()),
                        filter_path,
                        obj_path,
                    ),
                    _ => FilterResult::operator_failed(
                        operator,
                        "value is not a string nor array",
                        filter_path,
                        obj_path,
                    ),
                },
                "#lower" => match value {
                    serde_json::Value::String(value_str) => inner_test(
                        operator_arg,
                        &serde_json::Value::String(value_str.to_lowercase()),
                        filter_path,
                        obj_path,
                    ),
                    _ => FilterResult::operator_failed(
                        operator,
                        "value is not a string",
                        filter_path,
                        obj_path,
                    ),
                },
                "#upper" => match value {
                    serde_json::Value::String(value_str) => inner_test(
                        operator_arg,
                        &serde_json::Value::String(value_str.to_uppercase()),
                        filter_path,
                        obj_path,
                    ),
                    _ => FilterResult::operator_failed(
                        operator,
                        "value is not a string",
                        filter_path,
                        obj_path,
                    ),
                },
                "#keys" => match value {
                    serde_json::Value::Object(value_obj) => inner_test(
                        operator_arg,
                        &serde_json::Value::Array(
                            value_obj
                                .keys()
                                .map(|k| serde_json::Value::String(k.clone()))
                                .collect(),
                        ),
                        filter_path,
                        obj_path,
                    ),
                    _ => FilterResult::operator_failed(
                        operator,
                        "value is not an object",
                        filter_path,
                        obj_path,
                    ),
                },
                "#values" => match value {
                    serde_json::Value::Object(value_obj) => inner_test(
                        operator_arg,
                        &serde_json::Value::Array(value_obj.values().cloned().collect()),
                        filter_path,
                        obj_path,
                    ),
                    _ => FilterResult::operator_failed(
                        operator,
                        "value is not an object",
                        filter_path,
                        obj_path,
                    ),
                },
                "#base64" => match value {
                    serde_json::Value::String(value_str) => {
                        // Decode the base64 string.
                        let decoded_value = match BASE64_ENGINE.decode(value_str) {
                            Ok(decoded_value) => decoded_value,
                            Err(e) => {
                                return FilterResult::operator_failed(
                                    operator,
                                    format!("failed to decode base64: {}", e),
                                    filter_path,
                                    obj_path,
                                )
                            }
                        };

                        // Convert the decoded value to a string.
                        let decoded_value_str = match String::from_utf8(decoded_value) {
                            Ok(str) => str,
                            Err(e) => {
                                return FilterResult::operator_failed(
                                    operator,
                                    format!("failed to convert base64 to string: {}", e),
                                    filter_path,
                                    obj_path,
                                )
                            }
                        };

                        // Attempt to parse the decoded value as a JSON value.
                        // If this fails, assume the value is supposed to be a
                        // string.
                        let decoded_value_json = match serde_json::from_str(&decoded_value_str) {
                            Ok(json) => json,
                            Err(_) => serde_json::Value::String(decoded_value_str),
                        };

                        inner_test(operator_arg, &decoded_value_json, filter_path, obj_path)
                    }
                    _ => FilterResult::operator_failed(
                        operator,
                        "value is not a string",
                        filter_path,
                        obj_path,
                    ),
                },

                _ => FilterResult::fatal_unknown_operator(operator, filter_path, obj_path),
            }
        }
    }
}
