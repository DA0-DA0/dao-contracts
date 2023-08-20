use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Uint128};

// re-export Metadata related structs so that using this contract as lib can access the struct
pub use osmosis_std::types::cosmos::bank::v1beta1::{DenomUnit, Metadata};

#[cw_serde]
pub enum InstantiateMsg {
    /// `NewToken` will create a new token when instantiate the contract.
    /// Newly created token will have full denom as `factory/<contract_address>/<subdenom>`.
    /// It will be attached to the contract setup the beforesend listener automatically.
    NewToken {
        /// component of fulldenom (`factory/<contract_address>/<subdenom>`).
        subdenom: String,
    },
    /// `ExistingToken` will use already created token. So to set this up,
    /// tokenfactory admin needs to create a new token and set beforesend listener manually.
    ExistingToken { denom: String },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    ChangeTokenFactoryAdmin {
        new_admin: String,
    },
    ChangeContractOwner {
        new_owner: String,
    },
    SetDenomMetadata {
        /// Set denom metadata. see: https://docs.cosmos.network/main/modules/bank#denom-metadata.
        metadata: Metadata,
    },

    /// Grant/revoke mint allowance.
    SetMinter {
        address: String,
        allowance: Uint128,
    },

    /// Grant/revoke burn allowance.
    SetBurner {
        address: String,
        allowance: Uint128,
    },

    /// Grant/revoke permission to blacklist addresses
    SetBlacklister {
        address: String,
        status: bool,
    },

    /// Grant/revoke permission to freeze the token
    SetFreezer {
        address: String,
        status: bool,
    },

    /// Mint token to address. Mint allowance is required and wiil be deducted after successful mint.
    Mint {
        to_address: String,
        amount: Uint128,
    },

    /// Burn token to address. Burn allowance is required and wiil be deducted after successful burn.
    Burn {
        from_address: String,
        amount: Uint128,
    },

    /// Block target address from sending/receiving token attached to this contract
    /// tokenfactory's beforesend listener must be set to this contract in order for it to work as intended.
    Blacklist {
        address: String,
        status: bool,
    },

    /// Block every token transfers of the token attached to this contract
    /// tokenfactory's beforesend listener must be set to this contract in order for it to work as intended.
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
    pub allowance: Uint128,
}

#[cw_serde]
pub struct AllowanceInfo {
    pub address: String,
    pub allowance: Uint128,
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
