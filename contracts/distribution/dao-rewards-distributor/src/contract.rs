#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, from_json, to_json_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps,
    DepsMut, Env, MessageInfo, Order, Response, StdError, StdResult, Uint128, Uint256, WasmMsg,
};
use cw2::set_contract_version;
use cw20::{Cw20ReceiveMsg, Denom};
use cw_utils::{Duration, Expiration};
use dao_interface::voting::{
    Query as VotingQueryMsg, TotalPowerAtHeightResponse, VotingPowerAtHeightResponse,
};
use std::collections::HashMap;
use std::convert::TryInto;

use crate::hooks::{
    execute_membership_changed, execute_nft_stake_changed, execute_stake_changed,
    subscribe_denom_to_hook,
};
use crate::msg::{
    ExecuteMsg, InfoResponse, InstantiateMsg, PendingRewardsResponse, QueryMsg, ReceiveMsg,
    RewardDenomRegistrationMsg, RewardEmissionConfig,
};
use crate::state::{
    DenomRewardConfig, CUMULATIVE_REWARDS_PER_TOKEN, REWARD_DENOM_CONFIGS, USER_REWARD_CONFIGS,
};
use crate::ContractError;
use crate::ContractError::{InvalidCw20, InvalidFunds};

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

    Ok(Response::new().add_attribute("owner", msg.owner.unwrap_or_else(|| "None".to_string())))
}

pub fn validate_voting_power_contract(
    deps: &DepsMut,
    vp_contract: String,
) -> Result<Addr, ContractError> {
    let vp_contract = deps.api.addr_validate(&vp_contract)?;
    let _: TotalPowerAtHeightResponse = deps.querier.query_wasm_smart(
        &vp_contract,
        &VotingQueryMsg::TotalPowerAtHeight { height: None },
    )?;
    Ok(vp_contract)
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
        // todo: make claim with optional vector of denoms or whatever
        ExecuteMsg::Claim { denom } => execute_claim(deps, env, info, denom),
        ExecuteMsg::Fund {} => execute_fund_native(deps, env, info),
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::UpdateRewardEmissionConfig {
            denom,
            new_emission_rate,
            new_emission_time,
        } => execute_update_denom_emission_config(
            deps,
            env,
            info,
            denom,
            new_emission_rate,
            new_emission_time,
        ),
        ExecuteMsg::UpdateOwnership(action) => execute_update_owner(deps, info, env, action),
        ExecuteMsg::Shutdown { denom } => execute_shutdown(deps, info, env, denom),
        ExecuteMsg::RegisterRewardDenom(msg) => execute_register_reward_denom(deps, info, msg),
    }
}

/// registers a new denom for rewards distribution.
/// only the owner can register a new denom.
/// a denom can only be registered once; update if you need to change something.
pub fn execute_register_reward_denom(
    deps: DepsMut,
    info: MessageInfo,
    msg: RewardDenomRegistrationMsg,
) -> Result<Response, ContractError> {
    // only the owner can initiate a shutdown
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    msg.reward_emission_config.validate_emission_time_window()?;

    let checked_denom = msg.denom.into_checked(deps.as_ref())?;
    let hook_caller = deps.api.addr_validate(&msg.hook_caller)?;
    let vp_contract = validate_voting_power_contract(&deps, msg.vp_contract)?;

    // Initialize the reward config
    let reward_config = DenomRewardConfig {
        distribution_expiration: Expiration::Never {},
        reward_emission_config: msg.reward_emission_config,
        denom: checked_denom,
        last_update: Expiration::Never {},
        funded_amount: Uint128::zero(),
        hook_caller: hook_caller.clone(),
        vp_contract,
        period_start_date: Expiration::Never {},
    };
    let str_denom = reward_config.to_str_denom();

    // store the new reward denom config or error if it already exists
    REWARD_DENOM_CONFIGS.update(
        deps.storage,
        str_denom.to_string(),
        |existing| match existing {
            Some(_) => Err(ContractError::DenomAlreadyRegistered {}),
            None => Ok(reward_config),
        },
    )?;

    // registered denom starts with no accumulated rewards
    CUMULATIVE_REWARDS_PER_TOKEN.save(deps.storage, str_denom.to_string(), &Uint256::zero())?;

    // update the registered hooks to include the new denom
    subscribe_denom_to_hook(deps, hook_caller.clone(), str_denom)?;

    Ok(Response::default())
}

