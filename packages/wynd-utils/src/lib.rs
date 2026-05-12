mod curve;
mod scalable_curve;

pub use curve::{Curve, CurveError, PiecewiseLinear, SaturatingLinear};
pub use scalable_curve::{ScalableCurve, ScalableLinear, ScalablePiecewise};
