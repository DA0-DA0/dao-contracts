use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Uint128};
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
    /// Token Factory admin for the existing token needs trasfer admin over
    /// to this contract, and optionally set the `BeforeSendHook` manually.
    ExistingToken { denom: String },
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Allow adds the target address to the allowlist to be able to send or recieve tokens even if the token
    /// is frozen. Token Factory's BeforeSendHook listener must be set to this contract in order for this feature
    /// to work.
    ///
    /// This functionality is intedended for DAOs who do not wish to have a their tokens liquid while bootstrapping
    /// their DAO. For example, a DAO may wish to white list a Token Staking contract (to allow users to stake their
    /// tokens in the DAO) or a Merkle Drop contract (to allow users to claim their tokens).
    Allow { address: String, status: bool },

    /// Burn token to address. Burn allowance is required and wiil be deducted after successful burn.
    Burn {
        from_address: String,
        amount: Uint128,
    },

    /// Mint token to address. Mint allowance is required and wiil be deducted after successful mint.
    Mint { to_address: String, amount: Uint128 },

    /// Deny adds the target address to the denylist, whis prevents them from sending/receiving the token attached
    /// to this contract tokenfactory's BeforeSendHook listener must be set to this contract in order for this
    /// feature to work as intended.
    Deny { address: String, status: bool },

    /// Block every token transfers of the token attached to this contract.
    /// Token Factory's BeforeSendHook listener must be set to this contract in order for this
    /// feature to work as intended.
    Freeze { status: bool },

    /// Force transfer token from one address to another.
    ForceTransfer {
        amount: Uint128,
        from_address: String,
        to_address: String,
    },

    /// Attempt to SetBeforeSendHook on the token attached to this contract.
    /// This will fail if the token already has a SetBeforeSendHook or the chain
    /// still does not support it.
    SetBeforeSendHook {},

    /// Grant/revoke burn allowance.
    SetBurnerAllowance { address: String, allowance: Uint128 },

    /// Set denom metadata. see: https://docs.cosmos.network/main/modules/bank#denom-metadata.
    SetDenomMetadata { metadata: Metadata },

    /// Grant/revoke mint allowance.
    SetMinterAllowance { address: String, allowance: Uint128 },

    /// Updates the admin of the Token Factory token.
    /// Normally this is the cw-tokenfactory-issuer contract itself.
    /// This is intended to be used only if you seek to transfer ownership
    /// of the Token somewhere else (i.e. to another management contract).
    UpdateTokenFactoryAdmin { new_admin: String },

    /// Updates the owner of this contract who is allowed to call privileged methods.
    /// NOTE: this is separate from the Token Factory token admin, for this contract to work
    /// at all, it needs to the be the Token Factory token admin.
    ///
    /// Normally, the contract owner will be a DAO.
    UpdateContractOwner { new_owner: String },
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

    /// IsDenied returns wether the user is on denylist or not. Response: StatusResponse
    #[returns(StatusResponse)]
    IsDenied { address: String },

    /// Denylist enumerates over all addresses on the denylist. Response: DenylistResponse
    #[returns(DenylistResponse)]
    Denylist {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// IsAllowed returns wether the user is on the allowlist or not. Response: StatusResponse
    #[returns(StatusResponse)]
    IsAllowed { address: String },

    /// Allowlist enumerates over all addresses on the allowlist. Response: AllowlistResponse
    #[returns(AllowlistResponse)]
    Allowlist {
        start_after: Option<String>,
        limit: Option<u32>,
    },

    /// Returns whether features that require MsgBeforeSendHook are enabled
    /// Most Cosmos chains do not support this feature yet.
    #[returns(bool)]
    BeforeSendHookFeaturesEnabled {},
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
pub struct DenylistResponse {
    pub denylist: Vec<StatusInfo>,
}

#[cw_serde]
pub struct AllowlistResponse {
    pub allowlist: Vec<StatusInfo>,
}
