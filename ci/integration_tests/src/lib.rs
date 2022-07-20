#![feature(test)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_harness::test_runner::cosm_orc_test_runner)]
// NOTE: the custom_test_frameworks causes clippy to incorrectly reports unused code
#![allow(dead_code)]

#[cfg(test)]
mod tests;

mod test_harness;

mod helpers;
