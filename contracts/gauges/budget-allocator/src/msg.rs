use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Coin;

// Re-export the orchestrator-facing query enum and its response types so
// downstream consumers can depend on either adapter through one type
// surface. Crates that only need the orchestrator-facing surface can
// `use gauge_adapter::msg::AdapterQueryMsg`.
pub use gauge_adapter::msg::{
    AdapterQueryMsg, AllOptionsResponse, CheckOptionResponse, SampleGaugeMsgsResponse,
};

#[cw_serde]
pub struct InstantiateMsg {
    /// Address allowed to update the option list. Typically the DAO's core
    /// module.
    pub admin: String,
    /// Initial set of valid options.
    pub options: Vec<String>,
    /// Per-epoch budget distributed proportional to weights.
    pub epoch_budget: Coin,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Admin-only: add a new option to the valid set.
    AddOption { option: String },
    /// Admin-only: remove an option from the valid set.
    RemoveOption { option: String },
    /// Admin-only: replace the per-epoch budget.
    UpdateBudget { epoch_budget: Coin },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Inspect the stored config (admin + current budget).
    #[returns(crate::state::Config)]
    Config {},
    /// All currently-valid options (proxy for `AdapterQueryMsg::AllOptions`).
    #[returns(AllOptionsResponse)]
    AllOptions {},
    /// Check whether `option` is in the valid set.
    #[returns(CheckOptionResponse)]
    CheckOption { option: String },
    /// Translate a selected set into `BankMsg::Send` payouts.
    #[returns(SampleGaugeMsgsResponse)]
    SampleGaugeMsgs {
        /// Option + weight pairs, weights summing to ≤ 1.0.
        selected: Vec<(String, cosmwasm_std::Decimal)>,
    },
}

#[cw_serde]
pub enum MigrateMsg {}
