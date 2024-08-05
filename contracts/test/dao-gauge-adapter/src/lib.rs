
//! Gauge adapter contract to mock in tests.
//! I wrote it so that InstantiateMsg contains list of initially
//! available options. Query for CheckOption checks if option is already added,
//! otherwise returns true - option is valid.

pub mod contract;