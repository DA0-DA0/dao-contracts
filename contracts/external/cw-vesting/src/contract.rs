#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_json, to_json_binary, Binary, Coin, CosmosMsg, DelegationResponse, Deps, DepsMut,
    DistributionMsg, Env, MessageInfo, Response, StakingMsg, StakingQuery, StdResult, Timestamp,
    Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use cw_denom::CheckedDenom;
use cw_ownable::OwnershipError;
use cw_utils::{must_pay, nonpayable};
use dao_voting::deposit::DepositRefundPolicy;
use dao_voting::pre_propose::ProposalCreationPolicy;
use dao_voting::{proposal::SingleChoiceProposeMsg, voting::Vote};

use crate::error::ContractError;
use crate::msg::{DaoActionsMsg, ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg};
use crate::state::{DAO_STAKING_LIMITS, PAYMENT, UNBONDING_DURATION_SECONDS};
use crate::vesting::{Status, VestInit};

const CONTRACT_NAME: &str = "crates.io:cw-vesting";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    cw_ownable::initialize_owner(deps.storage, deps.api, msg.owner.as_deref())?;

    let denom = msg.denom.into_checked(deps.as_ref())?;
    let recipient = deps.api.addr_validate(&msg.recipient)?;
    let start_time = msg.start_time.unwrap_or(env.block.time);

    if start_time.plus_seconds(msg.vesting_duration_seconds) <= env.block.time {
        return Err(ContractError::Instavest);
    }

    let vest = PAYMENT.initialize(
        deps.storage,
        VestInit {
            total: msg.total,
            schedule: msg.schedule,
            start_time,
            duration_seconds: msg.vesting_duration_seconds,
            denom,
            recipient,
            title: msg.title,
            description: msg.description,
        },
    )?;
    UNBONDING_DURATION_SECONDS.save(deps.storage, &msg.unbonding_duration_seconds)?;

    let resp = match vest.denom {
        CheckedDenom::Native(ref denom) => {
            let sent = must_pay(&info, denom)?;
            if vest.total() != sent {
                return Err(ContractError::WrongFundAmount {
                    sent,
                    expected: vest.total(),
                });
            }
            PAYMENT.set_funded(deps.storage)?;

            // If the payment denomination is the same as the native
            // denomination, set the staking rewards receiver to the
            // payment receiver so that when they stake vested tokens
            // they receive the rewards.
            if denom.as_str() == deps.querier.query_bonded_denom()? {
                // Check if dao_staking is enabled. Throw an error if it is.
                if let Some(_) = msg.dao_staking {
                    return Err(ContractError::DaoStakingNotSupported {});
                }

                Some(CosmosMsg::Distribution(
                    DistributionMsg::SetWithdrawAddress {
                        address: vest.recipient.to_string(),
                    },
                ))
            } else {
                // Check if dao_staking is enabled, and save dao_staking limits if it is.
                if let Some(dao_staking) = msg.dao_staking {
                    DAO_STAKING_LIMITS.save(deps.storage, &dao_staking)?;
                }

                None
            }
        }
        CheckedDenom::Cw20(_) => {
            nonpayable(&info)?; // Funding happens in ExecuteMsg::Receive.
            None
        }
    };

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", msg.owner.unwrap_or_else(|| "None".to_string()))
        .add_messages(resp))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => execute_receive_cw20(env, deps, info, msg),
        ExecuteMsg::Distribute { amount } => execute_distribute(env, deps, amount),
        ExecuteMsg::Cancel {} => execute_cancel_vesting_payment(env, deps, info),
        ExecuteMsg::UpdateOwnership(action) => execute_update_owner(deps, info, env, action),
        ExecuteMsg::Delegate { validator, amount } => {
            execute_delegate(env, deps, info, validator, amount)
        }
        ExecuteMsg::Redelegate {
            src_validator,
            dst_validator,
            amount,
        } => execute_redelegate(env, deps, info, src_validator, dst_validator, amount),
        ExecuteMsg::Undelegate { validator, amount } => {
            execute_undelegate(env, deps, info, validator, amount)
        }
        ExecuteMsg::SetWithdrawAddress { address } => {
            execute_set_withdraw_address(deps, env, info, address)
        }
        ExecuteMsg::WithdrawDelegatorReward { validator } => execute_withdraw_rewards(validator),
        ExecuteMsg::WithdrawCanceledPayment { amount } => {
            execute_withdraw_canceled_payment(deps, env, amount)
        }
        ExecuteMsg::RegisterSlash {
            validator,
            time,
            amount,
            during_unbonding,
        } => execute_register_slash(deps, env, info, validator, time, amount, during_unbonding),
        ExecuteMsg::DaoActions(action) => match action {
            DaoActionsMsg::Stake {
                amount,
                staking_contract,
            } => execute_dao_stake(deps, env, info, amount, staking_contract),
            DaoActionsMsg::Unstake {
                amount,
                staking_contract,
            } => execute_dao_unstake(deps, env, info, amount, staking_contract),
            DaoActionsMsg::Vote {
                proposal_module,
                proposal_id,
                vote,
                rationale,
            } => execute_dao_vote(
                deps,
                env,
                info,
                proposal_module,
                proposal_id,
                vote,
                rationale,
            ),
            DaoActionsMsg::Propose {
                proposal,
                proposal_module,
            } => execute_dao_propose(deps, env, info, proposal_module, proposal),
        },
    }
}

