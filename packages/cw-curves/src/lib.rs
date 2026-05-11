pub mod curve;
pub mod curves;
pub mod utils;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod diff_tests;

pub use curve::{Curve, CurveError, DecimalPlaces};
