use cosmwasm_std::{Addr, CosmosMsg, Deps, StdError, StdResult, Uint128};
use cw_asset::{AssetBase, AssetInfo, AssetInfoBase, AssetInfoUnchecked};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Information about the token to use for proposal deposits.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DepositToken {
    /// Use a specific token address as the deposit token.
    Token { asset: AssetInfoUnchecked },
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
pub struct DepositInfo {
    /// The address of the cw20 token to be used for proposal
    /// deposits.
    pub token: DepositToken,
    /// The number of tokens that must be deposited to create a
    /// proposal.
    pub deposit: Uint128,
    /// If failed proposals should have their deposits refunded.
    pub refund_policy: DepositRefundPolicy,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DepositRefundPolicy {
    Always,
    OnlyPassed,
    Never,
}

/// Counterpart to the `DepositInfo` struct which has been processed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CheckedDepositInfo {
    /// The address of the cw20 token to be used for proposal
    /// deposits.
    pub token: AssetInfo,
    /// The number of tokens that must be deposited to create a
    /// proposal.
    pub deposit: Uint128,
    /// If failed proposals should have their deposits refunded.
    pub refund_policy: DepositRefundPolicy,
}

impl DepositInfo {
    /// Converts deposit info into checked deposit info.
    pub fn into_checked(self, deps: Deps, dao: Addr) -> StdResult<CheckedDepositInfo> {
        let Self {
            token,
            deposit,
            refund_policy,
        } = self;
        let token = match token {
            DepositToken::Token { asset } => match asset.check(deps.api, None) {
                Ok(info) => Ok(info),
                Err(err) => Err(err),
            },
            DepositToken::VotingModuleToken {} => {
                let voting_module: Addr = deps
                    .querier
                    .query_wasm_smart(dao, &cw_core::msg::QueryMsg::VotingModule {})?;
                let token_addr: Addr = deps.querier.query_wasm_smart(
                    voting_module,
                    &cw_core_interface::voting::Query::TokenContract {},
                )?;
                Ok(AssetInfo::Cw20(token_addr))
            }
        }?;

        match token {
            AssetInfoBase::Native(ref _denom) => Ok(CheckedDepositInfo {
                token,
                deposit,
                refund_policy,
            }),
            AssetInfoBase::Cw20(ref addr) => {
                // Make an info query as a smoke test that we are indeed
                // working with a token here. We can't turbofish this
                // type. See <https://github.com/rust-lang/rust/issues/83701>
                let _info: cw20::TokenInfoResponse = deps
                    .querier
                    .query_wasm_smart(addr, &cw20::Cw20QueryMsg::TokenInfo {})?;
                Ok(CheckedDepositInfo {
                    token,
                    deposit,
                    refund_policy,
                })
            }
            _ => Err(StdError::GenericErr {
                msg: String::from("Unsupported asset"),
            }),
        }
    }
}

pub fn get_deposit_msg(
    info: &Option<CheckedDepositInfo>,
    contract: &Addr,
    sender: &Addr,
) -> StdResult<Vec<CosmosMsg>> {
    match info {
        Some(info) => {
            if info.deposit.is_zero() {
                Ok(vec![])
            } else {
                let asset = AssetBase::new(info.token.clone(), info.deposit);
                let transfer_msg = match asset.info {
                    AssetInfoBase::Cw20(ref _addr) => asset.transfer_from_msg(sender, contract),
                    AssetInfoBase::Native(ref _denom) => asset.transfer_msg(contract),
                    _ => Err(StdError::GenericErr {
                        msg: String::from("Unsupported asset"),
                    }),
                }?;
                Ok(vec![transfer_msg])
            }
        }
        None => Ok(vec![]),
    }
}

pub fn get_return_deposit_msg(
    deposit_info: &CheckedDepositInfo,
    receiver: &Addr,
) -> StdResult<Vec<CosmosMsg>> {
    if deposit_info.deposit.is_zero() {
        return Ok(vec![]);
    }
    let asset = AssetBase::new(deposit_info.token.clone(), deposit_info.deposit);
    let transfer_msg = asset.transfer_msg(receiver)?;
    Ok(vec![transfer_msg])
}
