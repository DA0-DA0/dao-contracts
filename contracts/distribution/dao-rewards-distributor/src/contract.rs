#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_json, to_json_binary, to_json_string, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128, Uint256, WasmMsg
};
use cw2::set_contract_version;
use cw20::Denom::Cw20;
use cw20::{Cw20ReceiveMsg, Denom, UncheckedDenom};
use cw4::MemberChangedHookMsg;
use cw_utils::{Duration, Expiration};
use dao_hooks::{nft_stake::NftStakeChangedHookMsg, stake::StakeChangedHookMsg};
use dao_interface::voting::{
    Query as VotingQueryMsg, TotalPowerAtHeightResponse, VotingPowerAtHeightResponse,
};
use std::collections::HashMap;
use std::convert::TryInto;

use crate::msg::{
    ExecuteMsg, InfoResponse, InstantiateMsg, PendingRewardsResponse, QueryMsg, ReceiveMsg,
};
use crate::state::{
    Config, RewardConfig, CONFIG, LAST_UPDATE_EXPIRATION, PENDING_REWARDS, REWARDS_PER_TOKEN, REWARD_CONFIG, STRING_DENOM_TO_CHECKED_DENOM, USER_REWARD_PER_TOKEN
};
use crate::ContractError;
use crate::ContractError::{InvalidCw20, InvalidFunds, NoRewardsClaimable};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Intialize the contract owner
    cw_ownable::initialize_owner(deps.storage, deps.api, msg.owner.as_deref())?;
    
    let mut reward_denoms: Vec<Denom> = Vec::with_capacity(msg.reward_denoms_whitelist.len());
    let mut denom_reward_rate_map: HashMap<String, Uint128> = HashMap::new();
    for unchecked_denom in msg.reward_denoms_whitelist {
        let denom_str = match &unchecked_denom {
            UncheckedDenom::Native(denom) => denom.to_string(),
            UncheckedDenom::Cw20(addr) => addr.to_string(),
        };
        let checked_denom = unchecked_denom.into_checked(deps.as_ref())?; 
        reward_denoms.push(checked_denom.clone());
        STRING_DENOM_TO_CHECKED_DENOM.save(
            deps.storage,
            denom_str.to_string(),
            &checked_denom,
        )?;
        denom_reward_rate_map.insert( denom_str, Uint128::zero());
    }

    // Verify contract provided is a voting module contract
    let _: TotalPowerAtHeightResponse = deps.querier.query_wasm_smart(
        &msg.vp_contract,
        &VotingQueryMsg::TotalPowerAtHeight { height: None },
    )?;

    // Optional hook caller is allowed to call voting power change hooks.
    // If not provided, only the voting power contract is used.
    let hook_caller: Option<Addr> = match msg.hook_caller {
        Some(addr) => Some(deps.api.addr_validate(&addr)?),
        None => None,
    };

    // Save the contract configuration
    let config = Config {
        vp_contract: deps.api.addr_validate(&msg.vp_contract)?,
        hook_caller,
        reward_denoms,
    };
    CONFIG.save(deps.storage, &config)?;

    // Reward duration must be greater than 0
    if let Duration::Height(0) | Duration::Time(0) = msg.reward_duration {
        return Err(ContractError::ZeroRewardDuration {});
    }

    // Initialize the reward config
    let reward_config = RewardConfig {
        period_finish_expiration: Expiration::Never {},
        denom_to_reward_rate: denom_reward_rate_map,
        reward_duration: msg.reward_duration,
    };
    REWARD_CONFIG.save(deps.storage, &reward_config)?;

    Ok(Response::new()
        .add_attribute("owner", msg.owner.unwrap_or_else(|| "None".to_string()))
        .add_attribute("vp_contract", config.vp_contract)
        .add_attribute("reward_rate", format!("{:?}", reward_config.denom_to_reward_rate))
        .add_attribute(
            "period_finish",
            reward_config.period_finish_expiration.to_string(),
        )
        .add_attribute("reward_duration", reward_config.reward_duration.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::StakeChangeHook(msg) => execute_stake_changed(deps, env, info, msg),
        ExecuteMsg::NftStakeChangeHook(msg) => execute_nft_stake_changed(deps, env, info, msg),
        ExecuteMsg::MemberChangedHook(msg) => execute_membership_changed(deps, env, info, msg),
        ExecuteMsg::Claim {} => execute_claim(deps, env, info),
        ExecuteMsg::Fund {} => execute_fund_native(deps, env, info),
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::UpdateRewardDuration { new_duration } => {
            execute_update_reward_duration(deps, env, info, new_duration)
        }
        ExecuteMsg::UpdateOwnership(action) => execute_update_owner(deps, info, env, action),
    }
}

pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let msg: ReceiveMsg = from_json(&wrapper.msg)?;
    let config = CONFIG.load(deps.storage)?;
    let sender = deps.api.addr_validate(&wrapper.sender)?;

    // we try to find the sent denom in our rewards config
    let cw20_denom = Denom::Cw20(info.sender.clone());
    let reward_denom = config.reward_denoms
        .iter()
        .find(|d| cw20_denom == **d);

    match reward_denom {
        // if we found the denom, we execute the funding
        Some(reward_denom) => match msg {
            ReceiveMsg::Fund {} => execute_fund(deps, env, sender, vec![(reward_denom.clone(), wrapper.amount)]),
        },
        // otherwise we error
        None => Err(InvalidCw20 {  }),
    }
}

pub fn execute_fund_native(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // we iterate over the expected reward denoms and check if they were provided
    let mut provided_denoms: Vec<(Denom, Uint128)> = Vec::with_capacity(info.funds.len()); 
    for reward_denom in config.reward_denoms {
        // we only care about native denoms here
        if let Denom::Native(denom) = reward_denom.clone() {
            match cw_utils::must_pay(&info, denom.as_str()) {
                Ok(paid_amount) => provided_denoms.push((reward_denom, paid_amount)),
                Err(_) => (),
            }
        }
    };

    // if we didn't find any native denoms, we error
    if provided_denoms.len() == 0 {
        return Err(InvalidFunds {})
    }

    execute_fund(deps, env, info.sender, provided_denoms)
}

pub fn execute_fund(
    mut deps: DepsMut,
    env: Env,
    sender: Addr,
    denoms_to_fund: Vec<(Denom, Uint128)>,
) -> Result<Response, ContractError> {
    // Ensure that the sender is the owner
    cw_ownable::assert_owner(deps.storage, &sender)?;
    update_rewards(&mut deps, &env, &sender)?;

    let mut reward_config = REWARD_CONFIG.load(deps.storage)?;

    // Ensure that the current reward period has ended and that period expiration is known.
    reward_config.validate_period_finish_expiration_if_set(&env.block)?;

    let reward_duration_value = reward_config.get_reward_duration_value();
    let period_finish_expiration = reward_config.reward_duration.after(&env.block);

    for (denom_to_fund, amount) in &denoms_to_fund {
        let new_rate = amount
            .checked_div(Uint128::from(reward_duration_value))
            .map_err(StdError::divide_by_zero)?;

        if new_rate == Uint128::zero() {
            return Err(ContractError::RewardRateLessThenOnePerBlock {});
        } else {
            reward_config.denom_to_reward_rate.insert(
                match denom_to_fund {
                    Denom::Native(denom) => denom.to_string(),
                    Denom::Cw20(addr) => addr.to_string(),
                },
                new_rate,
            );
        }
    }

    let new_reward_config = RewardConfig {
        period_finish_expiration,
        denom_to_reward_rate: reward_config.denom_to_reward_rate.clone(),
        // As we're not changing the value and changing the value
        // validates that the duration is non-zero we don't need to
        // check here.
        reward_duration: reward_config.reward_duration,
    };

    REWARD_CONFIG.save(deps.storage, &new_reward_config)?;
    LAST_UPDATE_EXPIRATION.save(
        deps.storage,
        &match reward_config.reward_duration {
            Duration::Height(_) => Expiration::AtHeight(env.block.height),
            Duration::Time(_) => Expiration::AtTime(env.block.time),
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "fund")
        .add_attribute("new_amounts", format!("{:?}", denoms_to_fund))
        .add_attribute("new_reward_rates", format!("{:?}", new_reward_config.denom_to_reward_rate)))
}

pub fn execute_stake_changed(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: StakeChangedHookMsg,
) -> Result<Response, ContractError> {
    // Check that the sender is the vp_contract (or the hook_caller if configured).
    check_hook_caller(deps.as_ref(), info)?;

    match msg {
        StakeChangedHookMsg::Stake { addr, .. } => execute_stake(deps, env, addr),
        StakeChangedHookMsg::Unstake { addr, .. } => execute_unstake(deps, env, addr),
    }
}

pub fn execute_membership_changed(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: MemberChangedHookMsg,
) -> Result<Response, ContractError> {
    // Check that the sender is the vp_contract (or the hook_caller if configured).
    check_hook_caller(deps.as_ref(), info)?;

    // Get the addresses of members whose voting power has changed.
    for member in msg.diffs {
        let addr = deps.api.addr_validate(&member.key)?;
        update_rewards(&mut deps, &env, &addr)?;
    }

    Ok(Response::new().add_attribute("action", "membership_changed"))
}