pub fn execute_receive_cw20(
    _env: Env,
    deps: DepsMut,
    info: MessageInfo,
    receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    // Only accepts cw20 tokens
    nonpayable(&info)?;

    let msg: ReceiveMsg = from_json(&receive_msg.msg)?;

    match msg {
        ReceiveMsg::Fund {} => {
            let vest = PAYMENT.get_vest(deps.storage)?;

            if vest.total() != receive_msg.amount {
                return Err(ContractError::WrongFundAmount {
                    sent: receive_msg.amount,
                    expected: vest.total(),
                });
            } // correct amount

            if !vest.denom.is_cw20(&info.sender) {
                return Err(ContractError::WrongCw20);
            } // correct denom

            if vest.status != Status::Unfunded {
                return Err(ContractError::Funded);
            } // correct status

            PAYMENT.set_funded(deps.storage)?;

            Ok(Response::new()
                .add_attribute("method", "fund_cw20_vesting_payment")
                .add_attribute("receiver", vest.recipient.to_string()))
        }
    }
}

pub fn execute_cancel_vesting_payment(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let msgs = PAYMENT.cancel(deps.storage, env.block.time, &info.sender)?;

    Ok(Response::new()
        .add_attribute("method", "remove_vesting_payment")
        .add_attribute("owner", info.sender)
        .add_attribute("removed_time", env.block.time.to_string())
        .add_messages(msgs))
}

pub fn execute_distribute(
    env: Env,
    deps: DepsMut,
    request: Option<Uint128>,
) -> Result<Response, ContractError> {
    let msg = PAYMENT.distribute(deps.storage, env.block.time, request)?;

    Ok(Response::new()
        .add_attribute("method", "distribute")
        .add_message(msg))
}

pub fn execute_withdraw_canceled_payment(
    deps: DepsMut,
    env: Env,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let owner = cw_ownable::get_ownership(deps.storage)?
        .owner
        .ok_or(OwnershipError::NoOwner)?;
    let msg = PAYMENT.withdraw_canceled_payment(deps.storage, env.block.time, amount, &owner)?;

    Ok(Response::new()
        .add_attribute("method", "withdraw_canceled_payment")
        .add_message(msg))
}

pub fn execute_update_owner(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    action: cw_ownable::Action,
) -> Result<Response, ContractError> {
    if let Status::Canceled { owner_withdrawable } = PAYMENT.get_vest(deps.storage)?.status {
        if action == cw_ownable::Action::RenounceOwnership && !owner_withdrawable.is_zero() {
            // Ownership cannot be removed if there are withdrawable
            // funds as this would lock those funds in the contract.
            return Err(ContractError::Cancelled);
        }
    }
    let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
    Ok(Response::default().add_attributes(ownership.into_attributes()))
}

