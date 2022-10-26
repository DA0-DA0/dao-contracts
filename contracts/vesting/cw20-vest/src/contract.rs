use std::collections::BinaryHeap;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, DepsMut, Env, MessageInfo, Response, Uint128, Deps, StdResult, Binary, Timestamp, Addr, from_slice, StdError, to_vec};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, Schedule, MigrateMsg};
use crate::state::{Config, CONFIG, SCHEDULES, ACTIVATED, Vest};
use crate::query_helpers::query_balance;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw20-vest";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let owner = msg
        .owner
        .as_deref()
        .map(|h| deps.api.addr_validate(h))
        .transpose()?;

    let manager = msg
        .manager
        .as_deref()
        .map(|h| deps.api.addr_validate(h))
        .transpose()?;

    let token_address = deps.api.addr_validate(&msg.token_address)?;
    let stake_address = deps.api.addr_validate(&msg.stake_address)?;
    let vest_total = save_schedules(deps.branch(), msg.schedules)?;
    let config = Config {
        owner,
        manager,
        token_address,
        stake_address: stake_address.clone(),
        vest_total: vest_total,
    };
    CONFIG.save(deps.storage, &config)?;
    ACTIVATED.save(deps.storage, &false, env.block.height)?;
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => execute::receive(deps, env, info, msg),
        ExecuteMsg::Vest {} => execute::vest(deps, env, info),
        ExecuteMsg::Claim {} => execute::claim(deps, env, info),
        ExecuteMsg::UpdateConfig {
            owner,
            manager,
        } => execute::update_config(deps, env, info, owner, manager),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetVestingStatusAtHeight { address, height } =>
            to_binary(&query::vesting_status_at_height(deps, env, address, height)?),
        QueryMsg::GetFundingStatusAtHeight { height } => 
            to_binary(&query::funding_status_at_height(deps, env, height)?),
        QueryMsg::GetVestingSchedule { address } => 
            to_binary(&query::vesting_schedule(deps, env, address)?),
        QueryMsg::GetConfig {} => 
            to_binary(&query::config(deps)?),
    }
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    use serde::{Deserialize, Serialize};

    // Set contract to version to latest
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    #[derive(Serialize, Deserialize, Clone)]
    struct BetaConfig {
        pub admin: Addr,
        pub token_address: Addr,
        pub stake_address: Addr,
        pub vest_total: Uint128,
    }

    match msg {
        MigrateMsg::FromBeta { manager } => {
            let data = deps
                .storage
                .get(b"config")
                .ok_or_else(|| StdError::not_found("config"))?;
            let beta_config: BetaConfig = from_slice(&data)?;
            let new_config = Config {
                owner: Some(beta_config.admin),
                manager: manager
                    .map(|human| deps.api.addr_validate(&human))
                    .transpose()?,
                token_address: beta_config.token_address,
                stake_address: beta_config.stake_address,
                vest_total: beta_config.vest_total,
            };
            deps.storage.set(b"config", &to_vec(&new_config)?);
            Ok(Response::default())
        }
        MigrateMsg::FromCompatible {} => Ok(Response::default()),
    }
}

fn calculate_cumulative_vest(env: &Env, vests: BinaryHeap<Vest>) -> StdResult<Uint128> {
    let now = env.block.time;
    let vests = vests.into_sorted_vec();
    let pos = vests.iter().position(|v| now < v.expiration);
    match pos {
        Some(0) => Ok(0u128.into()),
        Some(idx) => interpolate_vests(&vests[idx - 1], &vests[idx], now),
        None => Ok(vests.last().unwrap().amount),
    }
}

fn interpolate_vests(left: &Vest, right: &Vest, now: Timestamp) -> StdResult<Uint128> {
    let rise = right.amount.checked_sub(left.amount)?;
    let run = right.expiration.seconds() - left.expiration.seconds();
    let run: Uint128 = run.into();
    // this function should never be called on a two points with the same timestamp.
    if run.is_zero() {
        return Err(StdError::GenericErr { msg: "Bad invariant, left and right should have different timestamps".to_string() })
    }
    let x = now.seconds() - left.expiration.seconds();
    let x: Uint128 = x.into();
    let y = left.amount.checked_add(x.checked_mul(rise.checked_div(run)?)?)?;
    Ok(y)
}