pub fn execute_nft_stake_changed(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: NftStakeChangedHookMsg,
) -> Result<Response, ContractError> {
    // Check that the sender is the vp_contract (or the hook_caller if configured).
    check_hook_caller(deps.as_ref(), info)?;

    match msg {
        NftStakeChangedHookMsg::Stake { addr, .. } => execute_stake(deps, env, addr),
        NftStakeChangedHookMsg::Unstake { addr, .. } => execute_unstake(deps, env, addr),
    }
}

pub fn execute_stake(mut deps: DepsMut, env: Env, addr: Addr) -> Result<Response, ContractError> {
    update_rewards(&mut deps, &env, &addr)?;
    Ok(Response::new().add_attribute("action", "stake"))
}

pub fn execute_unstake(mut deps: DepsMut, env: Env, addr: Addr) -> Result<Response, ContractError> {
    update_rewards(&mut deps, &env, &addr)?;
    Ok(Response::new().add_attribute("action", "unstake"))
}

pub fn execute_claim(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // Update the rewards information for the sender.
    update_rewards(&mut deps, &env, &info.sender)?;

    // Get the pending rewards for the sender.
    let current_rewards = PENDING_REWARDS
        .load(deps.storage, info.sender.clone())
        .map_err(|_| NoRewardsClaimable {})?;

    let mut transfer_msgs: Vec<CosmosMsg> = Vec::new();
    let mut nullified_rewards: HashMap<String, Uint128> = HashMap::new();
    for (denom, amount) in current_rewards {
        if !amount.is_zero() {            
            // Get the checked denom for the string based denom
            let checked_denom = STRING_DENOM_TO_CHECKED_DENOM.load(deps.storage, denom.to_string())?;
            // generate a transfer message for the reward
            transfer_msgs.push(get_transfer_msg(info.sender.clone(), amount, checked_denom)?);
        }
        nullified_rewards.insert(denom, Uint128::zero());
    }

    // If no claim transfers can be done, error
    if transfer_msgs.len() == 0 {
        return Err(ContractError::NoRewardsClaimable {});
    }

    // save the nullified rewards
    PENDING_REWARDS.save(deps.storage, info.sender.clone(), &nullified_rewards)?;

    Ok(Response::new()
        .add_messages(transfer_msgs)
        .add_attribute("action", "claim"))
}

pub fn execute_update_owner(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    action: cw_ownable::Action,
) -> Result<Response, ContractError> {
    // Update the current contract owner.
    // Note, this is a two step process, the new owner must accept this ownership transfer.
    // First the owner specifies the new owner, then the new owner must accept.
    let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
    Ok(Response::default().add_attributes(ownership.into_attributes()))
}

/// Ensures hooks that update voting power are only called by the voting power contract
/// or the designated hook_caller contract (if configured).
pub fn check_hook_caller(deps: Deps, info: MessageInfo) -> Result<(), ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only the voting power contract or the designated hook_caller contract (if configured)
    // can call this hook.
    match config.hook_caller {
        Some(hook_caller) => {
            if info.sender != hook_caller {
                return Err(ContractError::InvalidHookSender {});
            }
        }
        None => {
            if info.sender != config.vp_contract {
                return Err(ContractError::InvalidHookSender {});
            };
        }
    }
    Ok(())
}

/// Returns the approqqate CosmosMsg for transferring the reward token.
pub fn get_transfer_msg(recipient: Addr, amount: Uint128, denom: Denom) -> StdResult<CosmosMsg> {
    match denom {
        Denom::Native(denom) => Ok(BankMsg::Send {
            to_address: recipient.into_string(),
            amount: vec![Coin { denom, amount }],
        }
        .into()),
        Denom::Cw20(addr) => {
            let cw20_msg = to_json_binary(&cw20::Cw20ExecuteMsg::Transfer {
                recipient: recipient.into_string(),
                amount,
            })?;
            Ok(WasmMsg::Execute {
                contract_addr: addr.into_string(),
                msg: cw20_msg,
                funds: vec![],
            }
            .into())
        }
    }
}