pub fn execute_delegate(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    validator: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let vest = PAYMENT.get_vest(deps.storage)?;

    match vest.status {
        Status::Unfunded => return Err(ContractError::NotFunded),
        Status::Funded => {
            if info.sender != vest.recipient {
                return Err(ContractError::NotReceiver);
            }
        }
        Status::Canceled { .. } => return Err(ContractError::Cancelled),
    }

    let denom = deps.querier.query_bonded_denom()?;
    if !vest.denom.is_native(&denom) {
        return Err(ContractError::NotStakeable);
    }

    PAYMENT.on_delegate(deps.storage, env.block.time, validator.clone(), amount)?;

    let msg = StakingMsg::Delegate {
        validator: validator.clone(),
        amount: Coin { denom, amount },
    };

    Ok(Response::new()
        .add_attribute("method", "delegate")
        .add_attribute("amount", amount.to_string())
        .add_attribute("validator", validator)
        .add_message(msg))
}

pub fn execute_redelegate(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    src_validator: String,
    dst_validator: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let vest = PAYMENT.get_vest(deps.storage)?;

    match vest.status {
        Status::Unfunded => return Err(ContractError::NotFunded),
        Status::Funded => {
            if info.sender != vest.recipient {
                return Err(ContractError::NotReceiver);
            }
        }
        Status::Canceled { .. } => return Err(ContractError::Cancelled),
    }

    let denom = deps.querier.query_bonded_denom()?;
    if !vest.denom.is_native(&denom) {
        return Err(ContractError::NotStakeable);
    }

    let resp: DelegationResponse = deps.querier.query(
        &StakingQuery::Delegation {
            delegator: env.contract.address.into_string(),
            validator: src_validator.clone(),
        }
        .into(),
    )?;

    let delegation = resp
        .delegation
        .ok_or(ContractError::NoDelegation(src_validator.clone()))?;
    if delegation.can_redelegate.amount < amount {
        return Err(ContractError::NonImmediateRedelegate {
            max: delegation.can_redelegate.amount,
        });
    }

    PAYMENT.on_redelegate(
        deps.storage,
        env.block.time,
        src_validator.clone(),
        dst_validator.clone(),
        amount,
    )?;

    let msg = StakingMsg::Redelegate {
        src_validator: src_validator.clone(),
        dst_validator: dst_validator.clone(),
        amount: Coin { denom, amount },
    };

    Ok(Response::new()
        .add_attribute("method", "redelegate")
        .add_attribute("amount", amount.to_string())
        .add_attribute("src_validator", src_validator)
        .add_attribute("dst_validator", dst_validator)
        .add_message(msg))
}

pub fn execute_undelegate(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    validator: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    nonpayable(&info)?;

    let vest = PAYMENT.get_vest(deps.storage)?;

    match vest.status {
        Status::Unfunded => return Err(ContractError::NotFunded),
        Status::Funded => {
            if info.sender != vest.recipient {
                return Err(ContractError::NotReceiver);
            }
        }
        // Anyone can undelegate while the contract is in the canceled
        // state. This is to prevent us from neededing to undelegate
        // all at once when the contract is canceled which could be a
        // DOS vector if the veste staked to 50+ validators.
        Status::Canceled { .. } => (),
    };

    let ubs = UNBONDING_DURATION_SECONDS.load(deps.storage)?;
    PAYMENT.on_undelegate(deps.storage, env.block.time, validator.clone(), amount, ubs)?;

    let denom = deps.querier.query_bonded_denom()?;

    let msg = StakingMsg::Undelegate {
        validator: validator.clone(),
        amount: Coin { denom, amount },
    };

    Ok(Response::default()
        .add_message(msg)
        .add_attribute("method", "undelegate")
        .add_attribute("validator", validator)
        .add_attribute("amount", amount))
}

