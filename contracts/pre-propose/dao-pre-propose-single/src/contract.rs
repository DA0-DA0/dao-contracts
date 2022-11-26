use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;

use dao_pre_propose_base::{
    error::PreProposeError,
    msg::{ExecuteMsg as ExecuteBase, InstantiateMsg as InstantiateBase, QueryMsg as QueryBase},
    state::PreProposeContract,
};
use dao_voting::proposal::SingleChoiceProposeMsg as ProposeMsg;

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-pre-propose-single";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cw_serde]
pub enum ProposeMessage {
    /// The propose message used to make a proposal to this
    /// module. Note that this is identical to the propose message
    /// used by dao-proposal-single, except that it omits the
    /// `proposer` field which it fills in for the sender.
    Propose {
        title: String,
        description: String,
        msgs: Vec<CosmosMsg<Empty>>,
    },
}

pub type InstantiateMsg = InstantiateBase<Empty>;
pub type ExecuteMsg = ExecuteBase<ProposeMessage, Empty>;
pub type QueryMsg = QueryBase<Empty>;

/// Internal version of the propose message that includes the
/// `proposer` field. The module will fill this in based on the sender
/// of the external message.
#[cw_serde]
enum ProposeMessageInternal {
    Propose(ProposeMsg),
}

type PrePropose = PreProposeContract<Empty, Empty, Empty, ProposeMessageInternal>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, PreProposeError> {
    let resp = PrePropose::default().instantiate(deps.branch(), env, info, msg)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, PreProposeError> {
    // We don't want to expose the `proposer` field on the propose
    // message externally as that is to be set by this module. Here,
    // we transform an external message which omits that field into an
    // internal message which sets it.
    type ExecuteInternal = ExecuteBase<ProposeMessageInternal, Empty>;
    let internalized = match msg {
        ExecuteMsg::Propose {
            msg:
                ProposeMessage::Propose {
                    title,
                    description,
                    msgs,
                },
        } => ExecuteInternal::Propose {
            msg: ProposeMessageInternal::Propose(ProposeMsg {
                // Fill in proposer based on message sender.
                proposer: Some(info.sender.to_string()),
                title,
                description,
                msgs,
            }),
        },
        ExecuteMsg::Extension { msg } => ExecuteInternal::Extension { msg },
        ExecuteMsg::Withdraw { denom } => ExecuteInternal::Withdraw { denom },
        ExecuteMsg::UpdateConfig {
            deposit_info,
            open_proposal_submission,
        } => ExecuteInternal::UpdateConfig {
            deposit_info,
            open_proposal_submission,
        },
        ExecuteMsg::AddProposalSubmittedHook { address } => {
            ExecuteInternal::AddProposalSubmittedHook { address }
        }
        ExecuteMsg::RemoveProposalSubmittedHook { address } => {
            ExecuteInternal::RemoveProposalSubmittedHook { address }
        }
        ExecuteMsg::ProposalCompletedHook {
            proposal_id,
            new_status,
        } => ExecuteInternal::ProposalCompletedHook {
            proposal_id,
            new_status,
        },
    };

    PrePropose::default().execute(deps, env, info, internalized)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    PrePropose::default().query(deps, env, msg)
}