pub fn update_rewards(deps: &mut DepsMut, env: &Env, addr: &Addr) -> StdResult<()> {
    let config = CONFIG.load(deps.storage)?;

    // Reward per token represents the amount of rewards per unit of voting power.
    let rewards_per_token_map = get_rewards_per_token(deps.as_ref(), env, &config.vp_contract)?;
    REWARDS_PER_TOKEN.save(deps.storage, &rewards_per_token_map)?;
    println!("rewards_per_token_map: {:?}", rewards_per_token_map);
    // The amount of rewards earned up until this point.
    let earned_rewards = get_rewards_earned(
        deps.as_ref(),
        env,
        addr,
        rewards_per_token_map.clone(),
        &config.vp_contract,
    )?;

    println!("earned_rewards: {:?}", earned_rewards);

    let mut pending_rewards = PENDING_REWARDS
        .load(deps.storage, addr.clone())
        .unwrap_or_default();
    println!("pending rewards: {:?}", pending_rewards);

    for (denom, amount) in earned_rewards {
        if !amount.is_zero() {
            let new_amount = match pending_rewards.get(&denom) {
                Some(pending_amount) => *pending_amount + amount,
                None => amount,
            };
            pending_rewards.insert(denom, new_amount);
        }
    }

    // Update the users pending rewards
    PENDING_REWARDS.save(deps.storage, addr.clone(), &pending_rewards)?;

    // Update the users latest reward per token value.
    USER_REWARD_PER_TOKEN.save(deps.storage, addr.clone(), &rewards_per_token_map)?;

    // Update the last time rewards were updated.
    let reward_config = REWARD_CONFIG.load(deps.storage)?;
    let last_time_reward_applicable =
        reward_config.get_latest_reward_distribution_expiration_date(&env.block);
    LAST_UPDATE_EXPIRATION.save(deps.storage, &last_time_reward_applicable)?;

    Ok(())
}

fn get_expiration_diff(a: Expiration, b: Expiration) -> StdResult<u64> {
    match (a, b) {
        (Expiration::AtHeight(a), Expiration::AtHeight(b)) => Ok(a - b),
        (Expiration::AtTime(a), Expiration::AtTime(b)) => Ok(a.seconds() - b.seconds()),
        (Expiration::Never {}, Expiration::Never {}) => Ok(0),
        _ => Err(StdError::generic_err(format!(
            "incompatible expirations: got a {:?}, b {:?}",
            a, b
        ))),
    }
}
pub fn get_rewards_per_token(deps: Deps, env: &Env, vp_contract: &Addr) -> StdResult<HashMap<String, Uint256>> {
    let reward_config = REWARD_CONFIG.load(deps.storage)?;
    // Get the total voting power at this block height.
    let total_power = get_total_voting_power(deps, env, vp_contract)?;

    // Get information on the last time rewards were updated.
    let last_time_reward_applicable =
        reward_config.get_latest_reward_distribution_expiration_date(&env.block);
    let last_update_expiration = LAST_UPDATE_EXPIRATION
        .load(deps.storage)
        .unwrap_or_default();

    // Get the amount of rewards per unit of voting power.
    let current_reward_per_token = REWARDS_PER_TOKEN.load(deps.storage).unwrap_or_default();

    let mut rewards_per_token_map: HashMap<String, Uint256> = HashMap::new();
    let reward_str_denoms: Box<dyn Iterator<Item = Result<String, StdError>>> = STRING_DENOM_TO_CHECKED_DENOM.keys(deps.storage, None, None, cosmwasm_std::Order::Ascending);
    for whitelisted_reward_denom in reward_str_denoms {
        let token = whitelisted_reward_denom?;
        let default_amount = Uint256::zero();
        let amount = current_reward_per_token.get(&token).unwrap_or(&default_amount);
        let additional_reward_for_token = if total_power == Uint128::zero() {
            Uint256::zero()
        } else {
            let reward_config_rate_for_token = match reward_config.denom_to_reward_rate.get(&token) {
                Some(val) => *val,
                None => Uint128::zero(),
            };
            
            let expiration_diff = Uint128::from(get_expiration_diff(
                last_time_reward_applicable,
                last_update_expiration,
            )?);

            let numerator = reward_config_rate_for_token
                .full_mul(expiration_diff)
                .checked_mul(scale_factor())?;
            let denominator = Uint256::from(total_power);
            numerator.checked_div(denominator)?
        };
        let new_reward_per_token = *amount + additional_reward_for_token;
        rewards_per_token_map.insert(token, new_reward_per_token);
    }

    Ok(rewards_per_token_map)
}

