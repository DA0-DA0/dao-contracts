use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, Uint128, WasmMsg};

use crate::ContractError;

#[cw_serde]
pub enum NativeSendMsgs {
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

impl NativeSendMsgs {
    /// This is a helper function to convert the Cw20SendMsgs into a CosmosMsg
    ///
    /// Returns (amount_funds_to_send, CosmosMsg), we need `amount_funds_to_send` because later
    /// we make sure total amount of funds sent is equal to the amount of funds promised
    fn into_checked_cosmos_msg(
        self,
        deps: Deps,
        denom: &str, // Promised denom.
    ) -> Result<(Uint128, CosmosMsg), ContractError> {
        let verify_coin = |coins: &Vec<Coin>| {
            if coins.len() != 1 {
                return Err(ContractError::InvalidSendMsgFunds {});
            }
            if coins[0].amount.is_zero() {
                return Err(ContractError::InvalidSendMsgFunds {});
            }
            if denom != coins[0].denom {
                return Err(ContractError::InvalidSendMsgFunds {});
            }
            Ok(coins[0].amount)
        };

        match self {
            NativeSendMsgs::BankSend { to_address, amount } => Ok((
                verify_coin(&amount)?,
                BankMsg::Send {
                    to_address: deps.api.addr_validate(&to_address)?.to_string(),
                    amount,
                }
                .into(),
            )),
            NativeSendMsgs::BankBurn { amount } => {
                Ok((verify_coin(&amount)?, BankMsg::Burn { amount }.into()))
            }
            NativeSendMsgs::WasmExecute {
                contract_addr,
                msg,
                funds,
            } => Ok((
                verify_coin(&funds)?,
                WasmMsg::Execute {
                    contract_addr: deps.api.addr_validate(&contract_addr)?.to_string(),
                    msg,
                    funds,
                }
                .into(),
            )),
            NativeSendMsgs::WasmInstantiate {
                admin,
                code_id,
                msg,
                funds,
                label,
            } => Ok((
                verify_coin(&funds)?,
                WasmMsg::Instantiate {
                    admin: admin.map(|a| deps.api.addr_validate(&a).unwrap().to_string()),
                    code_id,
                    msg,
                    funds,
                    label,
                }
                .into(),
            )),
        }
    }
}

#[cw_serde]
pub enum Cw20SendMsgs {
    Cw20Send {
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
    Cw20Burn {
        amount: Uint128,
    },
    Cw20Transfer {
        recipient: String,
        amount: Uint128,
    },
}

impl Cw20SendMsgs {
    /// This is a helper function to convert the Cw20SendMsgs into a CosmosMsg
    ///
    /// Returns (amount_funds_to_send, CosmosMsg), we need `amount_funds_to_send` because later
    /// we make sure total amount of funds sent is equal to the amount of funds promised
    fn into_checked_cosmos_msg(
        self,
        deps: Deps,
        cw20_addr: &str,
    ) -> Result<(Uint128, CosmosMsg), ContractError> {
        match self {
            Cw20SendMsgs::Cw20Send {
                contract,
                amount,
                msg,
            } => Ok((
                amount,
                WasmMsg::Execute {
                    contract_addr: cw20_addr.to_string(),
                    msg: to_binary(&cw20::Cw20ExecuteMsg::Send {
                        contract: deps.api.addr_validate(&contract)?.to_string(),
                        amount,
                        msg,
                    })?,
                    funds: vec![],
                }
                .into(),
            )),
            Cw20SendMsgs::Cw20Burn { amount } => Ok((
                amount,
                WasmMsg::Execute {
                    contract_addr: cw20_addr.to_string(),
                    msg: to_binary(&cw20::Cw20ExecuteMsg::Burn { amount })?,
                    funds: vec![],
                }
                .into(),
            )),
            Cw20SendMsgs::Cw20Transfer { recipient, amount } => Ok((
                amount,
                WasmMsg::Execute {
                    contract_addr: cw20_addr.to_string(),
                    msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                        recipient: deps.api.addr_validate(&recipient)?.to_string(),
                        amount,
                    })?,
                    funds: vec![],
                }
                .into(),
            )),
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
    pub promise: SwapInfo,
}

impl Counterparty {
    pub fn into_checked(self, deps: Deps) -> Result<CheckedCounterparty, ContractError> {
        Ok(CheckedCounterparty {
            address: deps.api.addr_validate(&self.address)?,
            provided: false,
            promise: self.promise.into_checked(deps)?,
        })
    }
}

#[cw_serde]
pub struct CheckedCounterparty {
    pub address: Addr,
    pub promise: CheckedSwapInfo,
    pub provided: bool,
}

/// Information about the token being used on one side of the escrow.
#[cw_serde]
pub enum SwapInfo {
    /// A native token.
    Native {
        denom: String,
        amount: Uint128,
        on_completion: Vec<NativeSendMsgs>,
    },
    /// A cw20 token.
    Cw20 {
        contract_addr: String,
        amount: Uint128,
        on_completion: Vec<Cw20SendMsgs>,
    },
}

impl SwapInfo {
    pub fn into_checked(self, deps: Deps) -> Result<CheckedSwapInfo, ContractError> {
        match self {
            SwapInfo::Native {
                denom,
                amount,
                on_completion,
            } => {
                if amount.is_zero() {
                    Err(ContractError::ZeroTokens {})
                } else {
                    let on_completion = if on_completion.is_empty() {
                        vec![]
                    } else {
                        let mut total_amount = Uint128::zero();
                        let cosmos_msgs = on_completion
                            .into_iter()
                            .map(|msg| {
                                let (amount, cosmos_msg) =
                                    msg.into_checked_cosmos_msg(deps, &denom)?;
                                total_amount += amount;
                                Ok(cosmos_msg)
                            })
                            .collect::<Result<Vec<CosmosMsg>, ContractError>>()?;

                        // Verify that total amount of funds matches funds sent in all messages
                        if total_amount != amount {
                            return Err(ContractError::WrongFundsCalculation {});
                        }
                        cosmos_msgs
                    };

                    Ok(CheckedSwapInfo::Native {
                        denom,
                        amount,
                        on_completion,
                    })
                }
            }
            SwapInfo::Cw20 {
                contract_addr,
                amount,
                on_completion,
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

                    let on_completion = if on_completion.is_empty() {
                        vec![]
                    } else {
                        let mut total_amount = Uint128::zero();
                        let cosmos_msgs = on_completion
                            .into_iter()
                            .map(|msg| {
                                let (amount, cosmos_msg) =
                                    msg.into_checked_cosmos_msg(deps, contract_addr.as_str())?;
                                total_amount += amount;
                                Ok(cosmos_msg)
                            })
                            .collect::<Result<Vec<CosmosMsg>, ContractError>>()?;

                        // Verify that total amount of funds matches funds sent in all messages
                        if total_amount != amount {
                            return Err(ContractError::WrongFundsCalculation {});
                        }
                        cosmos_msgs
                    };

                    Ok(CheckedSwapInfo::Cw20 {
                        contract_addr,
                        amount,
                        on_completion,
                    })
                }
            }
        }
    }
}

#[cw_serde]
pub enum CheckedSwapInfo {
    Native {
        denom: String,
        amount: Uint128,
        on_completion: Vec<CosmosMsg>,
    },
    Cw20 {
        contract_addr: Addr,
        amount: Uint128,
        on_completion: Vec<CosmosMsg>,
    },
}

impl CheckedSwapInfo {
    pub fn into_send_message(
        self,
        recipient: String,
        is_withdraw: bool,
    ) -> Result<Vec<CosmosMsg>, ContractError> {
        Ok(match self {
            Self::Native {
                denom,
                amount,
                on_completion,
            } => {
                // If completion msgs was specified we send them
                if !is_withdraw && !on_completion.is_empty() {
                    return Ok(on_completion);
                }

                // If completion msgs was not specified we send funds to the other party
                vec![BankMsg::Send {
                    to_address: recipient,
                    amount: vec![Coin { denom, amount }],
                }
                .into()]
            }
            Self::Cw20 {
                contract_addr,
                amount,
                on_completion,
            } => {
                // If completion msgs was specified we send them
                if !is_withdraw && !on_completion.is_empty() {
                    return Ok(on_completion);
                }

                // If completion msgs was not specified we transfer funds to the other party
                vec![WasmMsg::Execute {
                    contract_addr: contract_addr.into_string(),
                    msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer { recipient, amount })?,
                    funds: vec![],
                }
                .into()]
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_into_spend_message_native() {
        let info = CheckedSwapInfo::Native {
            amount: Uint128::new(100),
            denom: "uekez".to_string(),
            on_completion: vec![CosmosMsg::Bank(BankMsg::Send {
                to_address: "ekez".to_string(),
                amount: vec![Coin {
                    amount: Uint128::new(100),
                    denom: "uekez".to_string(),
                }],
            })],
        };
        let message = info.into_send_message("ekez".to_string(), false).unwrap();

        assert_eq!(
            message[0],
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
        let info = CheckedSwapInfo::Cw20 {
            amount: Uint128::new(100),
            contract_addr: Addr::unchecked("ekez_token"),
            on_completion: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                funds: vec![],
                contract_addr: "ekez_token".to_string(),
                msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                    recipient: "ekez".to_string(),
                    amount: Uint128::new(100),
                })
                .unwrap(),
            })],
        };
        let message = info.into_send_message("ekez".to_string(), false).unwrap();

        assert_eq!(
            message[0],
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
