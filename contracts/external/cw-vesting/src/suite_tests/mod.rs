mod suite;
mod tests;

// Advantage to using a macro for this is that the error trace links
// to the exact line that the error occured, instead of inside of a
// function where the assertion would otherwise happen.
macro_rules! is_error {
    ($x:expr, $e:expr) => {
        assert!(format!("{:#}", $x.unwrap_err()).contains($e))
    };
}
pub(crate) use is_error;
