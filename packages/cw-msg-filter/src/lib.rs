#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    BankMsg, Coin, CosmosMsg, DistributionMsg, GovMsg, IbcMsg, StakingMsg, StdError, Uint128,
    WasmMsg,
};
use itertools::Itertools;
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MsgFilterError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error("Invalid GenericWasmExecuteMsg configuration")]
    InvalidConfiguration,

    #[error("Invalid message")]
    InvalidMsg {},

    #[error("Spending limit exceeded for {denom}. Limit: {limit}, Actual: {actual}")]
    SpendingLimitExceeded {
        denom: String,
        limit: Uint128,
        actual: Uint128,
    },

    #[error("{count} messages exceeded max_msg_count of {max}")]
    TooManyMsgs { count: u8, max: u8 },
}

#[cw_serde]
pub struct MsgFilter {
    /// Specify msgs that will be allowed through the filter
    /// If set to `None` only `max_msg_count` and `spending_limits`
    /// will be used for validation
    pub allowed_msgs: Option<Vec<AllowedMsg>>,
    /// The maximum number of messages that can be included.
    pub max_msg_count: Option<u8>,
    /// Global limitations on spending
    pub spending_limits: Option<Vec<Coin>>,
}

#[cw_serde]
pub enum AllowedMsg {
    /// An exact message to call, the message must match this message
    Exact(CosmosMsg),
    /// A generic smart contract message that only checks the method key
    /// being called, the contract address, and the funds used.
    /// At least one of the optional fields must be specified or an invalid
    /// configuration error will be thrown.
    GenericWasmExecuteMsg {
        /// The smart contract this message is intended for
        contract: Option<String>,
        /// Validation deserializes the binary and checks key matches
        key: Option<String>,
        /// Specify funds a smart contract call can't exceed
        funds: Option<Vec<Coin>>,
    },
    /// Generic types of message
    Type(MsgType),
}

#[cw_serde]
pub enum MsgType {
    BankSend,
    BankBurn,
    BankMint,
    Custom,
    StakingDelegate,
    StakingUndelegate,
    StakingRedelegate,
    StakingWithdraw,
    DistributionSetWithdrawAddress,
    DistributionWithdrawDelegatorReward,
    Stargate,
    Any,
    IbcTransfer,
    IbcCloseChannel,
    WasmExecute,
    WasmInstantiate,
    WasmMigrate,
    WasmUpdateAdmin,
    WasmClearAdmin,
    GovVote,
}

impl MsgFilter {
    /// Check messages, throws an error if any of the messages are not allowed or count / spending
    /// limits have been exceeded.
    pub fn check_messages(&self, messages: &[CosmosMsg]) -> Result<(), MsgFilterError> {
        // Check max_msg_count
        if let Some(max_count) = self.max_msg_count {
            if messages.len() > max_count as usize {
                return Err(MsgFilterError::TooManyMsgs {
                    count: messages.len() as u8,
                    max: max_count,
                });
            }
        }

        // Check each message
        for msg in messages {
            if !self.is_message_allowed(msg)? {
                return Err(MsgFilterError::InvalidMsg {});
            }
        }

        // Check spending limits
        if let Some(limits) = &self.spending_limits {
            let total_spend = Self::calculate_total_spend(messages);
            self.check_spending_limits(&total_spend, limits)?;
        }

        Ok(())
    }

