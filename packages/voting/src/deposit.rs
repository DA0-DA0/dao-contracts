use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, CosmosMsg, Deps, MessageInfo, StdError, StdResult, Uint128,
    WasmMsg,
};
use cw_utils::{must_pay, PaymentError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::denom::{CheckedDenom, DenomError, UncheckedDenom};

/// Error type for deposit methods.
#[derive(Error, Debug)]
pub enum DepositError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Payment(#[from] PaymentError),

    #[error(transparent)]
    Denom(#[from] DenomError),

    #[error("invalid zero deposit. set the deposit to `None` to have no deposit")]
    ZeroDeposit,

    #[error("invalid deposit amount. got ({actual}), expected ({expected})")]
    InvalidDeposit { actual: Uint128, expected: Uint128 },
}

/// Information about the token to use for proposal deposits.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DepositToken {
    /// Use a specific token address as the deposit token.
    Token { denom: UncheckedDenom },
    /// Use the token address of the associated DAO's voting
    /// module. NOTE: in order to use the token address of the voting
    /// module the voting module must (1) use a cw20 token and (2)
    /// implement the `TokenContract {}` query type defined by
    /// `cw_core_macros::token_query`. Failing to implement that
    /// and using this option will cause instantiation to fail.
    VotingModuleToken {},
}

/// Information about the deposit required to create a proposal.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UncheckedDepositInfo {
    /// The address of the cw20 token to be used for proposal
    /// deposits.
    pub denom: DepositToken,
    /// The number of tokens that must be deposited to create a
    /// proposal. Must be a positive, non-zero number.
    pub amount: Uint128,
    /// If failed proposals should have their deposits refunded.
    pub refund_policy: DepositRefundPolicy,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DepositRefundPolicy {
    /// Deposits should always be refunded.
    Always,
    /// Deposits should only be refunded for passed proposals.
    OnlyPassed,
    /// Deposits should never be refunded.
    Never,
}

/// Counterpart to the `DepositInfo` struct which has been processed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CheckedDepositInfo {
    /// The address of the cw20 token to be used for proposal
    /// deposits.
    pub denom: CheckedDenom,
    /// The number of tokens that must be deposited to create a
    /// proposal. This is validated to be non-zero if this struct is
    /// constructed by converted via the `into_checked` method on
    /// `DepositInfo`.
    pub amount: Uint128,
    /// The policy used for refunding proposal deposits.
    pub refund_policy: DepositRefundPolicy,
}

impl UncheckedDepositInfo {
    /// Converts deposit info into checked deposit info.
    pub fn into_checked(self, deps: Deps, dao: Addr) -> Result<CheckedDepositInfo, DepositError> {
        let Self {
            denom,
            amount,
            refund_policy,
        } = self;
        // Check that the deposit is non-zero. Modules should make
        // deposit information optional and consumers should provide
        // `None` when they do not want to have a proposal deposit.
        if amount.is_zero() {
            return Err(DepositError::ZeroDeposit);
        }

        let denom = match denom {
            DepositToken::Token { denom } => denom.into_checked(deps),
            DepositToken::VotingModuleToken {} => {
                let voting_module: Addr = deps
                    .querier
                    .query_wasm_smart(dao, &cw_core::msg::QueryMsg::VotingModule {})?;
                // If the voting module has no token this will
                // error. This is desirable.
                let token_addr: Addr = deps.querier.query_wasm_smart(
                    voting_module,
                    &cw_core_interface::voting::Query::TokenContract {},
                )?;
                // We don't assume here that the voting module has
                // returned a valid token. Conversion of the unchecked
                // denom into a checked one will to a `TokenInfo {}`
                // query.
                UncheckedDenom::Cw20(token_addr.into_string()).into_checked(deps)
            }
        }?;

        Ok(CheckedDepositInfo {
            denom,
            amount,
            refund_policy,
        })
    }
}

impl CheckedDepositInfo {
    pub fn check_native_deposit_paid(&self, info: &MessageInfo) -> Result<(), DepositError> {
        if let Self {
            amount,
            denom: CheckedDenom::Native(denom),
            ..
        } = self
        {
            let paid = must_pay(info, denom)?;
            if paid != *amount {
                Err(DepositError::InvalidDeposit {
                    actual: paid,
                    expected: *amount,
                })
            } else {
                Ok(())
            }
        } else {
            // Nothing to do if we're a cw20.
            Ok(())
        }
    }

    pub fn get_take_deposit_messages(
        &self,
        depositor: &Addr,
        contract: &Addr,
    ) -> StdResult<Vec<CosmosMsg>> {
        let take_deposit_msg: Vec<CosmosMsg> = if let Self {
            amount,
            denom: CheckedDenom::Cw20(address),
            ..
        } = self
        {
            // into_checked() makes sure this isn't the case, but just for
            // posterity.
            if amount.is_zero() {
                vec![]
            } else {
                vec![WasmMsg::Execute {
                    contract_addr: address.to_string(),
                    funds: vec![],
                    msg: to_binary(&cw20::Cw20ExecuteMsg::TransferFrom {
                        owner: depositor.to_string(),
                        recipient: contract.to_string(),
                        amount: *amount,
                    })?,
                }
                .into()]
            }
        } else {
            // Deposits are pushed, not pulled for native
            // deposits. See: `check_native_deposit_paid`.
            vec![]
        };
        Ok(take_deposit_msg)
    }

    pub fn get_return_deposit_message(&self, depositor: &Addr) -> StdResult<CosmosMsg> {
        let message = match &self.denom {
            CheckedDenom::Native(denom) => BankMsg::Send {
                to_address: depositor.to_string(),
                amount: vec![Coin {
                    amount: self.amount,
                    denom: denom.to_string(),
                }],
            }
            .into(),
            CheckedDenom::Cw20(address) => WasmMsg::Execute {
                contract_addr: address.to_string(),
                msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                    recipient: depositor.to_string(),
                    amount: self.amount,
                })?,
                funds: vec![],
            }
            .into(),
        };
        Ok(message)
    }
}
