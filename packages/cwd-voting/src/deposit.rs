use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, Deps, MessageInfo, StdError, StdResult, Uint128, WasmMsg,
};
use cw_utils::{must_pay, PaymentError};

use thiserror::Error;

use cw_denom::{CheckedDenom, DenomError, UncheckedDenom};

/// Error type for deposit methods.
#[derive(Error, Debug, PartialEq)]
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
#[cw_serde]
pub enum DepositToken {
    /// Use a specific token address as the deposit token.
    Token { denom: UncheckedDenom },
    /// Use the token address of the associated DAO's voting
    /// module. NOTE: in order to use the token address of the voting
    /// module the voting module must (1) use a cw20 token and (2)
    /// implement the `TokenContract {}` query type defined by
    /// `cwd_macros::token_query`. Failing to implement that
    /// and using this option will cause instantiation to fail.
    VotingModuleToken {},
}

/// Information about the deposit required to create a proposal.
#[cw_serde]
pub struct UncheckedDepositInfo {
    /// The address of the token to be used for proposal deposits.
    pub denom: DepositToken,
    /// The number of tokens that must be deposited to create a
    /// proposal. Must be a positive, non-zero number.
    pub amount: Uint128,
    /// The policy used for refunding deposits on proposal completion.
    pub refund_policy: DepositRefundPolicy,
}

#[cw_serde]
pub enum DepositRefundPolicy {
    /// Deposits should always be refunded.
    Always,
    /// Deposits should only be refunded for passed proposals.
    OnlyPassed,
    /// Deposits should never be refunded.
    Never,
}

/// Counterpart to the `DepositInfo` struct which has been
/// processed. This type should never be constructed literally and
/// should always by built by calling `into_checked` on a
/// `DepositInfo` instance.
#[cw_serde]
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
                    .query_wasm_smart(dao, &cwd_core::msg::QueryMsg::VotingModule {})?;
                // If the voting module has no token this will
                // error. This is desirable.
                let token_addr: Addr = deps.querier.query_wasm_smart(
                    voting_module,
                    &cwd_interface::voting::Query::TokenContract {},
                )?;
                // We don't assume here that the voting module has
                // returned a valid token. Conversion of the unchecked
                // denom into a checked one will do a `TokenInfo {}`
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
            // must_pay > may_pay. The method this is getting called
            // in is accepting a deposit. It seems likely to me that
            // if other payments are here it's a bug in a frontend and
            // not an intentional thing.
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

    pub fn get_return_deposit_message(&self, depositor: &Addr) -> StdResult<Vec<CosmosMsg>> {
        // Should get caught in `into_checked()`, but to be pedantic.
        if self.amount.is_zero() {
            return Ok(vec![]);
        }
        let message = self.denom.get_transfer_to_message(depositor, self.amount)?;
        Ok(vec![message])
    }
}

#[cfg(test)]
pub mod tests {
    use cosmwasm_std::{coin, coins, testing::mock_info, BankMsg};

    use super::*;

    const NATIVE_DENOM: &str = "uekez";
    const CW20: &str = "cw20";

    #[test]
    fn test_check_native_deposit_paid_yes() {
        let info = mock_info("ekez", &coins(10, NATIVE_DENOM));
        let deposit_info = CheckedDepositInfo {
            denom: CheckedDenom::Native(NATIVE_DENOM.to_string()),
            amount: Uint128::new(10),
            refund_policy: DepositRefundPolicy::Always,
        };
        deposit_info.check_native_deposit_paid(&info).unwrap();

        let mut info = info;
        let mut deposit_info = deposit_info;

        // Doesn't matter what we submit if it's a cw20 token.
        info.funds = vec![];
        deposit_info.denom = CheckedDenom::Cw20(Addr::unchecked(CW20));
        deposit_info.check_native_deposit_paid(&info).unwrap();

        info.funds = coins(100, NATIVE_DENOM);
        deposit_info.check_native_deposit_paid(&info).unwrap();
    }

    #[test]
    fn test_native_deposit_paid_wrong_amount() {
        let info = mock_info("ekez", &coins(9, NATIVE_DENOM));
        let deposit_info = CheckedDepositInfo {
            denom: CheckedDenom::Native(NATIVE_DENOM.to_string()),
            amount: Uint128::new(10),
            refund_policy: DepositRefundPolicy::Always,
        };
        let err = deposit_info.check_native_deposit_paid(&info).unwrap_err();
        assert_eq!(
            err,
            DepositError::InvalidDeposit {
                actual: Uint128::new(9),
                expected: Uint128::new(10)
            }
        )
    }

    #[test]
    fn check_native_deposit_paid_wrong_denom() {
        let info = mock_info("ekez", &coins(10, "unotekez"));
        let deposit_info = CheckedDepositInfo {
            denom: CheckedDenom::Native(NATIVE_DENOM.to_string()),
            amount: Uint128::new(10),
            refund_policy: DepositRefundPolicy::Always,
        };
        let err = deposit_info.check_native_deposit_paid(&info).unwrap_err();
        assert_eq!(
            err,
            DepositError::Payment(PaymentError::MissingDenom(NATIVE_DENOM.to_string()))
        );
    }

