use cosmwasm_std::{Attribute, DepsMut, Env, MessageInfo, Response, StdResult, Uint128};
use cw20::Cw20ReceiveMsg;
use cw_denom::CheckedDenom;
use cw_ownable::get_ownership;
use dao_hooks::vote::VoteHookMsg;
use dao_interface::{
    proposal::GenericProposalInfo,
    state::{ProposalModule, ProposalModuleStatus},
};

use crate::{
    state::{reward, CONFIG, GENERIC_PROPOSAL_INFO, USER_PROPOSAL_HAS_VOTED, USER_VOTE_COUNT},
    ContractError,
};

pub fn claim(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    // Ensure the user has something to claim
    if !USER_VOTE_COUNT.has(deps.storage, &info.sender) {
        return Err(ContractError::NothingToClaim {});
    }

    // Get reward information
    let reward = reward(deps.as_ref(), &env.contract.address, &info.sender)?;

    // If the user has rewards, then we should generate a message
    let mut msgs = vec![];
    if !reward.amount.is_zero() {
        msgs.push(
            reward
                .denom
                .get_transfer_to_message(&info.sender, reward.amount)?,
        );
    }

    // Clean state
    USER_VOTE_COUNT.remove(deps.storage, &info.sender);

    Ok(Response::new()
        .add_attribute("action", "claim")
        .add_attribute("denom", reward.denom.to_string())
        .add_attribute("amount", reward.amount)
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

pub fn vote_hook(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: VoteHookMsg,
) -> Result<Response, ContractError> {
    let mut attrs: Vec<Attribute> = vec![];

    // Get ownership
    let ownership = get_ownership(deps.storage)?;

    if let Some(owner) = ownership.owner {
        // Validate the message is coming from a proposal module of the owner (DAO)
        let proposal_module = deps.querier.query_wasm_smart::<ProposalModule>(
            owner,
            &dao_interface::msg::QueryMsg::ProposalModule {
                address: info.sender.to_string(),
            },
        )?;

        // If the proposal module is disabled, then return error
        if proposal_module.status == ProposalModuleStatus::Disabled {
            return Err(ContractError::ProposalModuleIsInactive {});
        }

        // Check type of hook
        match msg {
            VoteHookMsg::NewVote {
                proposal_id, voter, ..
            } => {
                if let Ok(voter) = deps.api.addr_validate(&voter) {
                    // Get config
                    let mut config = CONFIG.load(deps.storage)?;

                    // Check if the voting incentives have expired
                    if config.expiration.is_expired(&env.block) {
                        return Err(ContractError::AlreadyExpired {});
                    }

                    // Get the proposal info
                    // If we have a value in the cache, then return the value
                    // If we don't have a value, then query for the value and set it in the cache
                    let proposal_info = if GENERIC_PROPOSAL_INFO
                        .has(deps.storage, (&info.sender, proposal_id))
                    {
                        GENERIC_PROPOSAL_INFO.load(deps.storage, (&info.sender, proposal_id))?
                    } else {
                        let proposal_info: GenericProposalInfo = deps.querier.query_wasm_smart(
                            info.sender.clone(),
                            &dao_interface::proposal::Query::GenericProposalInfo { proposal_id },
                        )?;

                        GENERIC_PROPOSAL_INFO.save(
                            deps.storage,
                            (&info.sender, proposal_id),
                            &proposal_info,
                        )?;

                        proposal_info
                    };

                    // Check if the vote came from a proposal at or after the start of the voting incentives
                    if proposal_info.start_height >= config.start_height {
                        // Check if the user has already voted for the proposal
                        if !USER_PROPOSAL_HAS_VOTED.has(deps.storage, (&voter, proposal_id)) {
                            // Increment counts
                            let user_votes = USER_VOTE_COUNT.update(
                                deps.storage,
                                &voter,
                                |x| -> StdResult<Uint128> {
                                    Ok(x.unwrap_or_default().checked_add(Uint128::one())?)
                                },
                            )?;
                            config.total_votes = config.total_votes.checked_add(Uint128::one())?;
                            CONFIG.save(deps.storage, &config)?;

                            // Set has voted
                            USER_PROPOSAL_HAS_VOTED.save(
                                deps.storage,
                                (&voter, proposal_id),
                                &true,
                            )?;

                            // Set attributes
                            attrs = vec![
                                Attribute {
                                    key: "total_votes".to_string(),
                                    value: config.total_votes.to_string(),
                                },
                                Attribute {
                                    key: "user_votes".to_string(),
                                    value: user_votes.to_string(),
                                },
                                Attribute {
                                    key: "user".to_string(),
                                    value: voter.to_string(),
                                },
                            ];
                        }
                    }
                }
            }
        }
    }

    Ok(Response::new()
        .add_attribute("action", "vote_hook")
        .add_attributes(attrs))
}

pub fn expire(deps: DepsMut, env: Env, _info: MessageInfo) -> Result<Response, ContractError> {
    // Get the config
    let mut config = CONFIG.load(deps.storage)?;

    // If already expired, then return an error
    if config.expiration_balance.is_some() {
        return Err(ContractError::AlreadyExpired {});
    }

    // Ensure the voting incentives period has passed expiration
    if !config.expiration.is_expired(&env.block) {
        return Err(ContractError::NotExpired {
            expiration: config.expiration,
        });
    }

    // Get the available balance to distribute
    let balance = config
        .denom
        .query_balance(&deps.querier, &env.contract.address)?;

    // Save the balance
    config.expiration_balance = Some(balance);
    CONFIG.save(deps.storage, &config)?;

    // If no votes have occurred, then funds should be sent to the owner
    let mut msgs = vec![];
    if USER_VOTE_COUNT.is_empty(deps.storage) {
        let ownership = get_ownership(deps.storage)?;

        if let Some(owner) = ownership.owner {
            msgs.push(config.denom.get_transfer_to_message(&owner, balance)?);
        }
    }

    // Clean state
    GENERIC_PROPOSAL_INFO.clear(deps.storage);
    USER_PROPOSAL_HAS_VOTED.clear(deps.storage);

    Ok(Response::new()
        .add_attribute("action", "expire")
        .add_attribute("balance", balance)
        .add_messages(msgs))
}

pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _cw20_receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Do not accept unexpected cw20
    if config.expiration.is_expired(&env.block) {
        return Err(ContractError::AlreadyExpired {});
    }
    match &config.denom {
        CheckedDenom::Native(_) => {
            return Err(ContractError::UnexpectedFunds {
                expected: config.denom,
                received: CheckedDenom::Cw20(info.sender),
            })
        }
        CheckedDenom::Cw20(expected_cw20) => {
            if expected_cw20 != info.sender {
                return Err(ContractError::UnexpectedFunds {
                    expected: config.denom,
                    received: CheckedDenom::Cw20(info.sender),
                });
            }
        }
    }

    Ok(Response::new()
        .add_attribute("action", "receive_cw20")
        .add_attribute("cw20", info.sender))
}
