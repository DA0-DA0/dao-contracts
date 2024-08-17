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

    #[error("Generic Wasm Execute Message has no criteria specified")]
    EmptyGenericWasmExecuteMsg,

    #[error("Invalid coin amount in funds: {denom}")]
    InvalidCoinAmount { denom: String },

    #[error("Invalid GenericWasmExecuteMsg configuration")]
    InvalidConfiguration,

    #[error("Invalid max_msg_count: {0}")]
    InvalidMaxMsgCount(u8),

    #[error("Invalid message")]
    InvalidMsg {},

    #[error("Invalid spending limit: {denom} amount must be positive")]
    InvalidSpendingLimit { denom: String },

    #[error("No filtering criteria specified")]
    NoFilteringCriteria,

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
    StakingDelegate,
    StakingUndelegate,
    StakingRedelegate,
    StakingWithdraw,
    DistributionSetWithdrawAddress,
    DistributionWithdrawDelegatorReward,
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
    /// Validates message filter configuration
    pub fn validate_config(&self) -> Result<(), MsgFilterError> {
        // Check if any filtering criteria is specified
        if self.allowed_msgs.is_none()
            && self.max_msg_count.is_none()
            && self.spending_limits.is_none()
        {
            return Err(MsgFilterError::NoFilteringCriteria);
        }

        // Validate max_msg_count
        if let Some(max_count) = self.max_msg_count {
            if max_count == 0 {
                return Err(MsgFilterError::InvalidMaxMsgCount(max_count));
            }
        }

        // Validate spending_limits
        if let Some(limits) = &self.spending_limits {
            for coin in limits {
                if coin.amount.is_zero() {
                    return Err(MsgFilterError::InvalidSpendingLimit {
                        denom: coin.denom.clone(),
                    });
                }
            }
        }

        // Validate allowed_msgs
        if let Some(allowed_msgs) = &self.allowed_msgs {
            for msg in allowed_msgs {
                match msg {
                    AllowedMsg::Exact(_) => {} // No additional validation needed
                    AllowedMsg::GenericWasmExecuteMsg {
                        contract,
                        key,
                        funds,
                    } => {
                        if contract.is_none() && key.is_none() && funds.is_none() {
                            return Err(MsgFilterError::EmptyGenericWasmExecuteMsg);
                        }
                        if let Some(funds) = funds {
                            for coin in funds {
                                if coin.amount.is_zero() {
                                    return Err(MsgFilterError::InvalidCoinAmount {
                                        denom: coin.denom.clone(),
                                    });
                                }
                            }
                        }
                    }
                    AllowedMsg::Type(_) => {} // No additional validation needed
                }
            }
        }

        Ok(())
    }

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
            check_spending_limits(&total_spend, limits)?;
        }

        Ok(())
    }

    /// Returns a list of messages filtering out any messages that don't meet the criteria
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
                            let funds_within_limit =
                                funds.as_ref().map_or(true, |f| compare_funds(msg_funds, f));

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
                    AllowedMsg::Type(msg_type) => {
                        if Self::matches_msg_type(msg_type, msg) {
                            return Ok(true);
                        }
                    }
                }
            }
            Ok(false)
        } else {
            Ok(true)
        }
    }

    /// Matches a message type. NOTE: We've removed the Custom, Stargate, and Any message types.
    /// These are too broad and could potentially allow unintended messages. Additionally, we can't
    /// easily track spending from these messages.
    pub fn matches_msg_type(msg_type: &MsgType, msg: &CosmosMsg) -> bool {
        matches!(
            (msg_type, msg),
            (MsgType::BankSend, CosmosMsg::Bank(BankMsg::Send { .. }))
                | (MsgType::BankBurn, CosmosMsg::Bank(BankMsg::Burn { .. }))
                | (
                    MsgType::StakingDelegate,
                    CosmosMsg::Staking(StakingMsg::Delegate { .. })
                )
                | (
                    MsgType::StakingUndelegate,
                    CosmosMsg::Staking(StakingMsg::Undelegate { .. })
                )
                | (
                    MsgType::StakingRedelegate,
                    CosmosMsg::Staking(StakingMsg::Redelegate { .. })
                )
                | (
                    MsgType::DistributionSetWithdrawAddress,
                    CosmosMsg::Distribution(DistributionMsg::SetWithdrawAddress { .. })
                )
                | (
                    MsgType::DistributionWithdrawDelegatorReward,
                    CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward { .. })
                )
                | (
                    MsgType::IbcTransfer,
                    CosmosMsg::Ibc(IbcMsg::Transfer { .. })
                )
                | (
                    MsgType::IbcCloseChannel,
                    CosmosMsg::Ibc(IbcMsg::CloseChannel { .. })
                )
                | (
                    MsgType::WasmExecute,
                    CosmosMsg::Wasm(WasmMsg::Execute { .. })
                )
                | (
                    MsgType::WasmInstantiate,
                    CosmosMsg::Wasm(WasmMsg::Instantiate { .. })
                )
                | (
                    MsgType::WasmMigrate,
                    CosmosMsg::Wasm(WasmMsg::Migrate { .. })
                )
                | (
                    MsgType::WasmUpdateAdmin,
                    CosmosMsg::Wasm(WasmMsg::UpdateAdmin { .. })
                )
                | (
                    MsgType::WasmClearAdmin,
                    CosmosMsg::Wasm(WasmMsg::ClearAdmin { .. })
                )
                | (MsgType::GovVote, CosmosMsg::Gov(GovMsg::Vote { .. }))
        )
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
}

