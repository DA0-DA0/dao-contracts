use cosmwasm_schema::{cw_serde, QueryResponses};
use dao_cw20::Cw20HookMsg;

#[cw_serde]
pub struct InstantiateMsg {
    pub dao: String,
    pub allowlist: Option<Vec<String>>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Cw20Hook(Cw20HookMsg),
    UpdateAllowlist {
        add: Vec<String>,
        remove: Vec<String>,
    },
}

// TODO token contract?
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the paginated list of addresses on the allowlist.
    #[returns(Vec<cosmwasm_std::Addr>)]
    Allowlist {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns the DAO address
    #[returns(cosmwasm_std::Addr)]
    Dao {},
    /// Returns the DAO voting module address
    #[returns(cosmwasm_std::Addr)]
    DaoVotingPowerModule {},
    /// Returns whether an address is allowed to recieve tokens
    #[returns(bool)]
    IsAllowed { address: String },
}

#[cw_serde]
pub struct MigrateMsg {}
