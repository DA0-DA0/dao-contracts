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
            "$ne" | "$neq" => Operator::Neq,
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

            // Transforms
            "#len" | "#size" => Operator::Len,
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

pub struct OperatorContext<'a> {
    pub operator: &'a str,
    pub operator_arg: &'a serde_json::Value,
    pub value: Option<&'a serde_json::Value>,
    pub filter_path: &'a str,
    pub obj_path: &'a str,
}