pub fn execute_set_withdraw_address(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let vest = PAYMENT.get_vest(deps.storage)?;
    match vest.status {
        Status::Unfunded | Status::Funded => {
            if info.sender != vest.recipient {
                return Err(ContractError::NotReceiver);
            }
        }
        // In the cancelled state the owner is receiving staking
        // rewards and may update the withdraw address.
        Status::Canceled { .. } => cw_ownable::assert_owner(deps.storage, &info.sender)?,
    }

    if address == env.contract.address {
        return Err(ContractError::SelfWithdraw);
    }

    let msg = DistributionMsg::SetWithdrawAddress {
        address: address.clone(),
    };

    Ok(Response::default()
        .add_attribute("method", "set_withdraw_address")
        .add_attribute("address", address)
        .add_message(msg))
}

pub fn execute_withdraw_rewards(validator: String) -> Result<Response, ContractError> {
    let withdraw_msg = DistributionMsg::WithdrawDelegatorReward { validator };
    Ok(Response::default()
        .add_attribute("method", "execute_withdraw_rewards")
        .add_message(withdraw_msg))
}

pub fn execute_register_slash(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    validator: String,
    time: Timestamp,
    amount: Uint128,
    during_unbonding: bool,
) -> Result<Response, ContractError> {
    cw_ownable::assert_owner(deps.storage, &info.sender)?;
    if time > env.block.time {
        Err(ContractError::FutureSlash)
    } else {
        PAYMENT.register_slash(
            deps.storage,
            validator.clone(),
            time,
            amount,
            during_unbonding,
        )?;
        Ok(Response::default()
            .add_attribute("method", "execute_register_slash")
            .add_attribute("during_unbonding", during_unbonding.to_string())
            .add_attribute("validator", validator)
            .add_attribute("time", time.to_string())
            .add_attribute("amount", amount))
    }
}

/// Stake tokens in a DAO
pub fn execute_dao_stake(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
    staking_contract: String,
) -> Result<Response, ContractError> {
    // Validate staking contract address
    deps.api.addr_validate(&staking_contract)?;

    // Load vest and check status, only recipients can stake
    let vest = PAYMENT.get_vest(deps.storage)?;
    match vest.status {
        Status::Unfunded => return Err(ContractError::NotFunded),
        Status::Funded => {
            if info.sender != vest.recipient {
                return Err(ContractError::NotReceiver);
            }
        }
        Status::Canceled { .. } => return Err(ContractError::Cancelled),
    }

    // Validate staking contract is on the allowist
    // Otherwise staking might be abused to get tokens out of this contract
    let dao_staking = DAO_STAKING_LIMITS.load(deps.storage)?;
    if !dao_staking
        .staking_contract_allowlist
        .contains(&staking_contract)
    {
        return Err(ContractError::NotOnAllowlist);
    }

    // TODO limitations on how much can be staked?

    // Construct stake message
    let msg = WasmMsg::Execute {
        contract_addr: staking_contract.clone(),
        msg: to_json_binary(&dao_voting_token_staked::msg::ExecuteMsg::Stake {})?,
        funds: vec![Coin {
            denom: vest.denom.to_string(),
            amount: amount,
        }],
    };

    Ok(Response::default()
        .add_message(msg)
        .add_attribute("method", "execute_dao_stake")
        .add_attribute("staking_contract", staking_contract)
        .add_attribute("amount", amount))
}

/// Unstake tokens in a DAO
pub fn execute_dao_unstake(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
    staking_contract: String,
) -> Result<Response, ContractError> {
    // Validate staking contract address
    deps.api.addr_validate(&staking_contract)?;

    // Load vest and check status, only recipients can unstake
    let vest = PAYMENT.get_vest(deps.storage)?;
    if info.sender != vest.recipient {
        return Err(ContractError::NotReceiver);
    };

    // Construct unstake message
    let msg = WasmMsg::Execute {
        contract_addr: staking_contract.clone(),
        msg: to_json_binary(&dao_voting_token_staked::msg::ExecuteMsg::Unstake { amount })?,
        funds: vec![],
    };

    Ok(Response::default()
        .add_message(msg)
        .add_attribute("method", "execute_dao_unstake")
        .add_attribute("staking_contract", staking_contract)
        .add_attribute("amount", amount))
}

