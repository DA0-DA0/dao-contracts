use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, StdError, Uint128, WasmMsg,
};

use crate::ContractError;

#[cw_serde]
pub struct SendMessage {
    pub address: String,
    pub message: Binary,
}

impl SendMessage {
    pub fn into_checked(self, deps: Deps) -> Result<CheckedSendMessage, ContractError> {
        Ok(CheckedSendMessage {
            address: deps.api.addr_validate(&self.address)?,
            message: self.message,
        })
    }
}

#[cw_serde]
pub struct CheckedSendMessage {
    pub address: Addr,
    pub message: Binary,
}

/// Information about the token being used on one side of the escrow.
#[cw_serde]
pub enum TokenInfo {
    /// A native token.
    Native { denom: String, amount: Uint128 },
    /// A cw20 token.
    Cw20 {
        contract_addr: String,
        amount: Uint128,
    },
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

/// Information about a counterparty in this escrow transaction and
/// their promised funds.
#[cw_serde]
pub struct Counterparty {
    /// The address of the counterparty.
    pub address: String,
    /// The funds they have promised to provide.
    pub promise: TokenInfo,
    /// The message to send once the escrow is complete.
    pub send_msg: Option<SendMessage>,
}

impl Counterparty {
    pub fn into_checked(self, deps: Deps) -> Result<CheckedCounterparty, ContractError> {
        let send_msg = if let Some(send_msg) = self.send_msg {
            Some(send_msg.into_checked(deps)?)
        } else {
            None
        };

        Ok(CheckedCounterparty {
            address: deps.api.addr_validate(&self.address)?,
            provided: false,
            promise: self.promise.into_checked(deps)?,
            send_msg,
        })
    }
}

#[cw_serde]
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

#[cw_serde]
pub struct CheckedCounterparty {
    pub address: Addr,
    pub promise: CheckedTokenInfo,
    pub provided: bool,
    pub send_msg: Option<CheckedSendMessage>,
}

impl CheckedTokenInfo {
    pub fn into_send_message(
        self,
        recipient: &Addr,
        send_msg: Option<CheckedSendMessage>,
    ) -> Result<CosmosMsg, StdError> {
        Ok(match self {
            Self::Native { denom, amount } => match send_msg {
                Some(CheckedSendMessage {
                    address,
                    message: msg,
                }) => WasmMsg::Execute {
                    contract_addr: address.to_string(),
                    msg,
                    funds: vec![Coin { denom, amount }],
                }
                .into(),
                None => BankMsg::Send {
                    to_address: recipient.to_string(),
                    amount: vec![Coin { denom, amount }],
                }
                .into(),
            },
            Self::Cw20 {
                contract_addr,
                amount,
            } => match send_msg {
                Some(CheckedSendMessage {
                    address,
                    message: msg,
                }) => WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    msg: to_binary(&cw20::Cw20ExecuteMsg::Send {
                        contract: address.to_string(),
                        amount,
                        msg,
                    })?,
                    funds: vec![],
                }
                .into(),
                None => WasmMsg::Execute {
                    contract_addr: contract_addr.into_string(),
                    msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                        recipient: recipient.to_string(),
                        amount,
                    })?,
                    funds: vec![],
                }
                .into(),
            },
        })
    }
}