fn save_schedules(
    deps: DepsMut,
    schedules: Vec<Schedule>,
) -> Result<Uint128, ContractError> {

    let mut vest_total = Uint128::zero();
    for Schedule { address, vests } in schedules.into_iter() {
        let addr = deps.api.addr_validate(&address)?;

        if vests.is_empty() {
            return Err(ContractError::BadConfig {});
        }
        let vests: BinaryHeap<Vest> = vests.into_iter().collect();
        let vests_vec = vests.clone().into_sorted_vec();
        
        let first_vest = vests_vec.first().unwrap();
        if !first_vest.amount.is_zero() {
            return Err(ContractError::VestScheduleDoesNotContainInitialZeroPoint {
                amount1: first_vest.amount,
                time1: first_vest.expiration.seconds(),
            });
        }

        // check that amounts are monotonically increasing
        // .into_sorted_vec() gives us an ascending vector so we don't need vests to be BinaryHeap<Reverse<Vest>>
        for window in vests_vec.windows(2) {
            if let [vest, next_vest] = window {
                if next_vest.amount < vest.amount {
                    return Err(ContractError::VestScheduleNotMonotonicallyIncreasing {
                        amount1: vest.amount,
                        time1: vest.expiration.seconds(),
                        amount2: next_vest.amount,
                        time2: next_vest.expiration.seconds(),
                    });
                }
            }
        }

        // check that there are no malformed cliffs (cliffs defined by more than two points at the same timestamp)
        for window in vests_vec.windows(3) {
            if let [vest_a, vest_b, vest_c] = window {
                if vest_a.expiration == vest_b.expiration && vest_b.expiration == vest_c.expiration {
                    return Err(ContractError::VestScheduleFeaturesMalformedCliff {
                        amount1: vest_a.amount,
                        time1: vest_a.expiration.seconds(),
                        amount2: vest_b.amount,
                        time2: vest_b.expiration.seconds(),
                        amount3: vest_c.amount,
                        time3: vest_c.expiration.seconds(),
                    });
                }
            }
        }
        vest_total = vest_total.checked_add(vests.peek().unwrap().amount)?;
        SCHEDULES.save(deps.storage, addr, &vests)?;
    }

    Ok(vest_total)
}

