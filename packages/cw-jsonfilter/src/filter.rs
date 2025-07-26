use crate::{
    decoder::{NoopDecoder, ProtobufDecoder},
    json_ops::operator::Operator,
    FilterResult,
};

pub struct CwJsonFilter<D> {
    /// optional ProtobufDecoder trait object to decode
    /// a protobuf messages into JSON.
    pub decode_protobuf: Option<D>,
}

impl Default for CwJsonFilter<NoopDecoder> {
    fn default() -> Self {
        Self::new(None)
    }
}

impl CwJsonFilter<NoopDecoder> {
    /// Static convenience function for the default filter with no protobuf
    /// decoder.
    pub fn check(filter: &serde_json::Value, obj: &serde_json::Value) -> FilterResult {
        let cwjf = CwJsonFilter::default();
        cwjf.matches(filter, obj)
    }
}

impl<D: ProtobufDecoder> CwJsonFilter<D> {
    /// Create a new filter, optionally providing a protobuf decoder to use with
    /// the #proto/#stargate transformer. If not provided, the filter will not
    /// be able to use protobuf transformers.
    pub fn new(decode_protobuf: Option<D>) -> Self {
        Self { decode_protobuf }
    }

    /// Matches an object against a filter and returns whether the object passes
    /// the filter.
    ///
    /// # Arguments
    ///
    /// * `filter` - A reference to a `serde_json::Value` representing the
    ///   filter to apply. Must be an object.
    /// * `obj` - A reference to a `serde_json::Value` representing the object
    ///   to match against. Must be an object.
    ///
    /// # Returns
    ///
    /// * `FilterResult::Pass` if the filter matches the object.
    /// * `FilterResult::Fail(FilterError)` if the filter does not match the
    ///   object.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_json::json;
    /// use cw_jsonfilter::CwJsonFilter;
    ///
    /// let filter = json!({"name": "John", "age": 30});
    /// let obj = json!({"name": "John", "age": 30, "city": "New York"});
    ///
    /// assert!(CwJsonFilter::check(&filter, &obj).is_pass());
    /// ```
    #[must_use]
    pub fn matches(&self, filter: &serde_json::Value, obj: &serde_json::Value) -> FilterResult {
        self.inner_matches(filter, Some(obj), "@", "@")
    }

