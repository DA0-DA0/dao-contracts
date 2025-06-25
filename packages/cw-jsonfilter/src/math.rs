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
    if a.is_u64() && b.is_u64() {
        let a = a.as_u64().unwrap();
        let b = b.as_u64().unwrap();
        return Some(a < b);
    } else if a.is_i64() && b.is_i64() {
        let a = a.as_i64().unwrap();
        let b = b.as_i64().unwrap();
        return Some(a < b);
    } else if a.is_string() && b.is_string() {
        let a = a.as_str().unwrap();
        let b = b.as_str().unwrap();
        return Some(a < b);
    }

    // Try to convert to float last. `is_f64` ignores integers, even if they
    // would be valid floats, so to compare decimals we just need to convert.
    let a = a.as_f64();
    let b = b.as_f64();
    if let (Some(a), Some(b)) = (a, b) {
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
pub fn gt_json(a: &serde_json::Value, b: &serde_json::Value) -> Option<bool> {
    if a.is_u64() && b.is_u64() {
        let a = a.as_u64().unwrap();
        let b = b.as_u64().unwrap();
        return Some(a > b);
    } else if a.is_i64() && b.is_i64() {
        let a = a.as_i64().unwrap();
        let b = b.as_i64().unwrap();
        return Some(a > b);
    } else if a.is_string() && b.is_string() {
        let a = a.as_str().unwrap();
        let b = b.as_str().unwrap();
        return Some(a > b);
    }

    // Try to convert to float last. `is_f64` ignores integers, even if they
    // would be valid floats, so to compare decimals we just need to convert.
    let a = a.as_f64();
    let b = b.as_f64();
    if let (Some(a), Some(b)) = (a, b) {
        Some(a > b)
    } else {
        None
    }
}
