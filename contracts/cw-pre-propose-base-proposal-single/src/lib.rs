pub mod contract;

pub use contract::{ExecuteMsg, InstantiateMsg, ProposeMessage, QueryMsg};

// Exporting this means that contracts interacting with this one don't
// need an explicit dependency on the base contract to read config
// queries.
pub use cw_pre_propose_base::state::Config;
