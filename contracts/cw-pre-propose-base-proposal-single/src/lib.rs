pub mod contract;

pub use contract::{ExecuteMsg, InstantiateMsg, ProposeMessage, QueryMsg};

// Exporting these means that contracts interacting with this one don't
// need an explicit dependency on the base contract to read queries.
pub use cw_pre_propose_base::msg::DepositInfoResponse;
pub use cw_pre_propose_base::state::Config;