fn check_spending_limits(
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
    // Create HashMaps for both actual and limit coins
    let actual_map: HashMap<_, _> = actual.iter().map(|c| (&c.denom, c.amount)).collect();
    let limit_map: HashMap<_, _> = limit.iter().map(|c| (&c.denom, c.amount)).collect();

    // Check if all limit coins are satisfied by the actual coins
    for (denom, &limit_amount) in limit_map.iter() {
        let actual_amount = actual_map.get(denom).cloned().unwrap_or(Uint128::zero());
        if actual_amount > limit_amount || (limit_amount.is_zero() && !actual_amount.is_zero()) {
            return false;
        }
    }

    // Check if there are any actual coins not present in the limit
    for (denom, &actual_amount) in actual_map.iter() {
        if !limit_map.contains_key(denom)
            || (!actual_amount.is_zero() && limit_map[denom].is_zero())
        {
            return false;
        }
    }

    true
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

    #[test]
    fn test_valid_config() {
        let filter = MsgFilter {
            allowed_msgs: Some(vec![AllowedMsg::Type(MsgType::BankSend)]),
            max_msg_count: Some(5),
            spending_limits: Some(coins(100, "utoken")),
        };
        assert!(filter.validate_config().is_ok());
    }

    #[test]
    fn test_no_filtering_criteria() {
        let filter = MsgFilter {
            allowed_msgs: None,
            max_msg_count: None,
            spending_limits: None,
        };
        assert!(matches!(
            filter.validate_config(),
            Err(MsgFilterError::NoFilteringCriteria)
        ));
    }

    #[test]
    fn test_invalid_max_msg_count() {
        let filter = MsgFilter {
            allowed_msgs: None,
            max_msg_count: Some(0),
            spending_limits: None,
        };
        assert!(matches!(
            filter.validate_config(),
            Err(MsgFilterError::InvalidMaxMsgCount(0))
        ));
    }

    #[test]
    fn test_invalid_spending_limit() {
        let filter = MsgFilter {
            allowed_msgs: None,
            max_msg_count: None,
            spending_limits: Some(vec![coin(0, "utoken")]),
        };
        assert!(matches!(
            filter.validate_config(),
            Err(MsgFilterError::InvalidSpendingLimit { denom }) if denom == "utoken"
        ));
    }

    #[test]
    fn test_empty_generic_wasm_execute_msg() {
        let filter = MsgFilter {
            allowed_msgs: Some(vec![AllowedMsg::GenericWasmExecuteMsg {
                contract: None,
                key: None,
                funds: None,
            }]),
            max_msg_count: None,
            spending_limits: None,
        };
        assert!(matches!(
            filter.validate_config(),
            Err(MsgFilterError::EmptyGenericWasmExecuteMsg)
        ));
    }

    #[test]
    fn test_invalid_coin_amount_in_generic_wasm_execute_msg() {
        let filter = MsgFilter {
            allowed_msgs: Some(vec![AllowedMsg::GenericWasmExecuteMsg {
                contract: Some("contract".to_string()),
                key: None,
                funds: Some(vec![coin(0, "utoken")]),
            }]),
            max_msg_count: None,
            spending_limits: None,
        };
        assert!(matches!(
            filter.validate_config(),
            Err(MsgFilterError::InvalidCoinAmount { denom }) if denom == "utoken"
        ));
    }

    #[test]
    fn test_msg_type_filtering() {
        let filter = MsgFilter {
            allowed_msgs: Some(vec![AllowedMsg::Type(MsgType::BankSend)]),
            max_msg_count: None,
            spending_limits: None,
        };

        let allowed_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: "recipient".to_string(),
            amount: vec![coin(100, "utoken")],
        });
        assert!(filter.is_message_allowed(&allowed_msg).unwrap());

        let disallowed_msg = CosmosMsg::Staking(StakingMsg::Delegate {
            validator: "validator".to_string(),
            amount: coin(100, "utoken"),
        });
        assert!(!filter.is_message_allowed(&disallowed_msg).unwrap());
    }

    #[test]
    fn test_compare_funds() {
        // Basic case
        assert!(compare_funds(
            &[coin(100, "utoken")],
            &[coin(100, "utoken")]
        ));
        assert!(compare_funds(&[coin(50, "utoken")], &[coin(100, "utoken")]));
        assert!(!compare_funds(
            &[coin(150, "utoken")],
            &[coin(100, "utoken")]
        ));

        // Multiple denominations
        assert!(compare_funds(
            &[coin(50, "utoken"), coin(30, "ustake")],
            &[coin(100, "utoken"), coin(50, "ustake")]
        ));
        assert!(!compare_funds(
            &[coin(150, "utoken"), coin(30, "ustake")],
            &[coin(100, "utoken"), coin(50, "ustake")]
        ));

        // Denominations in actual not in limit
        assert!(!compare_funds(
            &[coin(50, "utoken"), coin(30, "ustake")],
            &[coin(100, "utoken")]
        ));

        // Denominations in limit not in actual
        assert!(compare_funds(
            &[coin(50, "utoken")],
            &[coin(100, "utoken"), coin(50, "ustake")]
        ));

        // Empty slices
        assert!(compare_funds(&[], &[]));
        assert!(compare_funds(&[], &[coin(100, "utoken")]));
        assert!(!compare_funds(&[coin(100, "utoken")], &[]));

        // Zero amounts
        assert!(compare_funds(&[coin(0, "utoken")], &[coin(100, "utoken")]));
        assert!(!compare_funds(&[coin(100, "utoken")], &[coin(0, "utoken")]));
        assert!(compare_funds(&[coin(0, "utoken")], &[coin(0, "utoken")]));
    }
}
