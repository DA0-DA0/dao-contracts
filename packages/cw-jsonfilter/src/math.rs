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
pub fn lt_json(a: &serde_json::Value, b: &serde_json::Value) -> Option<bool> {
    if let (Some(a), Some(b)) = (a.as_u64(), b.as_u64()) {
        Some(a < b)
    } else if let (Some(a), Some(b)) = (a.as_i64(), b.as_i64()) {
        Some(a < b)
    } else if let (Some(a), Some(b)) = (a.as_str(), b.as_str()) {
        Some(a < b)
    } else if a.is_f64() && b.is_f64() {
        // explicit `is_f64()` checks for both values because `as_f64()`
        // silently casts integers (signed & unsigned) into `f64` which
        // would allow for conversion between different numeric types.
        if let (Some(a), Some(b)) = (a.as_f64(), b.as_f64()) {
            Some(a < b)
        } else {
            None
        }
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
pub fn gt_json(a: &serde_json::Value, b: &serde_json::Value) -> Option<bool> {
    if let (Some(a), Some(b)) = (a.as_u64(), b.as_u64()) {
        Some(a > b)
    } else if let (Some(a), Some(b)) = (a.as_i64(), b.as_i64()) {
        Some(a > b)
    } else if let (Some(a), Some(b)) = (a.as_str(), b.as_str()) {
        Some(a > b)
    } else if a.is_f64() && b.is_f64() {
        // explicit `is_f64()` checks for both values because `as_f64()`
        // silently casts integers (signed & unsigned) into `f64` which
        // would allow for conversion between different numeric types.
        if let (Some(a), Some(b)) = (a.as_f64(), b.as_f64()) {
            Some(a > b)
        } else {
            None
        }
    } else {
        None
    }
}

mod tests {
    use serde_json::json;

    #[test]
    fn lt_json_f64_f64_happy() {
        let filter_val: f64 = 86.0;
        let object_val: f64 = 85.0;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$lt": filter_val } }),
            &json!({ "number": object_val })
        );

        assert!(check_result.is_pass());
    }

    #[test]
    fn lt_json_i64_i64_happy() {
        let filter_val: i64 = -20;
        let object_val: i64 = -21;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$lt": filter_val } }),
            &json!({ "number": object_val })
        );

        assert!(check_result.is_pass());
    }

    #[test]
    fn lt_json_u64_u64_happy() {
        let filter_val: u64 = 21;
        let object_val: u64 = 20;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$lt": filter_val } }),
            &json!({ "number": object_val })
        );

        assert!(check_result.is_pass());
    }



    #[test]
    fn lt_json_f64_u64_err() {
        let filter_val: f64 = 85.0;
        let object_val: u64 = 86;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$lt": filter_val } }),
            &json!({ "number": object_val })
        );

        assert!(check_result.is_fail());
    }

    #[test]
    fn lt_json_u64_f64_err() {
        let filter_val: u64 = 85;
        let object_val: f64 = 86.0;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$lt": filter_val } }),
            &json!({ "number": object_val })
        );

        assert!(check_result.is_fail());
    }

    #[test]
    fn lt_json_i64_f64_err() {
        let filter_val: i64 = 1;
        let object_val: f64 = 2.0;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$lt": filter_val } }),
            &json!({ "number": object_val })
        );

        assert!(check_result.is_fail());
    }

    #[test]
    fn lt_json_f64_i64_err() {
        let filter_val: f64 = 1.0;
        let object_val: i64 = 2;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$lt": filter_val } }),
            &json!({ "number": object_val })
        );

        assert!(check_result.is_fail());
    }

    #[test]
    fn gt_json_f64_f64_happy() {
        let filter_val: f64 = 85.0;
        let object_val: f64 = 86.0;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$gt": filter_val } }),
            &json!({ "number": object_val })
        );

        assert!(check_result.is_pass());
    }

    #[test]
    fn gt_json_i64_i64_happy() {
        let filter_val: i64 = -21;
        let object_val: i64 = -20;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$gt": filter_val } }),
            &json!({ "number": object_val })
        );

        assert!(check_result.is_pass());
    }

    #[test]
    fn gt_json_u64_u64_happy() {
        let filter_val: u64 = 20;
        let object_val: u64 = 21;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$gt": filter_val } }),
            &json!({ "number": object_val })
        );

        assert!(check_result.is_pass());
    }



    #[test]
    fn gt_json_f64_u64_err() {
        let filter_val: f64 = 85.0;
        let object_val: u64 = 86;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$gt": filter_val } }),
            &json!({ "number": object_val })
        );

        assert!(check_result.is_fail());
    }

    #[test]
    fn gt_json_u64_f64_err() {
        let filter_val: u64 = 85;
        let object_val: f64 = 86.0;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$gt": filter_val } }),
            &json!({ "number": object_val })
        );

        assert!(check_result.is_fail());
    }

    #[test]
    fn gt_json_i64_f64_err() {
        let filter_val: i64 = 2;
        let object_val: f64 = 1.0;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$gt": filter_val } }),
            &json!({ "number": object_val })
        );

        assert!(check_result.is_fail());
    }

    #[test]
    fn gt_json_f64_i64_err() {
        let filter_val: i64 = 2;
        let object_val: f64 = 1.0;

        let check_result = crate::CwJsonFilter::check(
            &json!({ "number": { "$gt": filter_val } }),
            &json!({ "number": object_val })
        );

        assert!(check_result.is_fail());
    }
}