pub mod execute {
    use super::*;
    use cosmwasm_std::{WasmMsg, from_binary, QueryRequest, WasmQuery};
    use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, Cw20Contract};
    use cw20_stake::msg::{StakedValueResponse, StakedBalanceAtHeightResponse};
    use crate::{state::{CLAIMS, SCHEDULES, MAX_CLAIMS, CLAIMED_TOTAL}, query_helpers::query_staking_config, msg::ReceiveMsg};

    pub fn receive(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        wrapper: Cw20ReceiveMsg,
    ) -> Result<Response, ContractError> {
        let config = CONFIG.load(deps.storage)?;
        if info.sender != config.token_address {
            return Err(ContractError::InvalidToken {
                received: info.sender,
                expected: config.token_address,
            });
        }
        let msg: ReceiveMsg = from_binary(&wrapper.msg)?;
        match msg {
            ReceiveMsg::Fund {} => fund(deps, env, info, config),
        }
    }

    pub fn fund(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        config: Config,
    ) -> Result<Response, ContractError> {
        let balance = query_balance(deps.as_ref(), &config.token_address, &env.contract.address)?;
        if balance < config.vest_total {
            return Ok(Response::new()
                .add_attribute("action", "receive_fund")
                .add_attribute("from", info.sender)
                .add_attribute("activated", "false"))
        }

        ACTIVATED.save(deps.storage, &true, env.block.height)?;
        let token_address = Cw20Contract(config.token_address);
        let msg = token_address.call(Cw20ExecuteMsg::Send {
            amount: balance,
            contract: config.stake_address.to_string(),
            msg: to_binary(&cw20_stake::msg::ReceiveMsg::Stake {})?,
        })?;
        Ok(Response::new()
            .add_message(msg)
            .add_attribute("action", "receive_fund")
            .add_attribute("from", info.sender)
            .add_attribute("activated", "true"))
    }

    pub fn vest(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
        if !ACTIVATED.load(deps.storage)? {
            return Err(ContractError::Unfunded {})
        }

        let config = CONFIG.load(deps.storage)?;
        let vests = SCHEDULES.load(deps.storage, info.sender.clone())?;
        let claimed = CLAIMED_TOTAL.may_load(deps.storage, info.sender.clone())?.unwrap_or_default();

        let vested = calculate_cumulative_vest(&env, vests)?;
        let claimable = vested.checked_sub(claimed)?;
        let total_payout = claimed.checked_add(claimable)?;

        if claimable.is_zero() {
            return Ok(Response::new()
                .add_attribute("action", "vest")
                .add_attribute("from", info.sender)
                .add_attribute("amount", claimable));
        }

        CLAIMED_TOTAL.save(deps.storage, info.sender.clone(), &total_payout, env.block.height)?;

        // Need to calculate what we'll receive back in staking rewards.
        let StakedValueResponse { value } = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: config.stake_address.to_string(),
            msg: to_binary(&cw20_stake::msg::QueryMsg::StakedValue { address: env.contract.address.to_string() })?,
        }))?;
        let StakedBalanceAtHeightResponse { balance, height } = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: config.stake_address.to_string(),
            msg: to_binary(&cw20_stake::msg::QueryMsg::StakedBalanceAtHeight { address: env.contract.address.to_string(), height: None })?,
        }))?;
        let amount_to_claim = if !value.is_zero() && !balance.is_zero() {
            value
                .checked_div(balance)
                .map_err(|e| ContractError::DivideByZero(e))?
                .checked_mul(claimable)
                .map_err(|e| ContractError::Overflow(e))?
        } else {
            Uint128::zero()
        };

        let unstake_msg = WasmMsg::Execute {
            contract_addr: config.stake_address.to_string(),
            msg: to_binary(&cw20_stake::msg::ExecuteMsg::Unstake {
                amount: amount_to_claim,
            })?,
            funds: vec![],
        };

        let staking_config = query_staking_config(deps.as_ref(), &config.stake_address)?;
        match staking_config.unstaking_duration {
            None => {
                let transfer_msg = WasmMsg::Execute {
                    contract_addr: config.token_address.to_string(),
                    msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                        recipient: info.sender.to_string(),
                        amount: amount_to_claim,
                    })?,
                    funds: vec![],
                };
                Ok(Response::new()
                    .add_message(unstake_msg)
                    .add_message(transfer_msg)
                    .add_attribute("action", "vest")
                    .add_attribute("from", info.sender)
                    .add_attribute("amount", amount_to_claim)
                    .add_attribute("claim_duration", "None"))
            }
            Some(duration) => {
                let outstanding_claims = CLAIMS.query_claims(deps.as_ref(), &info.sender)?.claims;
                if outstanding_claims.len() >= MAX_CLAIMS as usize {
                    return Err(ContractError::TooManyClaims {});
                }
    
                CLAIMS.create_claim(
                    deps.storage,
                    &info.sender,
                    amount_to_claim,
                    duration.after(&env.block),
                )?;
                Ok(Response::new()
                    .add_message(unstake_msg)
                    .add_attribute("action", "vest")
                    .add_attribute("from", info.sender)
                    .add_attribute("amount", amount_to_claim)
                    .add_attribute("claim_duration", format!("{}", duration)))
            }
        }
    }

    pub fn claim(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
        if !ACTIVATED.load(deps.storage)? {
            return Err(ContractError::Unfunded {})
        }

        let release = CLAIMS.claim_tokens(deps.storage, &info.sender, &env.block, None)?;
        if release.is_zero() {
            return Err(ContractError::NothingToClaim {});
        }
        let config = CONFIG.load(deps.storage)?;
        let balance = query_balance(deps.as_ref(), &config.token_address, &env.contract.address)?;

        let messages = if balance < release {
            vec![WasmMsg::Execute {
                contract_addr: config.stake_address.to_string(),
                msg: to_binary(&cw20_stake::msg::ExecuteMsg::Claim {})?,
                funds: vec![],
            }]
        } else {
            vec![]
        };

        let transfer_msg = cosmwasm_std::WasmMsg::Execute {
            contract_addr: config.token_address.to_string(),
            msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount: release,
            })?,
            funds: vec![],
        };
        messages.push(transfer_msg);

        Ok(Response::new()
            .add_messages(messages)
            .add_attribute("action", "claim")
            .add_attribute("from", info.sender)
            .add_attribute("amount", release))
    }

    pub fn update_config(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        new_owner: Option<String>,
        new_manager: Option<String>
    ) -> Result<Response, ContractError> {
        let new_owner = new_owner
            .map(|new_owner| deps.api.addr_validate(&new_owner))
            .transpose()?;
        let new_manager = new_manager
            .map(|new_manager| deps.api.addr_validate(&new_manager))
            .transpose()?;

        let mut config: Config = CONFIG.load(deps.storage)?;
        if config.owner.as_ref().map_or(true, |owner| &info.sender != owner) && config.manager.as_ref().map_or(true, |manager| &info.sender != manager) {
            return Err(ContractError::Unauthorized {});
        };
        if Some(info.sender) != config.owner && new_owner != config.owner {
            return Err(ContractError::OnlyOwnerCanChangeOwner {});
        };

        config.owner = new_owner;
        config.manager = new_manager;

        CONFIG.save(deps.storage, &config)?;
        Ok(Response::new()
            .add_attribute("action", "update_config")
            .add_attribute(
                "owner",
                config
                    .owner
                    .map(|a| a.to_string())
                    .unwrap_or_else(|| "None".to_string()),
            )
            .add_attribute(
                "manager",
                config
                    .manager
                    .map(|a| a.to_string())
                    .unwrap_or_else(|| "None".to_string()),
            ))
    }

}

pub mod query {
    use super::*;

    use crate::{msg::{GetVestingStatusAtHeightResponse, GetFundingStatusAtHeightResponse, GetVestingScheduleResponse}, state::{CLAIMS, CLAIMED_TOTAL}};