    /// Returns a list of messages filtered for any messages that don't meet the criteria
    pub fn filter_messages<'a>(
        &self,
        messages: &'a [CosmosMsg],
    ) -> Result<Vec<&'a CosmosMsg>, MsgFilterError> {
        let filtered_msgs: Vec<&CosmosMsg> = messages
            .iter()
            .filter(|&msg| self.is_message_allowed(msg).unwrap_or(false))
            .collect();

        if let Some(max_count) = self.max_msg_count {
            if filtered_msgs.len() > max_count as usize {
                return Err(MsgFilterError::TooManyMsgs {
                    count: filtered_msgs.len() as u8,
                    max: max_count,
                });
            }
        }

        Ok(filtered_msgs)
    }

    /// Checks if a single message is allowed.
    pub fn is_message_allowed(&self, msg: &CosmosMsg) -> Result<bool, MsgFilterError> {
        if let Some(allowed_msgs) = &self.allowed_msgs {
            for allowed in allowed_msgs {
                match allowed {
                    AllowedMsg::Exact(exact_msg) => {
                        if msg == exact_msg {
                            return Ok(true);
                        }
                    }
                    AllowedMsg::GenericWasmExecuteMsg {
                        contract,
                        key,
                        funds,
                    } => {
                        // Ensure at least one field is specified in the GenericWasmExecuteMsg
                        // If all fields are None, it's an invalid configuration
                        if contract.is_none() && key.is_none() && funds.is_none() {
                            return Err(MsgFilterError::InvalidConfiguration);
                        }

                        // Check if the message is a Wasm Execute message
                        if let CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr,
                            msg: wasm_msg,
                            funds: msg_funds,
                        }) = msg
                        {
                            // Check if the contract address matches (if specified)
                            // If contract is None, this always returns true
                            let contract_matches =
                                contract.as_ref().map_or(true, |c| c == contract_addr);

                            // Check if the specified key exists in the message (if key is specified)
                            let key_matches = if let Some(key) = key {
                                // Deserialize the Wasm message
                                let msg_value: Value =
                                    serde_json::from_slice(wasm_msg).map_err(|e| {
                                        MsgFilterError::Std(StdError::generic_err(e.to_string()))
                                    })?;
                                println!("{:?} {:?}", msg_value, key);

                                // Check if the key exists in the deserialized message
                                msg_value.get(key).is_some()
                            } else {
                                // If no key is specified, consider it a match
                                true
                            };

                            println!("key matches {:?}", key_matches);

                            // Check if the funds are within the specified limit (if funds are specified)
                            // If funds is None, this always returns true
                            let funds_within_limit = funds
                                .as_ref()
                                .map_or(true, |f| Self::compare_funds(msg_funds, f));

                            // The message is allowed if all specified conditions are met
                            if contract_matches && key_matches && funds_within_limit {
                                return Ok(true);
                            }
                        }
                        // If we reach here, either:
                        // 1. The message wasn't a Wasm Execute message, or
                        // 2. One of the specified conditions wasn't met
                        // In both cases, we continue to the next AllowedMsg in the list
                    }
                    AllowedMsg::Type(msg_type) => match (msg_type, msg) {
                        (MsgType::BankSend, CosmosMsg::Bank(BankMsg::Send { .. })) => {
                            return Ok(true)
                        }
                        (MsgType::BankBurn, CosmosMsg::Bank(BankMsg::Burn { .. })) => {
                            return Ok(true)
                        }
                        // (MsgType::BankMint, CosmosMsg::Bank(BankMsg::Mint { .. })) => {
                        //     return Ok(true)
                        // }
                        (MsgType::Custom, CosmosMsg::Custom(_)) => return Ok(true),
                        (
                            MsgType::StakingDelegate,
                            CosmosMsg::Staking(StakingMsg::Delegate { .. }),
                        ) => return Ok(true),
                        (
                            MsgType::StakingUndelegate,
                            CosmosMsg::Staking(StakingMsg::Undelegate { .. }),
                        ) => return Ok(true),
                        (
                            MsgType::StakingRedelegate,
                            CosmosMsg::Staking(StakingMsg::Redelegate { .. }),
                        ) => return Ok(true),
                        // (
                        //     MsgType::StakingWithdraw,
                        //     CosmosMsg::Staking(StakingMsg::Withdraw { .. }),
                        // ) => return Ok(true),
                        (
                            MsgType::DistributionSetWithdrawAddress,
                            CosmosMsg::Distribution(DistributionMsg::SetWithdrawAddress { .. }),
                        ) => return Ok(true),
                        (
                            MsgType::DistributionWithdrawDelegatorReward,
                            CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward {
                                ..
                            }),
                        ) => return Ok(true),
                        (MsgType::Stargate, CosmosMsg::Stargate { .. }) => return Ok(true),
                        // (MsgType::Any, CosmosMsg::Any(_)) => return Ok(true),
                        (MsgType::IbcTransfer, CosmosMsg::Ibc(IbcMsg::Transfer { .. })) => {
                            return Ok(true)
                        }
                        (MsgType::IbcCloseChannel, CosmosMsg::Ibc(IbcMsg::CloseChannel { .. })) => {
                            return Ok(true)
                        }
                        (MsgType::WasmExecute, CosmosMsg::Wasm(WasmMsg::Execute { .. })) => {
                            return Ok(true)
                        }
                        (
                            MsgType::WasmInstantiate,
                            CosmosMsg::Wasm(WasmMsg::Instantiate { .. }),
                        ) => return Ok(true),
                        (MsgType::WasmMigrate, CosmosMsg::Wasm(WasmMsg::Migrate { .. })) => {
                            return Ok(true)
                        }
                        (
                            MsgType::WasmUpdateAdmin,
                            CosmosMsg::Wasm(WasmMsg::UpdateAdmin { .. }),
                        ) => return Ok(true),
                        (MsgType::WasmClearAdmin, CosmosMsg::Wasm(WasmMsg::ClearAdmin { .. })) => {
                            return Ok(true)
                        }
                        (MsgType::GovVote, CosmosMsg::Gov(GovMsg::Vote { .. })) => return Ok(true),
                        _ => continue,
                    },
                }
            }
            Ok(false)
        } else {
            Ok(true)
        }
    }

    /// Calculate total spend from a list of Cosmos Msgs
    pub fn calculate_total_spend(messages: &[CosmosMsg]) -> HashMap<String, Uint128> {
        messages
            .iter()
            .flat_map(Self::extract_coins_from_msg)
            .into_grouping_map_by(|coin| coin.denom.clone())
            .fold(Uint128::zero(), |acc, _, coin| acc + coin.amount)
    }

    /// Get spent coins from a CosmosMsg
    pub fn extract_coins_from_msg(msg: &CosmosMsg) -> Vec<Coin> {
        match msg {
            // Bank messages
            CosmosMsg::Bank(BankMsg::Send { amount, .. }) => amount.clone(),
            CosmosMsg::Bank(BankMsg::Burn { amount, .. }) => amount.clone(),
            // CosmosMsg::Bank(BankMsg::Mint { amount, .. }) => vec![amount.clone()],

            // Staking messages
            CosmosMsg::Staking(StakingMsg::Delegate { amount, .. }) => vec![amount.clone()],
            CosmosMsg::Staking(StakingMsg::Undelegate { amount, .. }) => {
                vec![amount.clone()]
            }
            CosmosMsg::Staking(StakingMsg::Redelegate { amount, .. }) => {
                vec![amount.clone()]
            }

            // Distribution messages
            CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward { .. }) => vec![], // No direct spending

            // IBC messages
            CosmosMsg::Ibc(IbcMsg::Transfer { amount, .. }) => vec![amount.clone()],

            // Wasm messages
            CosmosMsg::Wasm(WasmMsg::Execute { funds, .. }) => funds.clone(),
            CosmosMsg::Wasm(WasmMsg::Instantiate { funds, .. }) => funds.clone(),
            CosmosMsg::Wasm(WasmMsg::Migrate { .. }) => vec![], // No funds involved
            CosmosMsg::Wasm(WasmMsg::UpdateAdmin { .. }) => vec![], // No funds involved
            CosmosMsg::Wasm(WasmMsg::ClearAdmin { .. }) => vec![], // No funds involved

            // Stargate, Custom, and Gov messages
            CosmosMsg::Stargate { .. } => vec![], // We can't determine spending for Stargate messages generically
            CosmosMsg::Custom(_) => vec![],       // We can't determine spending for Custom messages
            CosmosMsg::Gov(_) => vec![], // Gov messages don't directly involve coin transfers

            // Any messages
            // CosmosMsg::Any(_) => vec![], // We can't determine spending for Any messages

            // For any other message types, return an empty vector
            _ => vec![],
        }
    }

    fn check_spending_limits(
        &self,
        total_spend: &HashMap<String, Uint128>,
        limits: &[Coin],
    ) -> Result<(), MsgFilterError> {
        for limit in limits {
            if let Some(&amount) = total_spend.get(&limit.denom) {
                if amount > limit.amount {
                    return Err(MsgFilterError::SpendingLimitExceeded {
                        denom: limit.denom.clone(),
                        limit: limit.amount,
                        actual: amount,
                    });
                }
            }
        }
        Ok(())
    }

    fn compare_funds(actual: &[Coin], limit: &[Coin]) -> bool {
        // Create a HashMap from the actual coins for efficient lookup
        // The key is the coin denomination, and the value is the coin amount
        let actual_map: HashMap<_, _> = actual.iter().map(|c| (&c.denom, c.amount)).collect();

        // Check if all limit coins are satisfied by the actual coins
        limit.iter().all(|limit_coin| {
            actual_map
                .get(&limit_coin.denom)
                // If the denomination exists in actual_map:
                //   Check if the actual amount is less than or equal to the limit amount
                // If the denomination doesn't exist in actual_map:
                //   Return true (no actual spending for this denomination, so it's within the limit)
                .map_or(true, |&amount| amount <= limit_coin.amount)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{
        coin, coins, to_json_binary, BankMsg, CosmosMsg, IbcMsg, IbcTimeout, StakingMsg, Timestamp,
        WasmMsg,
    };

    fn mock_bank_send_msg(to: &str, amount: u128, denom: &str) -> CosmosMsg {
        CosmosMsg::Bank(BankMsg::Send {
            to_address: to.to_string(),
            amount: coins(amount, denom),
        })
    }

    fn mock_wasm_execute_msg(contract: &str, msg: &str, funds: &[Coin]) -> CosmosMsg {
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract.to_string(),
            msg: to_json_binary(&serde_json::json!({ "action": msg })).unwrap(),
            funds: funds.to_vec(),
        })
    }

    fn mock_staking_delegate_msg(validator: &str, amount: u128, denom: &str) -> CosmosMsg {
        CosmosMsg::Staking(StakingMsg::Delegate {
            validator: validator.to_string(),
            amount: coin(amount, denom),
        })
    }

    fn mock_ibc_transfer_msg(
        channel: &str,
        recipient: &str,
        amount: u128,
        denom: &str,
    ) -> CosmosMsg {
        CosmosMsg::Ibc(IbcMsg::Transfer {
            channel_id: channel.to_string(),
            to_address: recipient.to_string(),
            amount: coin(amount, denom),
            timeout: IbcTimeout::with_timestamp(Timestamp::from_nanos(1_000_000_202)),
        })
    }

    #[test]
    fn test_bank_send_allowed() {
        let filter = MsgFilter {
            allowed_msgs: Some(vec![AllowedMsg::Type(MsgType::BankSend)]),
            max_msg_count: None,
            spending_limits: None,
        };

        let messages = vec![mock_bank_send_msg("recipient", 100, "utoken")];
        assert!(filter.check_messages(&messages).is_ok());

        let messages = vec![mock_wasm_execute_msg("contract", "action", &[])];
        assert!(filter.check_messages(&messages).is_err());
    }

    #[test]
    fn test_exact_message_allowed() {
        let allowed_msg = mock_bank_send_msg("recipient", 100, "utoken");
        let filter = MsgFilter {
            allowed_msgs: Some(vec![AllowedMsg::Exact(allowed_msg.clone())]),
            max_msg_count: None,
            spending_limits: None,
        };

        let messages = vec![allowed_msg.clone()];
        let result = filter.check_messages(&messages);
        assert!(result.is_ok());

        let different_msg = mock_bank_send_msg("recipient", 200, "utoken");
        let messages = vec![different_msg];
        let result = filter.check_messages(&messages);
        assert!(matches!(result, Err(MsgFilterError::InvalidMsg {})));
    }

    #[test]
    fn test_wasm_execute_allowed() {
        let filter = MsgFilter {
            allowed_msgs: Some(vec![AllowedMsg::Type(MsgType::WasmExecute)]),
            max_msg_count: None,
            spending_limits: None,
        };

        let messages = vec![mock_wasm_execute_msg(
            "contract",
            "action",
            &coins(100, "utoken"),
        )];
        assert!(filter.check_messages(&messages).is_ok());

        let messages = vec![mock_bank_send_msg("recipient", 100, "utoken")];
        assert!(filter.check_messages(&messages).is_err());
    }

    #[test]
    fn test_multiple_allowed_types() {
        let filter = MsgFilter {
            allowed_msgs: Some(vec![
                AllowedMsg::Type(MsgType::BankSend),
                AllowedMsg::Type(MsgType::WasmExecute),
            ]),
            max_msg_count: None,
            spending_limits: None,
        };

        let messages = vec![
            mock_bank_send_msg("recipient", 100, "utoken"),
            mock_wasm_execute_msg("contract", "action", &coins(50, "utoken")),
        ];
        assert!(filter.check_messages(&messages).is_ok());

        let messages = vec![mock_staking_delegate_msg("validator", 100, "utoken")];
        assert!(filter.check_messages(&messages).is_err());
    }

    #[test]
    fn test_max_msg_count() {
        let filter = MsgFilter {
            allowed_msgs: Some(vec![AllowedMsg::Type(MsgType::BankSend)]),
            max_msg_count: Some(2),
            spending_limits: None,
        };

        let messages = vec![
            mock_bank_send_msg("recipient1", 100, "utoken"),
            mock_bank_send_msg("recipient2", 100, "utoken"),
        ];
        assert!(filter.check_messages(&messages).is_ok());

        let messages = vec![
            mock_bank_send_msg("recipient1", 100, "utoken"),
            mock_bank_send_msg("recipient2", 100, "utoken"),
            mock_bank_send_msg("recipient3", 100, "utoken"),
        ];
        assert!(filter.check_messages(&messages).is_err());
    }

    #[test]
    fn test_spending_limits() {
        let filter = MsgFilter {
            allowed_msgs: Some(vec![
                AllowedMsg::Type(MsgType::BankSend),
                AllowedMsg::Type(MsgType::WasmExecute),
            ]),
            max_msg_count: None,
            spending_limits: Some(coins(250, "utoken")),
        };

        let messages = vec![
            mock_bank_send_msg("recipient", 100, "utoken"),
            mock_wasm_execute_msg("contract", "action", &coins(100, "utoken")),
        ];
        assert!(filter.check_messages(&messages).is_ok());

        let messages = vec![
            mock_bank_send_msg("recipient", 200, "utoken"),
            mock_wasm_execute_msg("contract", "action", &coins(100, "utoken")),
        ];
        assert!(filter.check_messages(&messages).is_err());
    }

    #[test]
    fn test_generic_wasm_execute_msg() {
        // Test with all fields specified
        let filter = MsgFilter {
            allowed_msgs: Some(vec![AllowedMsg::GenericWasmExecuteMsg {
                contract: Some("allowed_contract".to_string()),
                key: Some("action".to_string()),
                funds: Some(coins(100, "utoken")),
            }]),
            max_msg_count: None,
            spending_limits: None,
        };

        let valid_msg = mock_wasm_execute_msg("allowed_contract", "action", &coins(50, "utoken"));
        assert!(filter.check_messages(&[valid_msg]).is_ok());

        let invalid_contract =
            mock_wasm_execute_msg("other_contract", "action", &coins(50, "utoken"));
        assert!(filter.check_messages(&[invalid_contract]).is_err());

        let invalid_action = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "allowed_contract".to_string(),
            msg: to_json_binary(&serde_json::json!({ "other_action": {} })).unwrap(),
            funds: coins(50, "utoken"),
        });
        assert!(filter.check_messages(&[invalid_action]).is_err());

        let invalid_funds =
            mock_wasm_execute_msg("allowed_contract", "action", &coins(150, "utoken"));
        assert!(filter.check_messages(&[invalid_funds]).is_err());

        // Test with only contract specified
        let filter = MsgFilter {
            allowed_msgs: Some(vec![AllowedMsg::GenericWasmExecuteMsg {
                contract: Some("allowed_contract".to_string()),
                key: None,
                funds: None,
            }]),
            max_msg_count: None,
            spending_limits: None,
        };

        let valid_msg =
            mock_wasm_execute_msg("allowed_contract", "any_action", &coins(1000, "utoken"));
        assert!(filter.check_messages(&[valid_msg]).is_ok());

        // Test with only key specified
        let filter = MsgFilter {
            allowed_msgs: Some(vec![AllowedMsg::GenericWasmExecuteMsg {
                contract: None,
                key: Some("action".to_string()),
                funds: None,
            }]),
            max_msg_count: None,
            spending_limits: None,
        };

        let valid_msg = mock_wasm_execute_msg("any_contract", "action", &coins(1000, "utoken"));
        assert!(filter.check_messages(&[valid_msg]).is_ok());

        // Test with only funds specified
        let filter = MsgFilter {
            allowed_msgs: Some(vec![AllowedMsg::GenericWasmExecuteMsg {
                contract: None,
                key: None,
                funds: Some(coins(100, "utoken")),
            }]),
            max_msg_count: None,
            spending_limits: None,
        };

        let valid_msg = mock_wasm_execute_msg("any_contract", "any_action", &coins(50, "utoken"));
        assert!(filter.check_messages(&[valid_msg]).is_ok());

        // Test invalid configuration
        let filter = MsgFilter {
            allowed_msgs: Some(vec![AllowedMsg::GenericWasmExecuteMsg {
                contract: None,
                key: None,
                funds: None,
            }]),
            max_msg_count: None,
            spending_limits: None,
        };

        let msg = mock_wasm_execute_msg("any_contract", "any_action", &coins(50, "utoken"));
        assert!(matches!(
            filter.check_messages(&[msg]),
            Err(MsgFilterError::InvalidConfiguration)
        ));
    }

    #[test]
    fn test_multiple_message_types() {
        let filter = MsgFilter {
            allowed_msgs: Some(vec![
                AllowedMsg::Type(MsgType::BankSend),
                AllowedMsg::Type(MsgType::StakingDelegate),
                AllowedMsg::Type(MsgType::IbcTransfer),
            ]),
            max_msg_count: None,
            spending_limits: Some(coins(500, "utoken")),
        };

        let messages = vec![
            mock_bank_send_msg("recipient", 100, "utoken"),
            mock_staking_delegate_msg("validator", 200, "utoken"),
            mock_ibc_transfer_msg("channel-1", "recipient", 150, "utoken"),
        ];
        assert!(filter.check_messages(&messages).is_ok());

        let messages = vec![
            mock_bank_send_msg("recipient", 200, "utoken"),
            mock_staking_delegate_msg("validator", 200, "utoken"),
            mock_ibc_transfer_msg("channel-1", "recipient", 150, "utoken"),
        ];
        assert!(filter.check_messages(&messages).is_err());
    }

    #[test]
    fn test_max_message_count() {
        let filter = MsgFilter {
            allowed_msgs: None,
            max_msg_count: Some(2),
            spending_limits: None,
        };

        let messages = vec![
            mock_bank_send_msg("recipient1", 100, "utoken"),
            mock_bank_send_msg("recipient2", 200, "utoken"),
            mock_bank_send_msg("recipient3", 300, "utoken"),
        ];

        let result = filter.check_messages(&messages);
        assert!(matches!(
            result,
            Err(MsgFilterError::TooManyMsgs { count: 3, max: 2 })
        ));
    }
}
