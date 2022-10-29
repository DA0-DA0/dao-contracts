pub mod contract;
pub mod msg;
pub mod state;

#[cfg(test)]
mod tests;

// Exporting these means that contracts interacting with this one don't
// need an explicit dependency on the base contract to read queries.
pub use cwd_pre_propose_base::msg::DepositInfoResponse;
pub use cwd_pre_propose_base::state::Config;