/// Vote in a DAO
pub fn execute_dao_vote(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    proposal_module: String,
    proposal_id: u64,
    vote: Vote,
    rationale: Option<String>,
) -> Result<Response, ContractError> {
    // Validate proposal module contract address
    deps.api.addr_validate(&proposal_module)?;

    // Check sender is the recipient of the vesting contract
    let vest = PAYMENT.get_vest(deps.storage)?;
    if info.sender != vest.recipient {
        return Err(ContractError::NotReceiver);
    };

    // Construct voting message
    let msg = WasmMsg::Execute {
        contract_addr: proposal_module.clone(),
        msg: to_json_binary(&dao_proposal_single::msg::ExecuteMsg::Vote {
            proposal_id,
            vote,
            rationale: rationale.clone(),
        })?,
        funds: vec![],
    };

    Ok(Response::default()
        .add_message(msg)
        .add_attribute("method", "execute_dao_vote")
        .add_attribute("proposal_module", proposal_module)
        .add_attribute("proposal_id", proposal_id.to_string())
        .add_attribute("vote", vote.to_string())
        .add_attribute("rationale", rationale.unwrap_or_default()))
}

// TODO how to handle if a deposit is slashed?
// TODO maybe we just don't handle proposals?
// Makes this contract much easier... as we don't have to worry about deposits?
// Cheeper audit too...
/// Create a proposal in a DAO
pub fn execute_dao_propose(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    proposal_module: String,
    proposal: SingleChoiceProposeMsg,
) -> Result<Response, ContractError> {
    // Check sender is the recipient of the vesting contract
    let vest = PAYMENT.get_vest(deps.storage)?;
    if info.sender != vest.recipient {
        return Err(ContractError::NotReceiver);
    };

    // Validate proposal module contract address
    let proposal_contract = deps.api.addr_validate(&proposal_module)?;

    // Get proposal creation policy
    let policy = deps.querier.query_wasm_smart(
        proposal_contract,
        &dao_proposal_single::msg::QueryMsg::ProposalCreationPolicy {},
    )?;

    // Match policy, if anyone proceed to make proposal directly
    // otherwise check deposits?
    match policy {
        ProposalCreationPolicy::Anyone {} => {
            // Construct message to submit proposal directly
            let msg = WasmMsg::Execute {
                contract_addr: proposal_module.clone(),
                msg: to_json_binary(&dao_proposal_single::msg::ExecuteMsg::Propose(proposal))?,
                funds: vec![],
            };

            Ok(Response::default()
                .add_message(msg)
                .add_attribute("method", "execute_dao_propose")
                .add_attribute("proposal_module", proposal_module))
        }
        ProposalCreationPolicy::Module { addr } => {
            // Get the config for the pre-propose module
            let config: dao_pre_propose_single::Config = deps
                .querier
                .query_wasm_smart(addr, &dao_pre_propose_single::QueryMsg::Config {})?;

            if let Some(deposit_info) = config.deposit_info {
                match deposit_info.refund_policy {
                    DepositRefundPolicy::Always {} => unimplemented!(),
                    DepositRefundPolicy::Never {} => unimplemented!(),
                    DepositRefundPolicy::OnlyPassed {} => unimplemented!(),
                }
            }

            unimplemented!();
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
        QueryMsg::Info {} => to_json_binary(&PAYMENT.get_vest(deps.storage)?),
        QueryMsg::Distributable { t } => to_json_binary(&PAYMENT.distributable(
            deps.storage,
            &PAYMENT.get_vest(deps.storage)?,
            t.unwrap_or(env.block.time),
        )?),
        QueryMsg::Stake(q) => PAYMENT.query_stake(deps.storage, q),
        QueryMsg::Vested { t } => to_json_binary(
            &PAYMENT
                .get_vest(deps.storage)?
                .vested(t.unwrap_or(env.block.time)),
        ),
        QueryMsg::TotalToVest {} => to_json_binary(&PAYMENT.get_vest(deps.storage)?.total()),
        QueryMsg::VestDuration {} => to_json_binary(&PAYMENT.duration(deps.storage)?),
    }
}
