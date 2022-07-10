use cosmwasm_std::{to_binary, Addr, CosmosMsg, Deps, StdResult, Uint128, WasmMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Information about the token to use for proposal deposits.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DepositToken {
    /// Use a specific token address as the deposit token.
    Token { address: String },
    /// Use the token address of the associated DAO's voting
    /// module. NOTE: in order to use the token address of the voting
    /// module the voting module must (1) use a cw20 token and (2)
    /// implement the `TokenContract {}` query type defined by
    /// `cw_core_macros::token_query`. Failing to implement that
    /// and using this option will cause instantiation to fail.
    VotingModuleToken {},
}

/// Information about the deposit required to create a proposal.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct DepositInfo {
    /// The address of the cw20 token to be used for proposal
    /// deposits.
    pub token: DepositToken,
    /// The number of tokens that must be deposited to create a
    /// proposal.
    pub deposit: Uint128,
    /// If failed proposals should have their deposits refunded.
    pub refund_failed_proposals: bool,
}

/// Counterpart to the `DepositInfo` struct which has been processed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CheckedDepositInfo {
    /// The address of the cw20 token to be used for proposal
    /// deposits.
    pub token: Addr,
    /// The number of tokens that must be deposited to create a
    /// proposal.
    pub deposit: Uint128,
    /// If failed proposals should have their deposits refunded.
    pub refund_failed_proposals: bool,
}

impl DepositInfo {
    /// Converts deposit info into checked deposit info.
    pub fn into_checked(self, deps: Deps, dao: Addr) -> StdResult<CheckedDepositInfo> {
        let Self {
            token,
            deposit,
            refund_failed_proposals,
        } = self;
        let token = match token {
            DepositToken::Token { address } => deps.api.addr_validate(&address)?,
            DepositToken::VotingModuleToken {} => {
                let voting_module: Addr = deps
                    .querier
                    .query_wasm_smart(dao, &cw_core::msg::QueryMsg::VotingModule {})?;
                let token_addr: Addr = deps.querier.query_wasm_smart(
                    voting_module,
                    &cw_core_interface::voting::Query::TokenContract {},
                )?;
                token_addr
            }
        };
        // Make an info query as a smoke test that we are indeed
        // working with a token here. We can't turbofish this
        // type. See <https://github.com/rust-lang/rust/issues/83701>
        let _info: cw20::TokenInfoResponse = deps
            .querier
            .query_wasm_smart(token.clone(), &cw20::Cw20QueryMsg::TokenInfo {})?;
        Ok(CheckedDepositInfo {
            token,
            deposit,
            refund_failed_proposals,
        })
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
                let transfer_msg = WasmMsg::Execute {
                    contract_addr: info.token.to_string(),
                    funds: vec![],
                    msg: to_binary(&cw20::Cw20ExecuteMsg::TransferFrom {
                        owner: sender.to_string(),
                        recipient: contract.to_string(),
                        amount: info.deposit,
                    })?,
                };
                let transfer_msg: CosmosMsg = transfer_msg.into();
                Ok(vec![transfer_msg])
            }
        }
        None => Ok(vec![]),
    }
}

pub fn get_return_deposit_msg(
    deposit_info: &CheckedDepositInfo,
    proposer: &Addr,
) -> StdResult<Vec<CosmosMsg>> {
    if deposit_info.deposit.is_zero() {
        return Ok(vec![]);
    }
    let transfer_msg = WasmMsg::Execute {
        contract_addr: deposit_info.token.to_string(),
        funds: vec![],
        msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
            recipient: proposer.to_string(),
            amount: deposit_info.deposit,
        })?,
    };
    let transfer_msg: CosmosMsg = transfer_msg.into();
    Ok(vec![transfer_msg])
}
