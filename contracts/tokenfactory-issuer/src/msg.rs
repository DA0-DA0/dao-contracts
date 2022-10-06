use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    pub denom: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    ChangeTokenFactoryAdmin {
        new_admin: String,
    },
    ChangeContractOwner {
        new_owner: String,
    },
    SetMinter {
        address: String,
        allowance: Uint128,
    },
    SetBurner {
        address: String,
        allowance: Uint128,
    },
    SetBlacklister {
        address: String,
        status: bool,
    },
    SetFreezer {
        address: String,
        status: bool,
    },
    Mint {
        to_address: String,
        amount: Uint128,
    },
    Burn {
        from_address: String,
        amount: Uint128,
    },
    Blacklist {
        address: String,
        status: bool,
    },
    Freeze {
        status: bool,
    },
}

/// SudoMsg is only exposed for internal Cosmos SDK modules to call.
/// This is showing how we can expose "admin" functionality than can not be called by
/// external users or contracts, but only trusted (native/Go) code in the blockchain
#[cw_serde]
pub enum SudoMsg {
    BlockBeforeSend {
        from: String,
        to: String,
        amount: Coin,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// IsFrozen returns if the entire token transfer functionality is frozen. Response: IsFrozenResponse
    #[returns(IsFrozenResponse)]
    IsFrozen {},
    /// Denom returns the token denom that this contract is the admin for. Response: DenomResponse
    #[returns(DenomResponse)]
    Denom {},
    /// Owner returns the owner of the contract. Response: OwnerResponse
    #[returns(OwnerResponse)]
    Owner {},
    /// Allowance returns the allowance of the specified address. Response: AllowanceResponse
    #[returns(AllowanceResponse)]
    BurnAllowance { address: String },
    /// Allowances Enumerates over all allownances. Response: Vec<AllowanceResponse>
    #[returns(AllowancesResponse)]
    BurnAllowances {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Allowance returns the allowance of the specified user. Response: AllowanceResponse
    #[returns(AllowanceResponse)]
    MintAllowance { address: String },
    /// Allowances Enumerates over all allownances. Response: AllowancesResponse
    #[returns(AllowancesResponse)]
    MintAllowances {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// IsBlacklisted returns wether the user is blacklisted or not. Response: StatusResponse
    #[returns(StatusResponse)]
    IsBlacklisted { address: String },
    /// Blacklistees enumerates over all addresses on the blacklist. Response: BlacklisteesResponse
    #[returns(BlacklisteesResponse)]
    Blacklistees {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// IsBlacklister returns if the addres has blacklister privileges. Response: StatusResponse
    #[returns(StatusResponse)]
    IsBlacklister { address: String },
    /// Blacklisters Enumerates over all the addresses with blacklister privileges. Response: BlacklisterAllowancesResponse
    #[returns(BlacklisterAllowancesResponse)]
    BlacklisterAllowances {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// IsFreezer returns whether the address has freezer status. Response: StatusResponse
    #[returns(StatusResponse)]
    IsFreezer { address: String },
    /// FreezerAllowances enumerates over all freezer addresses. Response: FreezerAllowancesResponse
    #[returns(FreezerAllowancesResponse)]
    FreezerAllowances {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

// We define a custom struct for each query response
#[cw_serde]
pub struct IsFrozenResponse {
    pub is_frozen: bool,
}

// We define a custom struct for each query response
#[cw_serde]
pub struct DenomResponse {
    pub denom: String,
}

#[cw_serde]
pub struct OwnerResponse {
    pub address: String,
}

#[cw_serde]
pub struct AllowanceResponse {
    pub allowance: u128,
}

#[cw_serde]
pub struct AllowanceInfo {
    pub address: String,
    pub allowance: u128,
}

#[cw_serde]
pub struct AllowancesResponse {
    pub allowances: Vec<AllowanceInfo>,
}

#[cw_serde]
pub struct StatusResponse {
    pub status: bool,
}

#[cw_serde]
pub struct StatusInfo {
    pub address: String,
    pub status: bool,
}

#[cw_serde]
pub struct BlacklisteesResponse {
    pub blacklistees: Vec<StatusInfo>,
}

#[cw_serde]
pub struct BlacklisterAllowancesResponse {
    pub blacklisters: Vec<StatusInfo>,
}

#[cw_serde]
pub struct FreezerAllowancesResponse {
    pub freezers: Vec<StatusInfo>,
}