/// shutdown the rewards distributor contract.
/// can only be called by the admin and only during the distribution period.
/// this will clawback all (undistributed) future rewards to the admin.
/// updates the period finish expiration to the current block.
pub fn execute_shutdown(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    denom: String,
) -> Result<Response, ContractError> {
    // only the owner can initiate a shutdown
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    let mut reward_config = REWARD_DENOM_CONFIGS.load(deps.storage, denom.to_string())?;

    // shutdown is only possible during the distribution period
    ensure!(
        !reward_config.distribution_expiration.is_expired(&env.block),
        ContractError::ShutdownError("Reward period not finished".to_string())
    );

    // we get the period start and finish units in u64 (seconds/blocks)
    let period_start_units = reward_config.get_period_start_units()?;

    let period_expiration_units = reward_config.get_period_finish_units()?;

    let reward_duration_units = period_expiration_units - period_start_units;

    // find the % of reward_duration that remains from current block
    let passed_units_since_start = match reward_config.reward_emission_config.reward_rate_time {
        Duration::Height(_) => Uint128::from(env.block.height - period_start_units),
        Duration::Time(_) => Uint128::from(env.block.time.seconds() - period_start_units),
    };

    // get the fraction of what part of rewards duration is in the past
    let reward_duration_passed_fraction =
        Decimal::from_ratio(passed_units_since_start, reward_duration_units);

    // sub from 1 to get the remaining rewards duration
    let remaining_reward_duration_fraction = Decimal::one() - reward_duration_passed_fraction;

    // to get the clawback msg
    let clawback_msg = get_transfer_msg(
        info.sender.clone(),
        reward_config.funded_amount * remaining_reward_duration_fraction,
        reward_config.denom.clone(),
    )?;

    // shutdown completes the rewards
    reward_config.distribution_expiration =
        match reward_config.reward_emission_config.reward_rate_time {
            Duration::Height(_) => Expiration::AtHeight(env.block.height),
            Duration::Time(_) => Expiration::AtTime(env.block.time),
        };

    REWARD_DENOM_CONFIGS.save(deps.storage, denom.to_string(), &reward_config)?;

    Ok(Response::new()
        .add_attribute("action", "shutdown")
        .add_message(clawback_msg))
}

pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    // verify msg
    let _msg: ReceiveMsg = from_json(&wrapper.msg)?;

    // only the token contract can execute this message
    let sender = deps.api.addr_validate(&wrapper.sender)?;

    let reward_denom_config = REWARD_DENOM_CONFIGS.load(deps.storage, info.sender.to_string())?;

    // sanity check
    ensure!(
        reward_denom_config.denom == Denom::Cw20(info.sender.clone()),
        InvalidCw20 {}
    );

    execute_fund(deps, env, sender, reward_denom_config, wrapper.amount)
}

pub fn execute_fund_native(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    // todo: switch to single denom funding and validate with cw_utils::must_pay()

    let mut provided_denoms: Vec<(DenomRewardConfig, Uint128)> =
        Vec::with_capacity(info.funds.len());
    for coin in info.funds.iter() {
        match REWARD_DENOM_CONFIGS.load(deps.storage, coin.denom.clone()) {
            Ok(config) => provided_denoms.push((config, coin.amount)),
            Err(_) => return Err(ContractError::InvalidFunds {}),
        }
    }

    // if we didn't find any native denoms, we error
    let (provided_denom_config, amount) = if provided_denoms.is_empty() || provided_denoms.len() > 1
    {
        return Err(InvalidFunds {});
    } else {
        (provided_denoms[0].0.clone(), provided_denoms[0].1)
    };

    execute_fund(deps, env, info.sender, provided_denom_config, amount)
}