    pub fn inner_matches(
        &self,
        filter: &serde_json::Value,
        obj: Option<&serde_json::Value>,
        filter_path: &str,
        obj_path: &str,
    ) -> FilterResult {
        // If the filter is an empty object and the object is not present, the
        // parent key is not found. This base case is required to be handled
        // here since the existence operator (and logical operators) can
        // operator on the absence of an object, meaning there is a condition
        // where a nonexistent object matches the filter. The operator matcher
        // checks existence of the object when necessary, and so do the other
        // types of filters below (object key/value, array, and primitive). Thus
        // this is the only case that needs to be specifically handled.
        if filter.as_object().map(|o| o.is_empty()).unwrap_or_default() && obj.is_none() {
            return FilterResult::key_not_found(filter_path, obj_path);
        }

        match filter {
            // Objects process each key present in the filter, behaving like an
            // $and operation for all keys.
            serde_json::Value::Object(filter_obj) => {
                for (filter_key, filter_val) in filter_obj {
                    let is_operator = filter_key.starts_with('$');
                    let is_transformer = filter_key.starts_with('#');
                    // The object doesn't have to exist at this point, since the
                    // existence operator (and logical operators) can operate on
                    // the absence of an object.
                    if is_operator || is_transformer {
                        let filter_path = &append_path(filter_path, filter_key);
                        match self.inner_matches_operator(
                            filter_key,
                            filter_val,
                            obj,
                            filter_path,
                            obj_path,
                        ) {
                            // If success, continue to next key.
                            FilterResult::Pass => continue,
                            // If failure, return the error.
                            failure_result => return failure_result,
                        }
                    } else {
                        // The object must be present at this point since we
                        // need to dig deeper into the object.
                        let obj = match obj {
                            Some(o) => o,
                            None => return FilterResult::key_not_found(filter_path, obj_path),
                        };

                        // If the key is a stringified number and the object is
                        // an array, index into the array.
                        match (obj.as_array(), filter_key.parse::<usize>()) {
                            (Some(obj_array), Ok(index)) => {
                                let filter_path = &append_array_path(filter_path, index);
                                let obj_path = &append_array_path(obj_path, index);
                                let value = obj_array.get(index);
                                match self.inner_matches(filter_val, value, filter_path, obj_path) {
                                    // If success, continue to next key.
                                    FilterResult::Pass => continue,
                                    // If failure, return the error.
                                    failure_result => return failure_result,
                                }
                            }
                            _ => {
                                let filter_path = &append_path(filter_path, filter_key);
                                let obj_path = &append_path(obj_path, filter_key);
                                let value = obj.get(filter_key);
                                match self.inner_matches(filter_val, value, filter_path, obj_path) {
                                    // If success, continue to next key.
                                    FilterResult::Pass => continue,
                                    // If failure, return the error.
                                    failure_result => return failure_result,
                                }
                            }
                        }
                    }
                }

                // Passes if all keys matched or there are no keys.
                FilterResult::Pass
            }
            // Arrays implicitly match each item in order, behaving like an $and
            // operation for all items, while also matching the array as a
            // whole (exact same number of items).
            serde_json::Value::Array(filter_list) => match obj {
                Some(serde_json::Value::Array(obj_list)) => {
                    let filter_len = filter_list.len();
                    let obj_len = obj_list.len();
                    if filter_len != obj_len {
                        return FilterResult::operator_failed(
                            "[...]",
                            format!(
                                "value array length ({}) != filter array length ({})",
                                obj_len, filter_len
                            ),
                            filter_path,
                            obj_path,
                        );
                    }

                    for (i, sub_filter) in filter_list.iter().enumerate() {
                        let filter_path = &append_array_path(filter_path, i);
                        let obj_path = &append_array_path(obj_path, i);
                        match self.inner_matches(sub_filter, obj_list.get(i), filter_path, obj_path)
                        {
                            // If success, continue to next item.
                            FilterResult::Pass => continue,
                            // If failure, return the error.
                            failure_result => return failure_result,
                        };
                    }

                    // Passes if all filters matched or there are no filters.
                    FilterResult::Pass
                }
                Some(_) => FilterResult::operator_failed(
                    "[...]",
                    "value is not an array",
                    filter_path,
                    obj_path,
                ),
                None => FilterResult::key_not_found(filter_path, obj_path),
            },
            // Match primitive values directly.
            _ => match obj.map(|o| o == filter) {
                Some(true) => FilterResult::Pass,
                Some(false) => FilterResult::operator_failed(
                    "implicit equality check",
                    "value does not match filter",
                    filter_path,
                    obj_path,
                ),
                None => FilterResult::key_not_found(filter_path, obj_path),
            },
        }
    }

