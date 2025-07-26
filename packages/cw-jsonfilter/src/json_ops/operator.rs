/// JSON Operator enum that declares the supported operations
pub enum Operator {
    // existence
    Exists,
    // logical
    And,
    Or,
    Xor,
    Not,
    // value comparison
    Eq,
    Ne,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    Range,
    RangeExclusive,
    Between,
    BetweenExclusive,
    // type
    Type,
    // array / string
    Contains,
    Overlap,
    Any,
    All,
    StartsWith,
    EndsWith,
    // transformers
    Len,
    Size,
    ToString,
    ToNumber,
    Lower,
    Upper,
    Keys,
    Values,
    Replace,
    Base64,
    Proto,
    Stargate,
}

impl Operator {
    pub fn from_name(s: &str) -> Option<Self> {
        let operator = match s {
            // Logical
            "$and" => Operator::And,
            "$or" => Operator::Or,
            "$xor" => Operator::Xor,
            "$not" => Operator::Not,

            // Existence
            "$exists" => Operator::Exists,

            // Comparison
            "$eq" => Operator::Eq,
            "$ne" => Operator::Ne,
            "$neq" => Operator::Neq,
            "$lt" => Operator::Lt,
            "$lte" => Operator::Lte,
            "$gt" => Operator::Gt,
            "$gte" => Operator::Gte,
            "$range" => Operator::Range,
            "$range_exclusive" => Operator::RangeExclusive,
            "$between" => Operator::Between,
            "$between_exclusive" => Operator::BetweenExclusive,

            // Type
            "$type" => Operator::Type,

            // Collections / Strings
            "$contains" => Operator::Contains,
            "$overlap" => Operator::Overlap,
            "$any" => Operator::Any,
            "$all" => Operator::All,
            "$startsWith" => Operator::StartsWith,
            "$endsWith" => Operator::EndsWith,

            // Transformers
            "#len" => Operator::Len,
            "#size" => Operator::Size,
            "#to_string" => Operator::ToString,
            "#to_number" => Operator::ToNumber,
            "#lower" => Operator::Lower,
            "#upper" => Operator::Upper,
            "#keys" => Operator::Keys,
            "#values" => Operator::Values,
            "#replace" => Operator::Replace,
            "#base64" => Operator::Base64,
            "#proto" => Operator::Proto,
            "#stargate" => Operator::Stargate,

            // failed to match a known operator, return None
            _ => return None,
        };

        Some(operator)
    }
}

/// Helper type used for wrapping the fields relevant for inner
/// operator matching functionality.
pub struct OperatorContext<'a> {
    /// The operator to apply to the value.
    pub operator: &'a str,
    /// The argument associated with the operator.
    pub operator_arg: &'a serde_json::Value,
    /// The value to apply the operator filter to. If None, the
    /// value does not exist in the object at the path specified by
    /// `obj_path`.
    pub value: Option<&'a serde_json::Value>,
    /// The path to the filter operator being applied.
    pub filter_path: &'a str,
    /// The path to the value from the object bei.
    pub obj_path: &'a str,
}