pub fn execute_fund(
    mut deps: DepsMut,
    env: Env,
    sender: Addr,
    mut denom_reward_config: DenomRewardConfig,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // Ensure that the sender is the owner
    cw_ownable::assert_owner(deps.storage, &sender)?;

    let denom_str = denom_reward_config.to_str_denom();

    // first we update the existing rewards (if any)
    update_rewards(&mut deps, &env, &sender, vec![denom_str.to_string()])?;

    // we derive the period for which the rewards are funded
    // by looking at the existing reward emission config and the funded amount
    let funded_period_duration = denom_reward_config
        .reward_emission_config
        .get_funded_period_duration(amount)?;
    let funded_period_units = match funded_period_duration {
        Duration::Height(h) => h,
        Duration::Time(t) => t,
    };

    denom_reward_config.distribution_expiration = match denom_reward_config.distribution_expiration
    {
        // if this is the first funding of the denom, the new expiration is the funded period duration
        // from the current block
        Expiration::Never {} => funded_period_duration.after(&env.block),
        // otherwise we add the duration units to the existing expiration
        Expiration::AtHeight(h) => {
            if h <= env.block.height {
                // expiration is the funded duration after current block
                Expiration::AtHeight(env.block.height + funded_period_units)
            } else {
                // if the previous expiration had not yet expired, we extend
                // the current rewards period by the newly funded duration
                Expiration::AtHeight(h + funded_period_units)
            }
        }
        Expiration::AtTime(t) => {
            if t.seconds() <= env.block.time.seconds() {
                // expiration is the funded duration after current block time
                Expiration::AtTime(t.plus_seconds(funded_period_units))
            } else {
                // if the previous expiration had not yet expired, we extend
                // the current rewards period by the newly funded duration
                Expiration::AtTime(env.block.time.plus_seconds(funded_period_units))
            }
        }
    };

    denom_reward_config = denom_reward_config
        .bump_funding_date(&env.block)
        .bump_last_update(&env.block);
    denom_reward_config.funded_amount += amount;

    REWARD_DENOM_CONFIGS.save(
        deps.storage,
        denom_reward_config.to_str_denom(),
        &denom_reward_config,
    )?;

    Ok(Response::default())
}