    /// Matches a filter operator against a value from the object and determines
    /// if the condition is met.
    ///
    /// # Arguments
    ///
    /// * `operator` - The operator to apply to the value.
    /// * `operator_arg` - The argument associated with the operator.
    /// * `value` - The value to apply the operator filter to. If None, the
    ///   value does not exist in the object at the path specified by
    ///   `obj_path`.
    /// * `filter_path` - The path to the filter operator being applied.
    /// * `obj_path` - The path to the value from the object bei.
    ///
    /// # Returns
    ///
    /// * `Ok(true)` if the condition specified by the filter operator is met.
    /// * `Ok(false)` if the condition specified by the filter operator is not
    ///   met.
    /// * `Err(FilterError)` if there is an error in the filtering process.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// * An unknown operator is used.
    /// * The operator argument or value is not the expected type based on
    ///   context.
    /// * The filter format is invalid.
    /// * The value is not found in the object when it is needed.
    fn inner_matches_operator(
        &self,
        operator: &str,
        operator_arg: &serde_json::Value,
        value: Option<&serde_json::Value>,
        filter_path: &str,
        obj_path: &str,
    ) -> FilterResult {
        let Some(op_kind) = Operator::from_name(operator) else {
            return FilterResult::fatal_unknown_operator(operator, filter_path, obj_path);
        };
        match op_kind {
            // Existence operator
            Operator::Exists => {
                self.handle_exists_op(operator, operator_arg, value, filter_path, obj_path)
            }

            // Logical operators
            Operator::And => {
                self.handle_and_op(operator, operator_arg, value, filter_path, obj_path)
            }
            Operator::Or => self.handle_or_op(operator, operator_arg, value, filter_path, obj_path),
            Operator::Xor => {
                self.handle_xor_op(operator, operator_arg, value, filter_path, obj_path)
            }
            Operator::Not => {
                self.handle_not_op(operator, operator_arg, value, filter_path, obj_path)
            }

            // Comparison operators
            Operator::Eq => self.handle_eq_op(operator, operator_arg, value, filter_path, obj_path),
            Operator::Ne | Operator::Neq => {
                self.handle_neq_op(operator, operator_arg, value, filter_path, obj_path)
            }
            Operator::Lt | Operator::Lte => {
                self.handle_lt_check_op(operator, operator_arg, value, filter_path, obj_path)
            }
            Operator::Gt | Operator::Gte => {
                self.handle_gt_check_op(operator, operator_arg, value, filter_path, obj_path)
            }
            Operator::Range
            | Operator::RangeExclusive
            | Operator::Between
            | Operator::BetweenExclusive => {
                self.handle_range_op(operator, operator_arg, value, filter_path, obj_path)
            }

            // type operator
            Operator::Type => {
                self.handle_type_op(operator, operator_arg, value, filter_path, obj_path)
            }

            // Array/String operators
            Operator::Contains => {
                self.handle_contains_op(operator, operator_arg, value, filter_path, obj_path)
            }
            Operator::Overlap => {
                self.handle_overlaps_op(operator, operator_arg, value, filter_path, obj_path)
            }
            Operator::Any => {
                self.handle_any_op(operator, operator_arg, value, filter_path, obj_path)
            }
            Operator::All => {
                self.handle_all_op(operator, operator_arg, value, filter_path, obj_path)
            }
            Operator::StartsWith | Operator::EndsWith => self.handle_starts_ends_with_op(
                operator,
                operator_arg,
                value,
                filter_path,
                obj_path,
            ),

            // Value transformers
            Operator::Len | Operator::Size => {
                self.handle_size_op(operator, operator_arg, value, filter_path, obj_path)
            }
            Operator::ToString => {
                self.handle_to_string_op(operator, operator_arg, value, filter_path, obj_path)
            }
            Operator::ToNumber => {
                self.handle_to_number_op(operator, operator_arg, value, filter_path, obj_path)
            }
            Operator::Lower => {
                self.handle_to_lower_op(operator, operator_arg, value, filter_path, obj_path)
            }
            Operator::Upper => {
                self.handle_to_upper_op(operator, operator_arg, value, filter_path, obj_path)
            }
            Operator::Keys => {
                self.handle_to_keys_op(operator, operator_arg, value, filter_path, obj_path)
            }
            Operator::Values => {
                self.handle_to_values_op(operator, operator_arg, value, filter_path, obj_path)
            }
            Operator::Replace => {
                self.handle_replace_op(operator, operator_arg, value, filter_path, obj_path)
            }
            Operator::Base64 => {
                self.handle_base64_op(operator, operator_arg, value, filter_path, obj_path)
            }
            Operator::Proto => {
                self.handle_proto_op(operator, operator_arg, value, filter_path, obj_path)
            }
            Operator::Stargate => {
                self.handle_stargate_op(operator, operator_arg, value, filter_path, obj_path)
            }
        }
    }
}

// Helper to reduce path allocations
#[inline]
pub fn append_path(base: &str, segment: &str) -> String {
    let mut path = String::with_capacity(base.len() + segment.len() + 1);
    path.push_str(base);
    path.push('.');
    path.push_str(segment);
    path
}

#[inline]
pub fn append_array_path(base: &str, index: usize) -> String {
    let mut path = String::with_capacity(base.len() + 10); // reasonable for most indices
    path.push_str(base);
    path.push('[');
    path.push_str(&index.to_string());
    path.push(']');
    path
}
