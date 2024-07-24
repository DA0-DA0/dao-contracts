#[allow(dead_code)]
fn main() {}

mod dao;
mod external;
mod gauges;
pub use dao::*;
pub use external::*;
pub use gauges::*;

#[cfg(test)]
mod tests;