pub fn get_rewards_earned(
    deps: Deps,
    env: &Env,
    addr: &Addr,
    reward_per_token: HashMap<String, Uint256>,
    vp_contract: &Addr,
) -> StdResult<HashMap<String, Uint128>> {
    // Get the users voting power at the current height.
    let voting_power = Uint256::from(get_voting_power(deps, env, vp_contract, addr)?);

    // Load the users latest reward per token value.
    let user_reward_per_token = USER_REWARD_PER_TOKEN
        .load(deps.storage, addr.clone())
        .unwrap_or_default();

    let mut entitled_rewards: HashMap<String, Uint128> = HashMap::new();
    // we iterate over passed in `reward_per_token` values and subtract previous entitlements
    for (denom, amount) in reward_per_token.iter() {

        // Calculate the difference between the current reward per token value and the users latest
        let to_sub = match user_reward_per_token.get(denom) {
            Some(val) => *val,
            None => Uint256::zero(),
        };
        let reward_factor = amount.checked_sub(to_sub)?;

        // Calculate the amount of rewards earned.
        // voting_power * reward_factor / scale_factor
        let earned_rewards_amount: Uint128 = voting_power
            .checked_mul(reward_factor)?
            .checked_div(scale_factor())?
            .try_into()?;
        entitled_rewards.insert(denom.to_string(), earned_rewards_amount);
    }

    Ok(entitled_rewards)
}

fn get_total_voting_power(deps: Deps, env: &Env, contract_addr: &Addr) -> StdResult<Uint128> {
    let msg = VotingQueryMsg::TotalPowerAtHeight {
        height: Some(env.block.height),
    };
    let resp: TotalPowerAtHeightResponse = deps.querier.query_wasm_smart(contract_addr, &msg)?;
    Ok(resp.power)
}

fn get_voting_power(
    deps: Deps,
    env: &Env,
    contract_addr: &Addr,
    addr: &Addr,
) -> StdResult<Uint128> {
    let msg = VotingQueryMsg::VotingPowerAtHeight {
        address: addr.into(),
        height: Some(env.block.height),
    };
    let resp: VotingPowerAtHeightResponse = deps.querier.query_wasm_smart(contract_addr, &msg)?;
    Ok(resp.power)
}

pub fn execute_update_reward_duration(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    new_duration: Duration,
) -> Result<Response, ContractError> {
    // Ensure the sender is the owner.
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut reward_config = REWARD_CONFIG.load(deps.storage)?;
    // Ensure that the current reward period has ended
    reward_config.validate_period_finish_expiration_if_set(&env.block)?;

    if let Duration::Height(0) | Duration::Time(0) = new_duration {
        return Err(ContractError::ZeroRewardDuration {});
    }

    let old_duration = reward_config.reward_duration;
    reward_config.reward_duration = new_duration;
    REWARD_CONFIG.save(deps.storage, &reward_config)?;

    Ok(Response::new()
        .add_attribute("action", "update_reward_duration")
        .add_attribute("new_duration", new_duration.to_string())
        .add_attribute("old_duration", old_duration.to_string()))
}

fn scale_factor() -> Uint256 {
    Uint256::from(10u8).pow(39)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Info {} => Ok(to_json_binary(&query_info(deps, env)?)?),
        QueryMsg::GetPendingRewards { address } => {
            Ok(to_json_binary(&query_pending_rewards(deps, env, address)?)?)
        }
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),
    }
}

pub fn query_info(deps: Deps, _env: Env) -> StdResult<InfoResponse> {
    let config = CONFIG.load(deps.storage)?;
    let reward = REWARD_CONFIG.load(deps.storage)?;
    Ok(InfoResponse { config, reward })
}

pub fn query_pending_rewards(
    deps: Deps,
    env: Env,
    addr: String,
) -> StdResult<PendingRewardsResponse> {
    let addr = deps.api.addr_validate(&addr)?;
    let config = CONFIG.load(deps.storage)?;
    
    let reward_per_token = get_rewards_per_token(deps, &env, &config.vp_contract)?;

    let earned_rewards =
        get_rewards_earned(deps, &env, &addr, reward_per_token, &config.vp_contract)?;

    let existing_rewards = PENDING_REWARDS
        .load(deps.storage, addr.clone())
        .unwrap_or_default();

    let mut pending_rewards: HashMap<String, Uint128> = HashMap::new();
    
    for whitelisted_rewards_denom in config.reward_denoms {
        let denom = match whitelisted_rewards_denom {
            Denom::Native(denom) => denom.to_string(),
            Denom::Cw20(addr) => addr.to_string(),
        };
        let default_amt = Uint128::zero();
        let earned_amount = earned_rewards.get(&denom).unwrap_or(&default_amt);
        let existing_amount = existing_rewards.get(&denom).unwrap_or(&default_amt);
        pending_rewards.insert(denom, *earned_amount + *existing_amount);
    }

    Ok(PendingRewardsResponse {
        address: addr.to_string(),
        pending_rewards,
        last_update_expiration: LAST_UPDATE_EXPIRATION.load(deps.storage)?,
    })
}
