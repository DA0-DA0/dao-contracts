use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, MessageInfo, StdError, Uint128,
    WasmMsg,
};
use cw_utils::must_pay;

use crate::ContractError;

#[cw_serde]
pub enum AcceptedMessages {
    BankSend {
        to_address: String,
        amount: Vec<Coin>,
    },
    BankBurn {
        amount: Vec<Coin>,
    },
    WasmExecute {
        contract_addr: String,
        msg: Binary,
        funds: Vec<Coin>,
    },
    WasmInstantiate {
        admin: Option<String>,
        code_id: u64,
        msg: Binary,
        funds: Vec<Coin>,
        label: String,
    },
}

/// Enum to accept either a cosmos msg (for recieved native tokens)
/// or address and binary message (for recieved cw20 tokens)
#[cw_serde]
pub enum SendMessage {
    SendCw20 {
        /// Contract address to execute the msg on
        contract_address: String,
        /// The message in binary format
        message: Binary,
    },
    SendNative {
        /// Vector of accepted messages to send
        messages: Vec<AcceptedMessages>,
    },
}

impl SendMessage {
    pub fn into_checked_cosmos_msgs(
        self,
        deps: Deps,
        other_token_info: CheckedTokenInfo,
    ) -> Result<Vec<CosmosMsg>, ContractError> {
        match self {
            SendMessage::SendCw20 {
                contract_address,
                message,
            } => {
                // We check if the other party token type is cw20
                if let CheckedTokenInfo::Cw20 {
                    contract_addr: cw20_address,
                    amount,
                } = other_token_info
                {
                    Ok(vec![WasmMsg::Execute {
                        contract_addr: cw20_address.to_string(),
                        msg: to_binary(&cw20::Cw20ExecuteMsg::Send {
                            contract: deps.api.addr_validate(&contract_address)?.to_string(),
                            amount,
                            msg: message,
                        })?,
                        funds: vec![],
                    }
                    .into()])
                } else {
                    return Err(ContractError::InvalidSendMsg {});
                }
            }
            SendMessage::SendNative { messages } => {
                if let CheckedTokenInfo::Native {
                    amount: total_amount,
                    denom,
                } = other_token_info
                {
                    if messages.is_empty() {
                        return Err(ContractError::InvalidSendMsg {});
                    }

                    let mut total_send_funds = Uint128::zero();
                    let cosmos_msgs = messages
                        .into_iter()
                        .map(|msg| match msg {
                            AcceptedMessages::BankSend {
                                to_address,
                                amount: amount_to_pay,
                            } => {
                                total_send_funds =
                                    self.add_to_total(total_send_funds, amount_to_pay, denom)?;

                                Ok(BankMsg::Send {
                                    to_address: deps.api.addr_validate(&to_address)?.to_string(),
                                    amount: amount_to_pay,
                                }
                                .into())
                            }
                            AcceptedMessages::BankBurn {
                                amount: amount_to_pay,
                            } => {
                                total_send_funds =
                                    self.add_to_total(total_send_funds, amount_to_pay, denom)?;

                                Ok(BankMsg::Burn {
                                    amount: amount_to_pay,
                                }
                                .into())
                            }
                            AcceptedMessages::WasmExecute {
                                contract_addr,
                                msg,
                                funds: amount_to_pay,
                            } => {
                                total_send_funds =
                                    self.add_to_total(total_send_funds, amount_to_pay, denom)?;

                                Ok(WasmMsg::Execute {
                                    contract_addr: deps
                                        .api
                                        .addr_validate(&contract_addr)?
                                        .to_string(),
                                    msg,
                                    funds: amount_to_pay,
                                }
                                .into())
                            }
                            AcceptedMessages::WasmInstantiate {
                                admin,
                                code_id,
                                msg,
                                funds: amount_to_pay,
                                label,
                            } => {
                                total_send_funds =
                                    self.add_to_total(total_send_funds, amount_to_pay, denom)?;

                                Ok(WasmMsg::Instantiate {
                                    admin: admin
                                        .map(|a| deps.api.addr_validate(&a).unwrap().to_string()),
                                    code_id,
                                    msg,
                                    funds: amount_to_pay,
                                    label,
                                }
                                .into())
                            }
                        })
                        .collect::<Result<Vec<CosmosMsg>, ContractError>>()?;

                    // Make sure that the funds we try to send, matches exactly to the total amount swapped.
                    if total_send_funds != total_amount {
                        return Err(ContractError::InvalidFunds {});
                    }

                    Ok(cosmos_msgs)
                } else {
                    return Err(ContractError::InvalidSendMsg {});
                }
            }
        }
    }
    /// This function check the given funds to make sure they are valid.
    /// and add the amount to the total funds.
    /// returns the new total.
    pub fn add_to_total(
        self,
        total_funds: Uint128,
        funds: Vec<Coin>,
        denom: String,
    ) -> Result<Uint128, ContractError> {
        let amount = must_pay(
            &MessageInfo {
                sender: Addr::unchecked(""),
                funds,
            },
            &denom,
        )?;

        Ok(total_funds + amount)
    }
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
}

impl Counterparty {
    pub fn into_checked(self, deps: Deps, send_msg: Option<Vec<CosmosMsg>>) -> Result<CheckedCounterparty, ContractError> {
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
    pub send_msg: Option<Vec<CosmosMsg>>
}

impl CheckedTokenInfo {
    pub fn into_send_message(
        self,
        deps: Deps,
        other_counterparty: &CheckedCounterparty,
        send_msg: Option<Vec<CosmosMsg>>,
    ) -> Result<Vec<CosmosMsg>, StdError> {
        Ok(match self {
            Self::Native { denom, amount } => match send_msg {
                Some(msgs) => msgs,
                None => vec![BankMsg::Send {
                    to_address: other_counterparty.address.to_string(),
                    amount: vec![Coin { denom, amount }],
                }
                .into()],
            },
            Self::Cw20 {
                contract_addr,
                amount,
            } => match send_msg {
                Some(msgs) => msgs,
                None => vec![WasmMsg::Execute {
                    contract_addr: contract_addr.into_string(),
                    msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                        recipient: other_counterparty.address.to_string(),
                        amount,
                    })?,
                    funds: vec![],
                }
                .into()],
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_into_spend_message_native() {
        let info = CheckedTokenInfo::Native {
            amount: Uint128::new(100),
            denom: "uekez".to_string(),
        };
        let message = info.into_send_message(&Addr::unchecked("ekez")).unwrap();

        assert_eq!(
            message,
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "ekez".to_string(),
                amount: vec![Coin {
                    amount: Uint128::new(100),
                    denom: "uekez".to_string()
                }]
            })
        );
    }

    #[test]
    fn test_into_spend_message_cw20() {
        let info = CheckedTokenInfo::Cw20 {
            amount: Uint128::new(100),
            contract_addr: Addr::unchecked("ekez_token"),
        };
        let message = info.into_send_message(&Addr::unchecked("ekez")).unwrap();

        assert_eq!(
            message,
            CosmosMsg::Wasm(WasmMsg::Execute {
                funds: vec![],
                contract_addr: "ekez_token".to_string(),
                msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                    recipient: "ekez".to_string(),
                    amount: Uint128::new(100)
                })
                .unwrap()
            })
        );
    }
}
