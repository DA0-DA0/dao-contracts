use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use cw20_hooks::hooks::Cw20HookMsg;
use cw_ownable::cw_ownable_execute;

use crate::state::{Allowance, Config};

#[cw_serde]
pub struct AllowanceUpdate {
    /// The address to set the allowance for.
    pub address: String,
    /// The allowance to set.
    pub allowance: Allowance,
}

#[cw_serde]
pub struct InstantiateMsg {
    /// The address that can update the config and allowances.
    pub owner: Option<String>,
    /// An initial list of allowances allowances.
    pub allowances: Option<Vec<AllowanceUpdate>>,
    /// The DAO whose members may be able to send and/or receive tokens.
    pub dao: String,
    /// The allowance assigned to DAO members with no explicit allowance set. If
    /// None, members with no allowance set cannot send nor receive tokens.
    pub member_allowance: Option<Allowance>,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    Cw20Hook(Cw20HookMsg),
    UpdateAllowances {
        /// Addresses to add/update allowances for.
        set: Vec<AllowanceUpdate>,
        /// Addresses to remove allowances from.
        remove: Vec<String>,
    },
    UpdateConfig {
        /// The DAO whose members may be able to send or receive tokens.
        dao: Option<String>,
        /// The allowance assigned to DAO members with no explicit allowance
        /// set. If None, members with no allowance set cannot send nor receive
        /// tokens.
        member_allowance: Option<Allowance>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the paginated list of allowances.
    #[returns(ListAllowancesResponse)]
    ListAllowances {
        /// The address to start after.
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns the config.
    #[returns(ConfigResponse)]
    Config {},
    /// Returns allowance for an address.
    #[returns(AllowanceResponse)]
    Allowance { address: String },
    /// Returns contract info.
    #[returns(dao_interface::voting::InfoResponse)]
    Info {},
    /// Returns info about the contract ownership.
    #[returns(cw_ownable::Ownership<cosmwasm_std::Addr>)]
    Ownership {},
}

#[cw_serde]
pub struct ConfigResponse {
    /// The config.
    pub config: Config,
}

#[cw_serde]
pub struct AllowanceEntry {
    /// The address.
    pub address: Addr,
    /// The allowance.
    pub allowance: Allowance,
}

#[cw_serde]
pub struct ListAllowancesResponse {
    /// The allowances.
    pub allowances: Vec<AllowanceEntry>,
}

#[cw_serde]
pub struct AllowanceResponse {
    /// The allowance.
    pub allowance: Allowance,
    /// Whether or not the allowance came from the member allowance fallback in
    /// the config.
    pub is_member_allowance: bool,
}

#[cw_serde]
pub struct MigrateMsg {}
