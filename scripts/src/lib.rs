#[allow(dead_code)]
fn main() {}

mod deploy;
mod dao;
mod external;
mod gauges;
mod distribution;
mod propose;
mod staking;
mod voting;

pub use dao::*;
pub use external::*;
pub use distribution::*;
pub use propose::*;
pub use staking::*;
pub use voting::*;
pub use gauges::*;

#[cfg(test)]
mod tests;