    pub fn vesting_status_at_height(deps: Deps, env: Env, addr: String, height: Option<u64>) -> StdResult<GetVestingStatusAtHeightResponse> {
        let addr = deps.api.addr_validate(&addr)?;
        let height = height.unwrap_or(env.block.height);

        let vests = SCHEDULES.load(deps.storage, addr.clone())?;
        let claims = CLAIMS.query_claims(deps, &addr)?.claims;
        let claimed = CLAIMED_TOTAL.may_load_at_height(deps.storage, addr.clone(), height)?.unwrap_or_default();

        // vests is validated to not be empty
        let vested_u_unvested = vests.peek().unwrap().amount;
        let vested = calculate_cumulative_vest(&env, vests)?;
        let unvested = vested_u_unvested.checked_sub(vested)?;
        let vested_staked = vested.checked_sub(claimed)?;

        let mut vested_unstaking = Uint128::from(0u128);
        let mut vested_unstaked = Uint128::from(0u128);
        for claim in claims {
            if claim.release_at.is_expired(&env.block) {
                vested_unstaked = vested_unstaked.checked_add(claim.amount)?;
            } else {
                vested_unstaking = vested_unstaking.checked_add(claim.amount)?;
            }
        }

        let vested_claimed = vested_u_unvested
            .checked_sub(unvested)?
            .checked_sub(vested_staked)?
            .checked_sub(vested_unstaking)?
            .checked_sub(vested_unstaked)?;
        
        Ok(GetVestingStatusAtHeightResponse { 
            vested_claimed,
            vested_unstaked,
            vested_unstaking,
            vested_staked,
            unvested_staked: unvested,
            height,
        })
    }

    pub fn funding_status_at_height(deps: Deps, env: Env, height: Option<u64>) -> StdResult<GetFundingStatusAtHeightResponse> {
        let height = height.unwrap_or(env.block.height);
        let activated = ACTIVATED.may_load_at_height(deps.storage, height)?.unwrap_or(false);
        Ok(GetFundingStatusAtHeightResponse {
            activated: activated,
            height,
        })
    }


    pub fn config(deps: Deps) -> StdResult<Config> {
        Ok(CONFIG.load(deps.storage)?)
    }

    pub fn vesting_schedule(deps: Deps, env: Env, address: String) -> StdResult<GetVestingScheduleResponse> {
        let address = deps.api.addr_validate(&address)?;
        let schedule = SCHEDULES.load(deps.storage, address)?;
        Ok(GetVestingScheduleResponse { schedule: schedule.into_sorted_vec() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    use cosmwasm_std::{Empty, Addr};
    use cw20::Cw20Coin;
    use cw_multi_test::{App, ContractWrapper, Contract, next_block, Executor};
    use cw_utils::Duration;

    use crate::{msg::{ReceiveMsg, GetVestingStatusAtHeightResponse}, state::Vest};

    #[test]
    fn test_with_unstake_duration() {
        let alice = Addr::unchecked("alice");
        let initial_balances = vec![Cw20Coin {
            address: "tester".to_string(),
            amount: Uint128::from(110u128),
        }];
        let schedule = vec![
            Schedule { 
                address: alice.clone().into(),
                vests: vec![
                    Vest {
                        expiration: Timestamp::from_seconds(20),
                        amount: 10u128.into(),
                    },
                    Vest {
                        expiration: Timestamp::from_seconds(45),
                        amount: 110u128.into(),
                    }
                ],
            }
        ];
        let mut harness = Harness::new(
            initial_balances,
            Some(Duration::Time(10u64)),
            schedule,
        );

        // time = 15
        assert_eq!(
            harness.query_balance(&harness.cw20_vest_addr),
            Uint128::from(0u128),
        );

        // fund the contract
        harness.send_balance(
            &harness.cw20_vest_addr.clone(),
            110u128,
            to_binary(&ReceiveMsg::Fund {}).unwrap()
        );

        // time = 20
        assert_eq!(
            harness.query_balance(&harness.cw20_stake_addr),
            Uint128::from(110u128),
        );
        assert_eq!(
            harness.query_staked_balance(&harness.cw20_vest_addr),
            Uint128::from(110u128),
        );
        assert_eq!(
            harness.query_vesting_status(&alice),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(0u128),
                vested_unstaked: Uint128::from(0u128),
                vested_unstaking: Uint128::from(0u128),
                vested_staked: Uint128::from(10u128),
                unvested_staked: Uint128::from(100u128),
                height: harness.app.block_info().height,
            }
        );

        harness.tick();

        // time = 25
        assert_eq!(
            harness.query_vesting_status(&alice),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(0u128),
                vested_unstaked: Uint128::from(0u128),
                vested_unstaking: Uint128::from(0u128),
                vested_staked: Uint128::from(30u128),
                unvested_staked: Uint128::from(80u128),
                height: harness.app.block_info().height,
            }
        );
        assert_eq!(
            harness.query_staked_balance(&harness.cw20_vest_addr),
            Uint128::from(110u128),
        );

        harness.vest(&alice);

        // time = 30
        assert_eq!(
            harness.query_vesting_status(&alice),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(0u128),
                vested_unstaked: Uint128::from(0u128),
                vested_unstaking: Uint128::from(30u128),
                vested_staked: Uint128::from(20u128),
                unvested_staked: Uint128::from(60u128),
                height: harness.app.block_info().height,
            }
        );
        assert_eq!(
            harness.query_staked_balance(&harness.cw20_vest_addr),
            Uint128::from(80u128),
        );

        harness.tick();
        harness.tick();

        // time = 40
        assert_eq!(
            harness.query_vesting_status(&alice),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(0u128),
                vested_unstaked: Uint128::from(30u128),
                vested_unstaking: Uint128::from(0u128),
                vested_staked: Uint128::from(60u128),
                unvested_staked: Uint128::from(20u128),
                height: harness.app.block_info().height,
            }
        );

