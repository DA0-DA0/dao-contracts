#![feature(test)]
#![feature(custom_test_frameworks)]
#![test_runner(cosm_orc_test_runner)]

#[cfg(test)]
mod tests;

mod test_harness;

mod helpers;

use crate::test_harness::test_runner::cosm_orc_test_runner;
