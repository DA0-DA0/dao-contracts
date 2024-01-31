use cosmwasm_std::{Attribute, CosmosMsg, DepsMut, Env, MessageInfo, Response};
use cw20::Cw20ReceiveMsg;
use cw_ownable::{assert_owner, get_ownership};
use dao_hooks::proposal::ProposalHookMsg;
use dao_interface::{proposal::GenericProposalInfo, state::ProposalModule};
use dao_voting::status::Status;

use crate::{msg::ProposalIncentivesUnchecked, state::PROPOSAL_INCENTIVES, ContractError};

pub fn proposal_hook(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ProposalHookMsg,
) -> Result<Response, ContractError> {
    let mut msgs: Vec<CosmosMsg> = vec![];
    let mut attrs: Vec<Attribute> = vec![];

    // Get ownership
    let ownership = get_ownership(deps.storage)?;

    if let Some(owner) = ownership.owner {
        // Validate the message is coming from a proposal module of the owner (DAO)
        deps.querier.query_wasm_smart::<ProposalModule>(
            owner,
            &dao_interface::msg::QueryMsg::ProposalModule {
                address: info.sender.to_string(),
            },
        )?;

        // Check prop status and type of hook

        if let ProposalHookMsg::ProposalStatusChanged { id, new_status, .. } = msg {
            // If prop status is success, add message to pay out rewards
            // Otherwise, do nothing
            if new_status == Status::Passed.to_string() {
                // Query for the proposal
                let proposal_info: GenericProposalInfo = deps.querier.query_wasm_smart(
                    info.sender,
                    &dao_interface::proposal::Query::GenericProposalInfo { proposal_id: id },
                )?;

                // Load proposal incentives config
                let proposal_incentives = PROPOSAL_INCENTIVES
                    .may_load_at_height(deps.storage, proposal_info.start_height)?;

                // Append the message if found
                if let Some(proposal_incentives) = proposal_incentives {
                    msgs.push(proposal_incentives.denom.get_transfer_to_message(
                        &proposal_info.proposer,
                        proposal_incentives.rewards_per_proposal,
                    )?);
                    attrs = proposal_incentives.into_attributes();
                    attrs.push(Attribute {
                        key: "proposer".to_string(),
                        value: proposal_info.proposer.to_string(),
                    });
                }
            }
        }
    }

    Ok(Response::default()
        .add_attribute("action", "proposal_hook")
        .add_attributes(attrs)
        .add_messages(msgs))
}

pub fn update_ownership(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    action: cw_ownable::Action,
) -> Result<Response, ContractError> {
    let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;

    Ok(Response::new()
        .add_attribute("action", "update_ownership")
        .add_attributes(ownership.into_attributes()))
}

pub fn update_proposal_incentives(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_incentives: ProposalIncentivesUnchecked,
) -> Result<Response, ContractError> {
    assert_owner(deps.storage, &info.sender)?;

    // Validate proposal incentives
    let proposal_incentives = proposal_incentives.into_checked(deps.as_ref())?;

    // Save the new proposal incentives
    PROPOSAL_INCENTIVES.save(deps.storage, &proposal_incentives, env.block.height)?;

    Ok(Response::new()
        .add_attribute("action", "update_proposal_incentives")
        .add_attributes(proposal_incentives.into_attributes()))
}

pub fn receive_cw20(
    _deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _cw20_receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    Ok(Response::new()
        .add_attribute("action", "receive_cw20")
        .add_attribute("cw20", info.sender))
}
