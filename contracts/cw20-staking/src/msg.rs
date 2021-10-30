use cosmwasm_std::{Binary, Uint128};
use cw20::{Expiration, Logo};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw0::Duration;

pub use cw_controllers::ClaimsResponse;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMsg {
    pub cw20_base: cw20_base::msg::InstantiateMsg,
    pub unstaking_duration: Option<Duration>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Transfer {
        recipient: String,
        amount: Uint128,
    },
    Burn {
        amount: Uint128,
    },
    Send {
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
    IncreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    DecreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    TransferFrom {
        owner: String,
        recipient: String,
        amount: Uint128,
    },
    SendFrom {
        owner: String,
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
    BurnFrom {
        owner: String,
        amount: Uint128,
    },
    Mint {
        recipient: String,
        amount: Uint128,
    },
    UpdateMarketing {
        project: Option<String>,
        description: Option<String>,
        marketing: Option<String>,
    },
    UploadLogo(Logo),
    Stake {
        amount: Uint128,
    },
    Unstake {
        amount: Uint128,
    },
    Claim {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Returns the current balance of the given address, 0 if unset.
    /// Return type: BalanceResponse.
    Balance {
        address: String,
    },
    /// Returns metadata on the contract - name, decimals, supply, etc.
    /// Return type: TokenInfoResponse.
    TokenInfo {},
    /// Only with "mintable" extension.
    /// Returns who can mint and the hard cap on maximum tokens after minting.
    /// Return type: MinterResponse.
    Minter {},
    /// Only with "allowance" extension.
    /// Returns how much spender can use from owner account, 0 if unset.
    /// Return type: AllowanceResponse.
    Allowance {
        owner: String,
        spender: String,
    },
    /// Only with "enumerable" extension (and "allowances")
    /// Returns all allowances this owner has approved. Supports pagination.
    /// Return type: AllAllowancesResponse.
    AllAllowances {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Only with "enumerable" extension
    /// Returns all accounts that have balances. Supports pagination.
    /// Return type: AllAccountsResponse.
    AllAccounts {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Only with "marketing" extension
    /// Returns more metadata on the contract to display in the client:
    /// - description, logo, project url, etc.
    /// Return type: MarketingInfoResponse
    MarketingInfo {},
    /// Only with "marketing" extension
    /// Downloads the embeded logo data (if stored on chain). Errors if no logo data ftored for this
    /// contract.
    /// Return type: DownloadLogoResponse.
    DownloadLogo {},
    StakedBalanceAtHeight {
        address: String,
        height: Option<u64>,
    },
    TotalStakedAtHeight {
        height: Option<u64>,
    },
    UnstakingDuration {},
    Claims {
        address: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StakedBalanceAtHeightResponse {
    pub balance: Uint128,
    pub height: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TotalStakedAtHeightResponse {
    pub total: Uint128,
    pub height: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct UnstakingDurationResponse {
    pub duration: Option<Duration>,
}