pub fn execute_claim(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
) -> Result<Response, ContractError> {
    // Update the rewards information for the sender.
    update_rewards(&mut deps, &env, &info.sender, vec![denom.to_string()])?;

    // Get the checked denom for the string based denom
    let denom_reward_config = REWARD_DENOM_CONFIGS.load(deps.storage, denom.to_string())?;

    let mut amount = Uint128::zero();

    USER_REWARD_CONFIGS.update(
        deps.storage,
        info.sender.clone(),
        |config| -> Result<_, ContractError> {
            let mut user_reward_config = config.unwrap_or_default();
            // updating the map returns the previous value if it existed.
            // we set the value to zero and store it in the amount defined before the update.
            amount = user_reward_config
                .pending_denom_rewards
                .insert(denom, Uint128::zero())
                .unwrap_or_default();
            Ok(user_reward_config)
        },
    )?;

    if amount.is_zero() {
        return Err(ContractError::NoRewardsClaimable {});
    }

    Ok(Response::new()
        .add_message(get_transfer_msg(
            info.sender.clone(),
            amount,
            denom_reward_config.denom,
        )?)
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

/// Returns the appropriate CosmosMsg for transferring the reward token.
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
pub fn update_rewards(
    deps: &mut DepsMut,
    env: &Env,
    addr: &Addr,
    denoms: Vec<String>,
) -> StdResult<()> {
    println!("[CONTRACT-UPDATE-REWARDS] Updating rewards for {:?}", addr);
    for denom in denoms {
        let reward_config = REWARD_DENOM_CONFIGS.load(deps.storage, denom.clone())?;

        // first, we calculate the rewards per token and update them
        let rewards_per_token = get_rewards_per_token(
            &reward_config,
            env,
            &reward_config.vp_contract,
            deps.as_ref(),
        )?;

        // update the cumulative rewards per token with latest rewards per token
        CUMULATIVE_REWARDS_PER_TOKEN.save(deps.storage, denom.clone(), &rewards_per_token)?;

        // then we calculate the rewards earned since last user action
        let earned_rewards = get_accrued_rewards_since_last_user_action(
            deps.as_ref(),
            env,
            addr,
            rewards_per_token,
            &reward_config.vp_contract,
            vec![denom.clone()],
        )?;

        // reflect the earned rewards in the user's reward config
        USER_REWARD_CONFIGS.update(deps.storage, addr.clone(), |config| -> StdResult<_> {
            // if user does not yet have a config, we create a new one
            let mut user_reward_config = config.unwrap_or_default();

            // get the pre-existing pending reward amount for the denom
            let previous_pending_denom_reward_amount = *user_reward_config
                .pending_denom_rewards
                .get(&denom)
                .unwrap_or(&Uint128::zero());

            // get the amount of newly earned rewards for the denom
            let earned_rewards_amount = earned_rewards.get(&denom).cloned().unwrap_or_default();

            user_reward_config.pending_denom_rewards.insert(
                denom.clone(),
                previous_pending_denom_reward_amount + earned_rewards_amount,
            );

            user_reward_config
                .user_reward_per_token
                .insert(denom.clone(), rewards_per_token);

            Ok(user_reward_config)
        })?;

        // Update the last update expiration in the DenomRewardConfig
        REWARD_DENOM_CONFIGS.update(deps.storage, denom.clone(), |config| -> StdResult<_> {
            match config {
                Some(rc) => Ok(rc.bump_last_update(&env.block)),
                None => Err(StdError::generic_err("Denom config not found")),
            }
        })?;
    }

    Ok(())
}

fn get_expiration_diff(a: Expiration, b: Expiration) -> StdResult<u64> {
    match (a, b) {
        (Expiration::AtHeight(a), Expiration::AtHeight(b)) => {
            if a >= b {
                Ok(a - b)
            } else {
                Ok(0)
            }
        }
        (Expiration::AtTime(a), Expiration::AtTime(b)) => {
            if a >= b {
                Ok(a.seconds() - b.seconds())
            } else {
                Ok(0)
            }
        }
        (Expiration::Never {}, Expiration::Never {}) => Ok(0),
        _ => Err(StdError::generic_err(format!(
            "incompatible expirations: got a {:?}, b {:?}",
            a, b
        ))),
    }
}

fn get_rewards_per_token(
    reward_config: &DenomRewardConfig,
    env: &Env,
    vp_contract: &Addr,
    deps: Deps,
) -> StdResult<Uint256> {
    let crpt = CUMULATIVE_REWARDS_PER_TOKEN.load(deps.storage, reward_config.to_str_denom())?;

    // query the current total voting power from the voting power contract
    let total_power = get_total_voting_power(deps, env, vp_contract)?;

    let last_time_reward_applicable =
        reward_config.get_latest_reward_distribution_expiration_date(&env.block);

    let expiration_diff = Uint128::from(get_expiration_diff(
        last_time_reward_applicable,
        reward_config.last_update,
    )?);

    if total_power.is_zero() {
        Ok(crpt)
    } else {
        let reward_rate_time_units = match reward_config.reward_emission_config.reward_rate_time {
            Duration::Height(h) => h,
            Duration::Time(t) => t,
        };

        let complete_distribution_periods =
            expiration_diff.checked_div(Uint128::try_from(reward_rate_time_units).unwrap())?;

        let numerator = reward_config
            .reward_emission_config
            .reward_rate_emission
            .full_mul(complete_distribution_periods)
            .checked_mul(scale_factor())?;
        let denominator = Uint256::from(total_power);
        Ok(crpt + numerator.checked_div(denominator)?)
    }
}

pub fn get_accrued_rewards_since_last_user_action(
    deps: Deps,
    env: &Env,
    addr: &Addr,
    reward_per_token: Uint256,
    vp_contract: &Addr,
    denoms: Vec<String>,
) -> StdResult<HashMap<String, Uint128>> {
    // Get the user's voting power at the current height.
    let voting_power = Uint256::from(get_voting_power(deps, env, vp_contract, addr)?);

    let mut entitled_rewards: HashMap<String, Uint128> = HashMap::new();

    let user_reward_config = USER_REWARD_CONFIGS
        .load(deps.storage, addr.clone())
        .unwrap_or_default();

    for denom in denoms.iter() {
        let user_last_reward_per_token = user_reward_config
            .user_reward_per_token
            .get(denom)
            .cloned()
            .unwrap_or_default();

        // Calculate the difference between the current reward per token value and the user's latest reward per token value
        let reward_factor = reward_per_token.checked_sub(user_last_reward_per_token)?;

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

pub fn execute_update_denom_emission_config(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
    new_emission_rate: Uint128,
    new_emission_time: Duration,
) -> Result<Response, ContractError> {
    // Ensure the sender is the owner.
    cw_ownable::assert_owner(deps.storage, &info.sender)?;

    if let Duration::Height(0) | Duration::Time(0) = new_emission_time {
        return Err(ContractError::ZeroRewardDuration {});
    }

    let mut reward_config = REWARD_DENOM_CONFIGS.load(deps.storage, denom.to_string())?;

    // Ensure that the current reward period has ended.
    // TODO: look whether into whether this is necessary
    reward_config.validate_period_finish_expiration_if_set(&env.block)?;
    reward_config.reward_emission_config = RewardEmissionConfig {
        reward_rate_emission: new_emission_rate,
        reward_rate_time: new_emission_time,
    };
    REWARD_DENOM_CONFIGS.save(deps.storage, denom, &reward_config)?;

    Ok(Response::new()
        .add_attribute("action", "update_reward_duration")
        .add_attribute("new_emission_rate", new_emission_rate.to_string())
        .add_attribute("new_emission_time", new_emission_time.to_string()))
}

pub fn scale_factor() -> Uint256 {
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
        QueryMsg::DenomRewardConfig { denom } => {
            let config = REWARD_DENOM_CONFIGS.load(deps.storage, denom)?;
            Ok(to_json_binary(&config)?)
        }
    }
}

pub fn query_info(deps: Deps, _env: Env) -> StdResult<InfoResponse> {
    let reward_configs = REWARD_DENOM_CONFIGS
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| item.map(|(_, v)| v))
        .collect::<StdResult<Vec<_>>>()?;
    Ok(InfoResponse { reward_configs })
}

pub fn query_pending_rewards(
    deps: Deps,
    env: Env,
    addr: String,
) -> StdResult<PendingRewardsResponse> {
    let addr = deps.api.addr_validate(&addr)?;
    let reward_configs = REWARD_DENOM_CONFIGS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    let mut pending_rewards: HashMap<String, Uint128> = HashMap::new();

    for (denom, reward_config) in reward_configs {
        let reward_per_token =
            get_rewards_per_token(&reward_config, &env, &reward_config.vp_contract, deps)?;

        let earned_rewards = get_accrued_rewards_since_last_user_action(
            deps,
            &env,
            &addr,
            reward_per_token,
            &reward_config.vp_contract,
            vec![denom.to_string()],
        )?;

        let user_reward_config = USER_REWARD_CONFIGS
            .load(deps.storage, addr.clone())
            .unwrap_or_default();

        let default_amt = Uint128::zero();
        let earned_amount = earned_rewards.get(&denom).unwrap_or(&default_amt);
        let existing_amount = user_reward_config
            .pending_denom_rewards
            .get(&denom)
            .unwrap_or(&default_amt);
        pending_rewards.insert(denom, *earned_amount + *existing_amount);
    }

    let pending_rewards_response = PendingRewardsResponse {
        address: addr.to_string(),
        pending_rewards,
    };
    Ok(pending_rewards_response)
}
