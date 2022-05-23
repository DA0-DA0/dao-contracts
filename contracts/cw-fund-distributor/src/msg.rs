use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    /// An optional admin for this contract. The admin may withdraw
    /// funds that have been earmarked for distribution by this
    /// contract.
    pub admin: Option<String>,
    /// The voting power contract that this contract will use to
    /// derermine rewards entitlement for addresses. Funds will be
    /// distributed porportional to the amount of voting power an
    /// address has. This contract must implement the
    /// `VotingPowerAtHeight` and `TotalPowerAtHeight` queries.
    pub voting_contract: String,
    /// The block height at which voting power will be determined for
    /// fund distribution.
    pub distribution_height: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Funds the contract with a cw20 token. The token is added to
    /// the contract's list of tokens to be distributed.
    Receive(cw20::Cw20ReceiveMsg),
    /// Funds the contract with native tokens. Any tokens sent in the
    /// `funds` field of this message will be distributed by this
    /// contract.
    Fund {},
    /// Sends the cw20 tokens the sender is entitled to to the
    /// sender. Errors if any of the tokens in TOKENS are not being
    /// distributed by this contract.
    ///
    /// If TOKENS is None all cw20 tokens avaliable are paid out.
    ClaimCw20s { tokens: Option<Vec<String>> },
    /// Sends the native tokens the sender is entitled to to the
    /// sender. Errors if any of the denoms in DENOMS are not being
    /// distributed by this contract.
    ///
    /// If DENOMS iis None all native tokens avaliable are paid out.
    ClaimNatives { denoms: Option<Vec<String>> },

    /// Callable only by the contract's admin. Returns this contract's
    /// token balances to the contract's admin. Errors if any token in
    /// TOKENS is not being distributed by this contract. If no TOKENS
    /// are specified all tokens are withdrawn.
    WithdrawCw20s { tokens: Option<Vec<String>> },
    /// Callable only by the contract's admin. Returns this contracts
    /// native balances to the contract's admin. Errors if any denom
    /// in DENOMS is not being distributed by this contract. If no
    /// DENOMS are specified all native denoms are withdrawn.
    WithdrawNatives { denoms: Option<Vec<String>> },
    /// Callable by the contract's admin. Updates the contract's admin
    /// to the new value.
    UpdateAdmin { admin: Option<String> },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Lists the native tokens this contract is distributing. Returns
    /// Vec<DenomResponse>.
    NativeDenoms {
        start_at: Option<String>,
        limit: Option<u32>,
    },
    /// Lists the cw20 tokens this contract is distributing. Returns
    /// Vec<DenomResponse>.
    Cw20Denoms {
        start_at: Option<String>,
        limit: Option<u32>,
    },
    /// Gets the number of native tokens ADDRESS is entitled to given
    /// a native token denom. If DENOM is not being distributed by the
    /// contract an error is returned, otherwise returns
    /// NativeEntitlementResponse.
    NativeEntitlement { address: String, denom: String },
    /// Gets the number of cw20 tokens ADDRESS is entitled to given a
    /// cw20 token address. If TOKEN is not being distributed by the
    /// contract an error is returned, otherwise returns
    /// Cw20EntitlementResponse.
    Cw20Entitlement { address: String, token: String },
    /// Lists all of the native entitlements for ADDRESS. Returns
    /// Vec<NativeEntitlementResponse>.
    NativeEntitlements {
        address: String,
        start_at: Option<String>,
        limit: Option<u32>,
    },
    /// Lists all of the cw20 entitlements for ADDRESS. Returns
    /// Vec<Cw20EntitlementResponse>.
    Cw20Entitlements {
        address: String,
        start_at: Option<String>,
        limit: Option<u32>,
    },
    /// Gets the current admin of the contract. Returns AdminResponse.
    Admin {},
    /// Gets information about the contract's voting power
    /// contract. Returns VotingContractResponse.
    VotingContract {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct DenomResponse {
    /// The contract's balance.
    pub contract_balance: Uint128,
    /// The denom for these tokens.
    pub denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct NativeEntitlementResponse {
    /// The amount of tokens the address is entitled to.
    pub amount: Uint128,
    /// The denom for these tokens.
    pub denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Cw20EntitlementResponse {
    /// The amount of tokens the address is entitled to.
    pub amount: Uint128,
    /// The address of the token contract.
    pub token_contract: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct AdminResponse {
    pub admin: Option<Addr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct VotingContractResponse {
    /// The voting power contract being used.
    pub contract: Addr,
    /// The height at which voting power is being determined.
    pub distribution_height: u64,
}
