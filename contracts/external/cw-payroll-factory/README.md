# cw-payroll-factory

[![cw-payroll-factory on crates.io](https://img.shields.io/crates/v/cw-payroll-factory.svg?logo=rust)](https://crates.io/crates/cw-payroll-factory)
[![docs.rs](https://img.shields.io/docsrs/cw-payroll-factory?logo=docsdotrs)](https://docs.rs/cw-payroll-factory/latest/cw_payroll_factory/)

Serves as a factory that instantiates [cw-vesting](../cw-vesting) contracts and stores them in an indexed maps for easy querying by recipient or the instantiator (i.e. give me all of my vesting payment contracts or give me all of a DAO's vesting payment contracts).

An optional `owner` can be specified when instantiating `cw-payroll-factory` that limits contract instantiation to a single account.