    // If you're receiving other denoms in the same place you're
    // receiving your deposit you should probably write your own
    // package, or you're working on dao dao and can remove this
    // one. At the time of writing, other denoms coming in with a
    // deposit seems like a frontend bug off.
    #[test]
    fn check_sending_other_denoms_is_not_allowed() {
        let info = mock_info("ekez", &[coin(10, "unotekez"), coin(10, "ekez")]);
        let deposit_info = CheckedDepositInfo {
            denom: CheckedDenom::Native(NATIVE_DENOM.to_string()),
            amount: Uint128::new(10),
            refund_policy: DepositRefundPolicy::Always,
        };

        let err = deposit_info.check_native_deposit_paid(&info).unwrap_err();
        assert_eq!(err, DepositError::Payment(PaymentError::MultipleDenoms {}));
    }

    #[test]
    fn check_native_deposit_paid_no_denoms() {
        let info = mock_info("ekez", &[]);
        let deposit_info = CheckedDepositInfo {
            denom: CheckedDenom::Native(NATIVE_DENOM.to_string()),
            amount: Uint128::new(10),
            refund_policy: DepositRefundPolicy::Always,
        };
        let err = deposit_info.check_native_deposit_paid(&info).unwrap_err();
        assert_eq!(err, DepositError::Payment(PaymentError::NoFunds {}));
    }

    #[test]
    fn test_get_take_deposit_messages() {
        // Does nothing if a native token is being used.
        let mut deposit_info = CheckedDepositInfo {
            denom: CheckedDenom::Native(NATIVE_DENOM.to_string()),
            amount: Uint128::new(10),
            refund_policy: DepositRefundPolicy::Always,
        };
        let messages = deposit_info
            .get_take_deposit_messages(&Addr::unchecked("ekez"), &Addr::unchecked(CW20))
            .unwrap();
        assert_eq!(messages, vec![]);

        // Does something for cw20s.
        deposit_info.denom = CheckedDenom::Cw20(Addr::unchecked(CW20));
        let messages = deposit_info
            .get_take_deposit_messages(&Addr::unchecked("ekez"), &Addr::unchecked("contract"))
            .unwrap();
        assert_eq!(
            messages,
            vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: CW20.to_string(),
                msg: to_binary(&cw20::Cw20ExecuteMsg::TransferFrom {
                    owner: "ekez".to_string(),
                    recipient: "contract".to_string(),
                    amount: Uint128::new(10)
                })
                .unwrap(),
                funds: vec![],
            })]
        );

        // Does nothing when the amount is zero (this would cause the
        // tx to fail for a valid cw20).
        deposit_info.amount = Uint128::zero();
        let messages = deposit_info
            .get_take_deposit_messages(&Addr::unchecked("ekez"), &Addr::unchecked(CW20))
            .unwrap();
        assert_eq!(messages, vec![]);
    }

    #[test]
    fn test_get_return_deposit_message_native() {
        let mut deposit_info = CheckedDepositInfo {
            denom: CheckedDenom::Native(NATIVE_DENOM.to_string()),
            amount: Uint128::new(10),
            refund_policy: DepositRefundPolicy::Always,
        };
        let messages = deposit_info
            .get_return_deposit_message(&Addr::unchecked("ekez"))
            .unwrap();
        assert_eq!(
            messages,
            vec![CosmosMsg::Bank(BankMsg::Send {
                to_address: "ekez".to_string(),
                amount: coins(10, "uekez")
            })]
        );

        // Don't fire a message if there is nothing to send!
        deposit_info.amount = Uint128::zero();
        let messages = deposit_info
            .get_return_deposit_message(&Addr::unchecked("ekez"))
            .unwrap();
        assert_eq!(messages, vec![]);
    }

    #[test]
    fn test_get_return_deposit_message_cw20() {
        let mut deposit_info = CheckedDepositInfo {
            denom: CheckedDenom::Cw20(Addr::unchecked(CW20)),
            amount: Uint128::new(10),
            refund_policy: DepositRefundPolicy::Always,
        };
        let messages = deposit_info
            .get_return_deposit_message(&Addr::unchecked("ekez"))
            .unwrap();
        assert_eq!(
            messages,
            vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: CW20.to_string(),
                msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                    recipient: "ekez".to_string(),
                    amount: Uint128::new(10)
                })
                .unwrap(),
                funds: vec![]
            })]
        );

        // Don't fire a message if there is nothing to send!
        deposit_info.amount = Uint128::zero();
        let messages = deposit_info
            .get_return_deposit_message(&Addr::unchecked("ekez"))
            .unwrap();
        assert_eq!(messages, vec![]);
    }
}
