use cosmwasm_std::{to_binary, Addr, BankMsg, Coin, CosmosMsg, Deps, StdError, Uint128, WasmMsg};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    msg::{Counterparty, TokenInfo},
    ContractError,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckedTokenInfo {
    Native {
        denom: String,
        amount: Uint128,
    },
    Cw20 {
        contract_addr: Addr,
        amount: Uint128,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CheckedCounterparty {
    pub address: Addr,
    pub promise: CheckedTokenInfo,
    pub provided: bool,
}

pub const COUNTERPARTY_ONE: Item<CheckedCounterparty> = Item::new("counterparty_one");
pub const COUNTERPARTY_TWO: Item<CheckedCounterparty> = Item::new("counterparty_two");

impl Counterparty {
    pub fn into_checked(self, deps: Deps) -> Result<CheckedCounterparty, ContractError> {
        Ok(CheckedCounterparty {
            address: deps.api.addr_validate(&self.address)?,
            provided: false,
            promise: self.promise.into_checked(deps)?,
        })
    }
}

impl TokenInfo {
    pub fn into_checked(self, deps: Deps) -> Result<CheckedTokenInfo, ContractError> {
        match self {
            TokenInfo::Native { denom, amount } => {
                if amount.is_zero() {
                    Err(ContractError::ZeroTokens {})
                } else {
                    Ok(CheckedTokenInfo::Native { denom, amount })
                }
            }
            TokenInfo::Cw20 {
                contract_addr,
                amount,
            } => {
                if amount.is_zero() {
                    Err(ContractError::ZeroTokens {})
                } else {
                    let contract_addr = deps.api.addr_validate(&contract_addr)?;
                    // Make sure we are dealing with a cw20.
                    let _: cw20::TokenInfoResponse = deps.querier.query_wasm_smart(
                        contract_addr.clone(),
                        &cw20::Cw20QueryMsg::TokenInfo {},
                    )?;
                    Ok(CheckedTokenInfo::Cw20 {
                        contract_addr,
                        amount,
                    })
                }
            }
        }
    }
}

impl CheckedTokenInfo {
    pub fn into_send_message(self, recipient: &Addr) -> Result<CosmosMsg, StdError> {
        Ok(match self {
            Self::Native { denom, amount } => BankMsg::Send {
                to_address: recipient.to_string(),
                amount: vec![Coin { denom, amount }],
            }
            .into(),
            Self::Cw20 {
                contract_addr,
                amount,
            } => WasmMsg::Execute {
                contract_addr: contract_addr.into_string(),
                msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                    recipient: recipient.to_string(),
                    amount,
                })?,
                funds: vec![],
            }
            .into(),
        })
    }
}
