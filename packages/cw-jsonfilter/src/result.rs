use std::fmt::Display;

/// The result of a filter/operator test. The distinction between fatal and
/// non-fatal errors is important because fatal errors should halt processing
/// immediately, whereas non-fatal errors indicate that the value simply does
/// not pass the filter. This is especially important when applying operators
/// that respond to the output of other operators, such as the logical operators
/// `$and`, `$or`, `$xor`, and `$not`—fatal errors should be returned
/// immediately instead of being considered typical failures in the logic chain.
/// Non-fatal errors should just be treated as a test failure.
///
/// To put it more explicitly: a fatal error should only occur when a filter is
/// malformed—it should not be dependent on the value passed in at all. Only
/// non-fatal errors should be dependent on the value passed in.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterResult {
    /// The filter passed.
    Pass,
    /// The filter failed due to a value not passing the filter.
    Fail(FilterFailure),
    /// The filter encountered a fatal error due to a malformed filter, not
    /// dependent on the value passed in whatsoever.
    Fatal(FilterFatalError),
}

#[derive(Debug, Clone, PartialEq)]
/// Represents errors that indicate that the filter failed.
pub enum FilterFailure {
    /// Indicates that a key was not found in the object being filtered.
    KeyNotFound {
        filter_path: String,
        obj_path: String,
    },
    /// The value did not pass the operator test.
    OperatorFailed {
        operator: String,
        reason: String,
        filter_path: String,
        obj_path: String,
    },
}

impl Display for FilterFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FilterFailure::KeyNotFound {
                filter_path,
                obj_path,
            } => {
                write!(
                    f,
                    "Key not found at object path: `{obj_path}` for filter path: `{filter_path}`"
                )
            }
            FilterFailure::OperatorFailed {
                operator,
                reason,
                filter_path,
                obj_path,
            } => write!(
                f,
                "Operator failed: `{operator}` at filter path: `{filter_path}` and object path: `{obj_path}` with reason: `{reason}`"
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Represents errors that are fatal and prevent a filter/operator from running.
pub enum FilterFatalError {
    /// Indicates that the schema of the filter is invalid.
    InvalidFilter {
        reason: String,
        filter_path: String,
        obj_path: String,
    },
    /// Indicates that an unknown operator was encountered in the filter.
    UnknownOperator {
        operator: String,
        filter_path: String,
        obj_path: String,
    },
}

impl Display for FilterFatalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FilterFatalError::InvalidFilter {
                reason,
                filter_path,
                obj_path,
            } => write!(
                f,
                "Invalid filter: `{reason}` at filter path: `{filter_path}` and object path: `{obj_path}`"
            ),
            FilterFatalError::UnknownOperator {
                operator,
                filter_path,
                obj_path,
            } => write!(
                f,
                "Unknown operator: `{operator}` at filter path: `{filter_path}` with object path: `{obj_path}`"
            ),
        }
    }
}

impl FilterResult {
    /// Creates a fatal `FilterResult` with a new
    /// `FilterFatalError::InvalidFilter` error.
    pub fn fatal_invalid_filter(
        reason: impl Into<String>,
        filter_path: &str,
        obj_path: &str,
    ) -> Self {
        Self::Fatal(FilterFatalError::InvalidFilter {
            reason: reason.into(),
            filter_path: filter_path.to_string(),
            obj_path: obj_path.to_string(),
        })
    }

    /// Creates a fatal `FilterResult` with a new
    /// `FilterFatalError::UnknownOperator` error.
    pub fn fatal_unknown_operator(
        operator: impl Into<String>,
        filter_path: impl Into<String>,
        obj_path: impl Into<String>,
    ) -> Self {
        Self::Fatal(FilterFatalError::UnknownOperator {
            operator: operator.into(),
            filter_path: filter_path.into(),
            obj_path: obj_path.into(),
        })
    }

    /// Creates a failed `FilterResult` with a new `FilterFailure::KeyNotFound`
    /// error.
    pub fn key_not_found(filter_path: impl Into<String>, obj_path: impl Into<String>) -> Self {
        Self::Fail(FilterFailure::KeyNotFound {
            filter_path: filter_path.into(),
            obj_path: obj_path.into(),
        })
    }

    /// Creates a failed `FilterResult` with a new
    /// `FilterFailure::OperatorFailed` error.
    pub fn operator_failed(
        operator: impl Into<String>,
        reason: impl Into<String>,
        filter_path: impl Into<String>,
        obj_path: impl Into<String>,
    ) -> Self {
        Self::Fail(FilterFailure::OperatorFailed {
            operator: operator.into(),
            reason: reason.into(),
            filter_path: filter_path.into(),
            obj_path: obj_path.into(),
        })
    }

    /// Creates a passed or failed `FilterResult` based on the boolean value.
    pub fn from_bool(
        pass: bool,
        operator: impl Into<String>,
        reason: impl Into<String>,
        filter_path: impl Into<String>,
        obj_path: impl Into<String>,
    ) -> Self {
        if pass {
            Self::Pass
        } else {
            Self::operator_failed(operator, reason, filter_path, obj_path)
        }
    }

    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Pass)
    }

    pub fn is_fail(&self) -> bool {
        matches!(self, Self::Fail(_))
    }

    pub fn is_fatal(&self) -> bool {
        matches!(self, Self::Fatal(_))
    }

    pub fn as_fail(&self) -> Option<&FilterFailure> {
        match self {
            Self::Fail(failure) => Some(failure),
            _ => None,
        }
    }

    pub fn as_fatal(&self) -> Option<&FilterFatalError> {
        match self {
            Self::Fatal(fatal) => Some(fatal),
            _ => None,
        }
    }
}

impl Display for FilterResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pass => write!(f, "Pass"),
            Self::Fail(failure) => write!(f, "Fail: {failure}"),
            Self::Fatal(fatal) => write!(f, "Fatal: {fatal}"),
        }
    }
}