        harness.claim(&Addr::unchecked(&alice));
        assert_eq!(
            harness.query_vesting_status(&alice),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(30u128),
                vested_unstaked: Uint128::from(0u128),
                vested_unstaking: Uint128::from(0u128),
                vested_staked: Uint128::from(80u128),
                unvested_staked: Uint128::from(0u128),
                height: harness.app.block_info().height,
            }
        );
        assert_eq!(
            harness.query_balance(&alice),
            Uint128::from(30u128),
        );
    }

    #[test]
    fn test_without_unstake_duration() {
        let alice = Addr::unchecked("alice");
        let initial_balances = vec![Cw20Coin {
            address: "tester".to_string(),
            amount: Uint128::from(110u128),
        }];
        let schedule = vec![
            Schedule {
                address: alice.clone().into(),
                vests: vec![
                    Vest {
                        expiration: Timestamp::from_seconds(20),
                        amount: 10u128.into(),
                    },
                    Vest {
                        expiration: Timestamp::from_seconds(45),
                        amount: 110u128.into(),
                    }
                ],
            }
        ];
        let mut harness = Harness::new(
            initial_balances,
            None,
            schedule,
        );

        // time = 15
        assert_eq!(
            harness.query_balance(&harness.cw20_vest_addr),
            Uint128::from(0u128),
        );

        // fund the contract
        harness.send_balance(
            &harness.cw20_vest_addr.clone(),
            110u128,
            to_binary(&ReceiveMsg::Fund {}).unwrap()
        );

        // time = 20
        assert_eq!(
            harness.query_balance(&harness.cw20_stake_addr),
            Uint128::from(110u128),
        );
        assert_eq!(
            harness.query_staked_balance(&harness.cw20_vest_addr),
            Uint128::from(110u128),
        );
        assert_eq!(
            harness.query_vesting_status(&alice),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(0u128),
                vested_unstaked: Uint128::from(0u128),
                vested_unstaking: Uint128::from(0u128),
                vested_staked: Uint128::from(10u128),
                unvested_staked: Uint128::from(100u128),
                height: harness.app.block_info().height,
            }
        );

        harness.tick();

        // time = 25
        assert_eq!(
            harness.query_vesting_status(&alice),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(0u128),
                vested_unstaked: Uint128::from(0u128),
                vested_unstaking: Uint128::from(0u128),
                vested_staked: Uint128::from(30u128),
                unvested_staked: Uint128::from(80u128),
                height: harness.app.block_info().height,
            }
        );
        assert_eq!(
            harness.query_staked_balance(&harness.cw20_vest_addr),
            Uint128::from(110u128),
        );

        harness.vest(&alice);
        // time = 30
        assert_eq!(
            harness.query_vesting_status(&alice),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(30u128),
                vested_unstaked: Uint128::from(0u128),
                vested_unstaking: Uint128::from(0u128),
                vested_staked: Uint128::from(20u128),
                unvested_staked: Uint128::from(60u128),
                height: harness.app.block_info().height,
            }
        );
        assert_eq!(
            harness.query_balance(&alice),
            Uint128::from(30u128),
        );
    }

    #[test]
    #[should_panic(expected = "Vest amounts not monotonically increasing over time: Vest amount 100 at time 20 is greater than amount 50 at time 45")]
    fn test_bad_schedule() {
        let alice = Addr::unchecked("alice");
        let initial_balances = vec![Cw20Coin {
            address: "tester".to_string(),
            amount: Uint128::from(110u128),
        }];
        let schedule = vec![
            Schedule { 
                address: alice.clone().into(),
                vests: vec![
                    Vest {
                        expiration: Timestamp::from_seconds(20),
                        amount: 100u128.into(),
                    },
                    Vest {
                        expiration: Timestamp::from_seconds(45),
                        amount: 50u128.into(),
                    }
                ],
            }
        ];
        Harness::new(
            initial_balances,
            None,
            schedule,
        );
    }

    #[test]
    fn test_multi_user_with_unstake_duration() {
        let alice = Addr::unchecked("alice");
        let bob = Addr::unchecked("bob");
        let initial_balances = vec![Cw20Coin {
            address: "tester".to_string(),
            amount: Uint128::from(220u128),
        }];
        let schedule = vec![
            Schedule { 
                address: alice.clone().into(),
                vests: vec![
                    Vest {
                        expiration: Timestamp::from_seconds(20),
                        amount: 10u128.into(),
                    },
                    Vest {
                        expiration: Timestamp::from_seconds(45),
                        amount: 110u128.into(),
                    }
                ],
            },
            Schedule { 
                address: bob.clone().into(),
                vests: vec![
                    Vest {
                        expiration: Timestamp::from_seconds(25),
                        amount: 10u128.into(),
                    },
                    Vest {
                        expiration: Timestamp::from_seconds(50),
                        amount: 110u128.into(),
                    }
                ],
            }
        ];
        let mut harness = Harness::new(
            initial_balances,
            Some(Duration::Time(10u64)),
            schedule,
        );

        // time = 15
        assert_eq!(
            harness.query_balance(&harness.cw20_vest_addr),
            Uint128::from(0u128),
        );

        // fund the contract
        harness.send_balance(
            &harness.cw20_vest_addr.clone(),
            220u128,
            to_binary(&ReceiveMsg::Fund {}).unwrap()
        );

        // time = 20
        assert_eq!(
            harness.query_balance(&harness.cw20_stake_addr),
            Uint128::from(220u128),
        );
        assert_eq!(
            harness.query_staked_balance(&harness.cw20_vest_addr),
            Uint128::from(220u128),
        );
        assert_eq!(
            harness.query_vesting_status(&alice),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(0u128),
                vested_unstaked: Uint128::from(0u128),
                vested_unstaking: Uint128::from(0u128),
                vested_staked: Uint128::from(10u128),
                unvested_staked: Uint128::from(100u128),
                height: harness.app.block_info().height,
            }
        );
        assert_eq!(
            harness.query_vesting_status(&bob),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(0u128),
                vested_unstaked: Uint128::from(0u128),
                vested_unstaking: Uint128::from(0u128),
                vested_staked: Uint128::from(0u128),
                unvested_staked: Uint128::from(110u128),
                height: harness.app.block_info().height,
            }
        );

        harness.tick();

        // time = 25
        assert_eq!(
            harness.query_vesting_status(&alice),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(0u128),
                vested_unstaked: Uint128::from(0u128),
                vested_unstaking: Uint128::from(0u128),
                vested_staked: Uint128::from(30u128),
                unvested_staked: Uint128::from(80u128),
                height: harness.app.block_info().height,
            }
        );
        assert_eq!(
            harness.query_vesting_status(&bob),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(0u128),
                vested_unstaked: Uint128::from(0u128),
                vested_unstaking: Uint128::from(0u128),
                vested_staked: Uint128::from(10u128),
                unvested_staked: Uint128::from(100u128),
                height: harness.app.block_info().height,
            }
        );
        assert_eq!(
            harness.query_staked_balance(&harness.cw20_vest_addr),
            Uint128::from(220u128),
        );

        harness.vest(&alice);

        // time = 30
        assert_eq!(
            harness.query_vesting_status(&alice),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(0u128),
                vested_unstaked: Uint128::from(0u128),
                vested_unstaking: Uint128::from(30u128),
                vested_staked: Uint128::from(20u128),
                unvested_staked: Uint128::from(60u128),
                height: harness.app.block_info().height,
            }
        );
        assert_eq!(
            harness.query_vesting_status(&bob),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(0u128),
                vested_unstaked: Uint128::from(0u128),
                vested_unstaking: Uint128::from(0u128),
                vested_staked: Uint128::from(30u128),
                unvested_staked: Uint128::from(80u128),
                height: harness.app.block_info().height,
            }
        );
        assert_eq!(
            harness.query_staked_balance(&harness.cw20_vest_addr),
            Uint128::from(190u128),
        );

        harness.vest(&bob);
        assert_eq!(
            harness.query_vesting_status(&alice),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(0u128),
                vested_unstaked: Uint128::from(30u128),
                vested_unstaking: Uint128::from(0u128),
                vested_staked: Uint128::from(40u128),
                unvested_staked: Uint128::from(40u128),
                height: harness.app.block_info().height,
            }
        );
        assert_eq!(
            harness.query_vesting_status(&bob),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(0u128),
                vested_unstaked: Uint128::from(0u128),
                vested_unstaking: Uint128::from(30u128),
                vested_staked: Uint128::from(20u128),
                unvested_staked: Uint128::from(60u128),
                height: harness.app.block_info().height,
            }
        );
        assert_eq!(
            harness.query_staked_balance(&harness.cw20_vest_addr),
            Uint128::from(160u128),
        );

        harness.tick();

        // time = 40
        assert_eq!(
            harness.query_vesting_status(&alice),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(0u128),
                vested_unstaked: Uint128::from(30u128),
                vested_unstaking: Uint128::from(0u128),
                vested_staked: Uint128::from(60u128),
                unvested_staked: Uint128::from(20u128),
                height: harness.app.block_info().height,
            }
        );
        assert_eq!(
            harness.query_vesting_status(&bob),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(0u128),
                vested_unstaked: Uint128::from(30u128),
                vested_unstaking: Uint128::from(0u128),
                vested_staked: Uint128::from(40u128),
                unvested_staked: Uint128::from(40u128),
                height: harness.app.block_info().height,
            }
        );

        harness.claim(&alice);
        assert_eq!(
            harness.query_vesting_status(&alice),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(30u128),
                vested_unstaked: Uint128::from(0u128),
                vested_unstaking: Uint128::from(0u128),
                vested_staked: Uint128::from(80u128),
                unvested_staked: Uint128::from(0u128),
                height: harness.app.block_info().height,
            }
        );
        assert_eq!(
            harness.query_vesting_status(&bob),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(0u128),
                vested_unstaked: Uint128::from(30u128),
                vested_unstaking: Uint128::from(0u128),
                vested_staked: Uint128::from(60u128),
                unvested_staked: Uint128::from(20u128),
                height: harness.app.block_info().height,
            }
        );
        assert_eq!(
            harness.query_balance(&alice),
            Uint128::from(30u128),
        );

        harness.claim(&bob);
        assert_eq!(
            harness.query_vesting_status(&alice),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(30u128),
                vested_unstaked: Uint128::from(0u128),
                vested_unstaking: Uint128::from(0u128),
                vested_staked: Uint128::from(80u128),
                unvested_staked: Uint128::from(0u128),
                height: harness.app.block_info().height,
            }
        );
        assert_eq!(
            harness.query_vesting_status(&bob),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(30u128),
                vested_unstaked: Uint128::from(0u128),
                vested_unstaking: Uint128::from(0u128),
                vested_staked: Uint128::from(80u128),
                unvested_staked: Uint128::from(0u128),
                height: harness.app.block_info().height,
            }
        );
        assert_eq!(
            harness.query_balance(&alice),
            Uint128::from(30u128),
        );
        assert_eq!(
            harness.query_balance(&alice),
            Uint128::from(30u128),
        );
    }

    #[test]
    #[should_panic(expected = "Unfunded")]
    fn test_not_funded() {
        let alice = Addr::unchecked("alice");
        let initial_balances = vec![Cw20Coin {
            address: "tester".to_string(),
            amount: Uint128::from(110u128),
        }];
        let schedule = vec![
            Schedule { 
                address: alice.clone().into(),
                vests: vec![
                    Vest {
                        expiration: Timestamp::from_seconds(20),
                        amount: 10u128.into(),
                    },
                    Vest {
                        expiration: Timestamp::from_seconds(45),
                        amount: 110u128.into(),
                    }
                ],
            }
        ];
        let mut harness = Harness::new(
            initial_balances,
            None,
            schedule,
        );

        // time = 15

        harness.tick();

        // time = 20

        assert_eq!(
            harness.query_vesting_status(&alice),
            GetVestingStatusAtHeightResponse {
                vested_claimed: Uint128::from(0u128),
                vested_unstaked: Uint128::from(0u128),
                vested_unstaking: Uint128::from(0u128),
                vested_staked: Uint128::from(10u128),
                unvested_staked: Uint128::from(100u128),
                height: harness.app.block_info().height,
            }
        );

        harness.vest(&alice);
    }

    struct Harness {
        app: App,
        info: MessageInfo,
        pub cw20_base_addr: Addr,
        pub cw20_stake_addr: Addr,
        pub cw20_vest_addr: Addr,
    }

    impl Harness {
        fn new(
            initial_balances: Vec<Cw20Coin>,
            unstaking_duration: Option<Duration>,
            schedules: Vec<Schedule>,
        ) -> Harness {
            let mut app = App::default();
            let info = MessageInfo {
                sender: Addr::unchecked("tester"),
                funds: vec![],
            };
            app.update_block(|block| {
                block.time = Timestamp::from_seconds(0);
                block.height = 0;
            });

            // Instantiate cw20 contract
            let cw20_base_addr = Self::instantiate_cw20(&mut app, initial_balances);
            app.update_block(next_block);

            // Instantiate staking contract
            let cw20_stake_addr = Self::instantiate_staking(&mut app, cw20_base_addr.clone(), unstaking_duration);
            app.update_block(next_block);

            let cw20_vest_addr = Self::instantiate_vest(&mut app, cw20_base_addr.clone(), cw20_stake_addr.clone(), schedules);
            app.update_block(next_block);

            Harness {
                app,
                info,
                cw20_base_addr,
                cw20_stake_addr,
                cw20_vest_addr,
            }
        }

        fn contract_staking() -> Box<dyn Contract<Empty>> {
            let contract = ContractWrapper::new(
                cw20_stake::contract::execute,
                cw20_stake::contract::instantiate,
                cw20_stake::contract::query,
            );
            Box::new(contract)
        }

        fn contract_cw20() -> Box<dyn Contract<Empty>> {
            let contract = ContractWrapper::new(
                cw20_base::contract::execute,
                cw20_base::contract::instantiate,
                cw20_base::contract::query,
            );
            Box::new(contract)
        }

        fn contract_vest() -> Box<dyn Contract<Empty>> {
            let contract = ContractWrapper::new(
                crate::contract::execute,
                crate::contract::instantiate,
                crate::contract::query,
            );
            Box::new(contract)
        }

        fn instantiate_cw20(app: &mut App, initial_balances: Vec<Cw20Coin>) -> Addr {
            let cw20_id = app.store_code(Self::contract_cw20());
            let msg = cw20_base::msg::InstantiateMsg {
                name: String::from("Test"),
                symbol: String::from("TEST"),
                decimals: 6,
                initial_balances,
                mint: None,
                marketing: None,
            };

            app.instantiate_contract(cw20_id, Addr::unchecked("sender"), &msg, &[], "cw20", None)
                .unwrap()
        }

        fn instantiate_staking(app: &mut App, cw20: Addr, unstaking_duration: Option<Duration>) -> Addr {
            let staking_code_id = app.store_code(Self::contract_staking());
            let msg = cw20_stake::msg::InstantiateMsg {
                owner: Some("owner".to_string()),
                manager: Some("manager".to_string()),
                token_address: cw20.to_string(),
                unstaking_duration,
            };
            app.instantiate_contract(
                staking_code_id,
                Addr::unchecked("admin"),
                &msg,
                &[],
                "staking",
                Some("admin".to_string()),
            )
            .unwrap()
        }

        fn instantiate_vest(app: &mut App, cw20: Addr, cw20_staking: Addr, schedules: Vec<Schedule>) -> Addr {
            let vest_code_id = app.store_code(Self::contract_vest());
            let msg = InstantiateMsg { 
                owner: Some("dao".to_string()),
                manager: Some("dao".to_string()),
                token_address: cw20.to_string(),
                stake_address: cw20_staking.to_string(),
                schedules,
            };
            app.instantiate_contract(
                vest_code_id,
                Addr::unchecked("admin"),
                &msg,
                &[],
                "vest",
                Some("admin".to_string()),
            )
            .unwrap()
        }

        fn tick(&mut self) {
            self.app.update_block(next_block);
        }

        fn send_balance(
            &mut self,
            recipient: &Addr,
            amount: impl Into<Uint128>,
            msg: Binary,
        ) {
            let msg = cw20::Cw20ExecuteMsg::Send {
                contract: recipient.into(),
                amount: amount.into(),
                msg
            };
            self.app.execute_contract(
                self.info.sender.clone(),
                self.cw20_base_addr.clone(),
                &msg,
                &[]).unwrap();
            self.tick();
        }

        fn vest(&mut self, sender: &Addr) {
            let msg = crate::msg::ExecuteMsg::Vest {};
            self.app.execute_contract(sender.clone(), self.cw20_vest_addr.clone(), &msg, &[]).unwrap();
            self.tick();
        }

        fn claim(&mut self, sender: &Addr) {
            let msg = crate::msg::ExecuteMsg::Claim {};
            self.app.execute_contract(sender.clone(), self.cw20_vest_addr.clone(), &msg, &[]).unwrap();
            self.tick();
        }

        fn query_vesting_status(&self, addr: impl Into<String>) -> GetVestingStatusAtHeightResponse {
            let msg = QueryMsg::GetVestingStatusAtHeight { address: addr.into(), height: None };
            self.app.wrap().query_wasm_smart(&self.cw20_vest_addr, &msg).unwrap()
        }

        // fn query_funding_status(&self, addr: impl Into<String>) -> GetFundingStatusAtHeightResponse {
        //     let msg = QueryMsg::GetFundingStatusAtHeight { height: None };
        //     self.app.wrap().query_wasm_smart(&self.cw20_vest_addr, &msg).unwrap()
        // }

        fn query_balance(&self, address: impl Into<String>) -> Uint128 {
            let msg = cw20::Cw20QueryMsg::Balance { address: address.into() };
            let result: cw20::BalanceResponse = self.app.wrap().query_wasm_smart(&self.cw20_base_addr, &msg).unwrap();
            result.balance
        }

        fn query_staked_balance(
            &self,
            address: impl Into<String>,
        ) -> Uint128 {
            let msg = cw20_stake::msg::QueryMsg::StakedBalanceAtHeight {
                address: address.into(),
                height: None,
            };
            let result: cw20_stake::msg::StakedBalanceAtHeightResponse =
                self.app.wrap().query_wasm_smart(&self.cw20_stake_addr, &msg).unwrap();
            result.balance
        }
    }
}