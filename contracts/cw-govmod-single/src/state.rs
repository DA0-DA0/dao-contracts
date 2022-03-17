use cosmwasm_std::{to_binary, Addr, CosmosMsg, Deps, StdResult, Uint128, WasmMsg};
use cw_storage_plus::{Item, Map};
use cw_utils::Duration;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    msg::{DepositInfo, DepositToken},
    proposal::{Proposal, Vote},
    threshold::Threshold,
};

/// Counterpart to the `DepositInfo` struct which has been processed.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
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

/// The governance module's configuration.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// The threshold a proposal must reach to complete.
    pub threshold: Threshold,
    /// The default maximum amount of time a proposal may be voted on
    /// before expiring.
    pub max_voting_period: Duration,
    /// If set to true only members may execute passed
    /// proposals. Otherwise, any address may execute a passed
    /// proposal.
    pub only_members_execute: bool,
    /// The address of the DAO that this governance module is
    /// associated with.
    pub dao: Addr,
    /// Information about the depost required to create a
    /// proposal. None if no deposit is required, Some otherwise.
    pub deposit_info: Option<CheckedDepositInfo>,
}

/// A vote cast for a proposal.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Ballot {
    /// The amount of voting power behind the vote.
    pub power: Uint128,
    /// The position.
    pub vote: Vote,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const PROPOSAL_COUNT: Item<u64> = Item::new("proposal_count");
pub const PROPOSALS: Map<u64, Proposal> = Map::new("proposals");
pub const BALLOTS: Map<(u64, Addr), Ballot> = Map::new("ballots");

impl DepositInfo {
    /// Converts deposit info into checked deposit info.
    pub fn into_checked(self, deps: Deps, dao: Addr) -> StdResult<CheckedDepositInfo> {
        let Self {
            token,
            deposit,
            refund_failed_proposals,
        } = self;
        let token = match token {
            DepositToken::Token(token) => deps.api.addr_validate(&token)?,
            DepositToken::VotingModuleToken => {
                let voting_module: Addr = deps
                    .querier
                    .query_wasm_smart(dao, &cw_governance::msg::QueryMsg::VotingModule {})?;
                let token_addr: Addr = deps.querier.query_wasm_smart(
                    voting_module,
                    &cw_governance_interface::voting::Query::TokenContract {},
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

pub fn get_return_deposit_msg(proposal: &Proposal) -> StdResult<Vec<CosmosMsg>> {
    match &proposal.deposit_info {
        Some(info) => {
            let transfer_msg = WasmMsg::Execute {
                contract_addr: info.token.to_string(),
                funds: vec![],
                msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                    recipient: proposal.proposer.to_string(),
                    amount: info.deposit,
                })?,
            };
            let transfer_msg: CosmosMsg = transfer_msg.into();
            Ok(vec![transfer_msg])
        }
        None => Ok(vec![]),
    }
}
