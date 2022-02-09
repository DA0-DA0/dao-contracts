#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, QuerierWrapper, Response, StdResult, Timestamp, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, Denom};
use std::cmp::min;

use crate::error::ContractError;
use crate::error::ContractError::RewardsNotStarted;
use crate::msg::{
    ClaimableRewardsResponse, ExecuteMsg, InfoResponse, InstantiateMsg, QueryMsg, ReceiveMsg,
};
use crate::state::{Config, LastClaim, CONFIG, LAST_CLAIMED};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    validate_instantiate_msg(&env, &msg)?;
    let config = Config {
        start_block: msg.start_block,
        end_block: msg.end_block,
        payment_per_block: msg.payment_per_block,
        total_amount: msg.total_amount,
        denom: msg.denom.clone(),
        staking_contract: deps.api.addr_validate(&*msg.distribution_token)?,
        funded: match msg.denom {
            Denom::Native(_) => true,
            Denom::Cw20(_) => false,
        },
        payment_block_delta: msg.payment_block_delta,
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::default())
}

pub fn validate_instantiate_msg(env: &Env, msg: &InstantiateMsg) -> Result<(), ContractError> {
    if env.block.height > msg.start_block {
        return Err(ContractError::StartBlockAlreadyOccurred {});
    }
    if msg.start_block > msg.end_block {
        return Err(ContractError::StartBlockAfterEndBlock {});
    }
    if msg.start_block % msg.payment_block_delta != 0 {
        return Err(ContractError::StartBlockNotDivisibleByPaymentDelta {});
    }
    if msg.end_block % msg.payment_block_delta != 0 {
        return Err(ContractError::EndBlockNotDivisibleByPaymentDelta {});
    }
    let duration = Uint128::from(((msg.end_block - msg.start_block) / msg.payment_block_delta) + 1);
    let calculated_total = duration
        .checked_mul(msg.payment_per_block)
        .map_err(cosmwasm_std::StdError::overflow)?
        .checked_mul(Uint128::from(msg.payment_block_delta))
        .map_err(cosmwasm_std::StdError::overflow)?;
    if calculated_total != msg.total_amount {
        return Err(ContractError::InvalidTotalAmount {});
    }
    Ok(())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Claim {} => try_claim(deps, _env.clone(), info, _env.block.height),
        ExecuteMsg::Receive(msg) => try_receive(deps, _env, info, msg),
        ExecuteMsg::ClaimUpToBlock { block } => try_claim(deps, _env, info, block),
    }
}

pub fn try_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let msg: ReceiveMsg = from_binary(&wrapper.msg)?;
    match msg {
        ReceiveMsg::Fund {} => try_fund(deps, env, info, wrapper.amount),
    }
}

pub fn try_fund(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if config.funded {
        return Err(ContractError::AlreadyFunded {});
    }
    if config.denom != Denom::Cw20(info.sender) {
        return Err(ContractError::IncorrectDenom {});
    }
    if amount != config.total_amount {
        return Err(ContractError::IncorrectFundingAmount {
            received: amount,
            expected: config.total_amount,
        });
    }
    config.funded = true;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "fund")
        .add_attribute("amount", amount))
}

pub fn try_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    up_to_block: u64,
) -> Result<Response, ContractError> {
    if up_to_block > env.block.height {
        return Err(ContractError::InvalidFutureBlock {});
    }
    let config = CONFIG.load(deps.storage)?;
    if !config.funded {
        return Err(ContractError::NotFunded {});
    };
    if env.block.height < config.start_block {
        return Err(RewardsNotStarted {
            current_block: env.block.height,
            start_block: config.start_block,
        });
    }
    let last_claimed = LAST_CLAIMED
        .may_load(deps.storage, info.sender.clone())?
        .unwrap_or(LastClaim {
            block_height: config.start_block,
            time: Timestamp::default(),
        });
    let (amount_owed, new_claim_height) = get_amount_owed(
        deps.as_ref(),
        &info.sender,
        &config,
        last_claimed.block_height,
        up_to_block,
    )?;

    if amount_owed == Uint128::zero() {
        return Err(ContractError::ZeroClaimable {});
    }

    let new_claim = LastClaim {
        block_height: new_claim_height,
        time: env.block.time,
    };
    LAST_CLAIMED.save(deps.storage, info.sender.clone(), &new_claim)?;

    let payment_msg = get_payment_msg(&env, &config, &info, amount_owed)?;

    Ok(Response::new()
        .add_message(payment_msg)
        .add_attribute("action", "claim")
        .add_attribute("amount", amount_owed))
}

fn get_amount_owed(
    deps: Deps,
    address: &Addr,
    config: &Config,
    last_claimed: u64,
    up_to_block: u64,
) -> StdResult<(Uint128, u64)> {
    let delta = config.payment_block_delta;
    let mut current_block = last_claimed;
    let mut amount = Uint128::zero();
    while current_block <= min(up_to_block, config.end_block) {
        let stake_info = get_stake_info_at_height(deps.querier, config, address, current_block)?;
        if stake_info.balance > Uint128::zero() {
            amount += ((Uint128::from(delta) * config.payment_per_block) * stake_info.balance)
                .checked_div(stake_info.total)?
        }
        current_block += delta;
    }
    Ok((amount, current_block))
}

fn get_payment_msg(
    _env: &Env,
    config: &Config,
    info: &MessageInfo,
    amount: Uint128,
) -> StdResult<CosmosMsg> {
    Ok(match config.clone().denom {
        Denom::Native(denom) => BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![Coin { amount, denom }],
        }
        .into(),
        Denom::Cw20(address) => {
            let transfer = Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount,
            };
            WasmMsg::Execute {
                contract_addr: address.to_string(),
                msg: to_binary(&transfer)?,
                funds: vec![],
            }
            .into()
        }
    })
}

struct StakeInfo {
    pub balance: Uint128,
    pub total: Uint128,
}
fn get_stake_info_at_height(
    deps: QuerierWrapper,
    config: &Config,
    address: &Addr,
    height: u64,
) -> StdResult<StakeInfo> {
    let balance_query = stake_cw20::msg::QueryMsg::StakedBalanceAtHeight {
        address: address.to_string(),
        height: Some(height),
    };
    let balance_response: stake_cw20::msg::StakedBalanceAtHeightResponse =
        deps.query_wasm_smart(config.staking_contract.to_string(), &balance_query)?;
    let total_query = stake_cw20::msg::QueryMsg::TotalStakedAtHeight {
        height: Some(height),
    };
    let total_response: stake_cw20::msg::TotalStakedAtHeightResponse =
        deps.query_wasm_smart(config.staking_contract.to_string(), &total_query)?;
    Ok(StakeInfo {
        balance: balance_response.balance,
        total: total_response.total,
    })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Info {} => to_binary(&query_info(deps)?),
        QueryMsg::ClaimableRewards { address } => {
            to_binary(&query_claimable_rewards(deps, env, address)?)
        }
    }
}

pub fn query_info(deps: Deps) -> StdResult<InfoResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(InfoResponse {
        start_block: config.start_block,
        end_block: config.end_block,
        payment_per_block: config.payment_per_block,
        total_amount: config.total_amount,
        denom: config.denom,
        staking_contract: config.staking_contract.to_string(),
        payment_block_delta: config.payment_block_delta,
    })
}

pub fn query_claimable_rewards(
    deps: Deps,
    env: Env,
    address: Addr,
) -> StdResult<ClaimableRewardsResponse> {
    let config = CONFIG.load(deps.storage)?;
    let last_claimed = LAST_CLAIMED
        .may_load(deps.storage, address.clone())?
        .unwrap_or(LastClaim {
            block_height: config.start_block,
            time: Timestamp::default(),
        });
    let (amount, _) = get_amount_owed(
        deps,
        &address,
        &config,
        last_claimed.block_height,
        env.block.height,
    )?;
    Ok(ClaimableRewardsResponse { amount })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::BorrowMut;

    use crate::msg::ExecuteMsg::{Claim, ClaimUpToBlock};
    use crate::msg::QueryMsg::{ClaimableRewards, Info};
    use cosmwasm_std::Empty;
    use cw20::{Cw20Coin, Cw20Contract};
    use cw_multi_test::{next_block, App, Contract, ContractWrapper, Executor};

    const OWNER: &str = "owner0001";
    const ADDR1: &str = "addr0001";
    const ADDR2: &str = "addr0002";
    const ADDR3: &str = "addr0003";
    const ADDR4: &str = "addr0004";
    const INITIAL_BALANCE: u32 = 1000;

    pub fn contract_rewards() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        Box::new(contract)
    }

    pub fn contract_cw20() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw20_base::contract::execute,
            cw20_base::contract::instantiate,
            cw20_base::contract::query,
        );
        Box::new(contract)
    }

    pub fn contract_stake_cw20() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            stake_cw20::contract::execute,
            stake_cw20::contract::instantiate,
            stake_cw20::contract::query,
        );
        Box::new(contract)
    }

    fn mock_app() -> App {
        App::default()
    }

    // uploads code and returns address of cw20 contract
    fn instantiate_cw20(app: &mut App) -> Addr {
        let cw20_id = app.store_code(contract_cw20());
        let msg = cw20_base::msg::InstantiateMsg {
            name: "test".to_string(),
            symbol: "STAKE".to_string(),
            decimals: 0,
            initial_balances: vec![
                Cw20Coin {
                    address: Addr::unchecked(ADDR1).to_string(),
                    amount: Uint128::from(INITIAL_BALANCE),
                },
                Cw20Coin {
                    address: Addr::unchecked(ADDR2).to_string(),
                    amount: Uint128::from(INITIAL_BALANCE),
                },
                Cw20Coin {
                    address: Addr::unchecked(ADDR3).to_string(),
                    amount: Uint128::from(INITIAL_BALANCE),
                },
                Cw20Coin {
                    address: Addr::unchecked(ADDR4).to_string(),
                    amount: Uint128::from(INITIAL_BALANCE),
                },
            ],
            mint: None,
            marketing: None,
        };
        let token_contract = app
            .instantiate_contract(cw20_id, Addr::unchecked(OWNER), &msg, &[], "cw20", None)
            .unwrap();

        let stake_cw20_id = app.store_code(contract_stake_cw20());
        let msg = stake_cw20::msg::InstantiateMsg {
            admin: None,
            token_address: token_contract.clone(),
            unstaking_duration: None,
        };

        let stake_contract = app
            .instantiate_contract(
                stake_cw20_id,
                Addr::unchecked(OWNER),
                &msg,
                &[],
                "cw20",
                None,
            )
            .unwrap();

        app.execute_contract(
            Addr::unchecked(ADDR1),
            token_contract.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: stake_contract.to_string(),
                amount: Uint128::from(INITIAL_BALANCE),
                msg: to_binary(&stake_cw20::msg::ReceiveMsg::Stake {}).unwrap(),
            },
            &[],
        )
        .unwrap();
        app.execute_contract(
            Addr::unchecked(ADDR2),
            token_contract.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: stake_contract.to_string(),
                amount: Uint128::from(INITIAL_BALANCE),
                msg: to_binary(&stake_cw20::msg::ReceiveMsg::Stake {}).unwrap(),
            },
            &[],
        )
        .unwrap();
        app.execute_contract(
            Addr::unchecked(ADDR3),
            token_contract.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: stake_contract.to_string(),
                amount: Uint128::from(INITIAL_BALANCE),
                msg: to_binary(&stake_cw20::msg::ReceiveMsg::Stake {}).unwrap(),
            },
            &[],
        )
        .unwrap();
        app.execute_contract(
            Addr::unchecked(ADDR4),
            token_contract,
            &cw20::Cw20ExecuteMsg::Send {
                contract: stake_contract.to_string(),
                amount: Uint128::from(INITIAL_BALANCE),
                msg: to_binary(&stake_cw20::msg::ReceiveMsg::Stake {}).unwrap(),
            },
            &[],
        )
        .unwrap();

        stake_contract
    }

    fn instantiate_rewards(
        app: &mut App,
        msg: InstantiateMsg,
        funds: &[Coin],
    ) -> anyhow::Result<Addr> {
        let contract_id = app.store_code(contract_rewards());
        app.instantiate_contract(
            contract_id,
            Addr::unchecked(OWNER),
            &msg,
            funds,
            "rewards",
            None,
        )
    }

    #[test]
    fn proper_initialization() {
        let mut app = mock_app();
        app.borrow_mut().update_block(|b| b.height = 0);
        let stakeable_token = instantiate_cw20(&mut app);
        let instantiate_msg = InstantiateMsg {
            start_block: 1000,
            end_block: 4000,
            payment_per_block: Uint128::new(4),
            total_amount: Uint128::new(16000),
            denom: Denom::Native("utest".to_string()),
            distribution_token: stakeable_token.to_string(),
            payment_block_delta: 1000,
        };
        let _reward = instantiate_rewards(&mut app, instantiate_msg, &[]).unwrap();
    }

    #[test]
    fn basic_native_distribution() {
        let mut app = mock_app();
        app.borrow_mut().update_block(|block| block.height = 0);
        let denom = "utest".to_string();
        let owner = Addr::unchecked(OWNER);

        let init_funds = vec![Coin {
            denom: denom.clone(),
            amount: Uint128::new(16000),
        }];
        app.borrow_mut().init_modules(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &owner, init_funds.clone())
                .unwrap()
        });

        println!(
            "native balance: {}",
            app.wrap()
                .query_balance(owner, denom.clone())
                .unwrap()
                .amount
        );
        let stakeable_token = instantiate_cw20(&mut app);
        let instantiate_msg = InstantiateMsg {
            start_block: 1000,
            end_block: 4000,
            payment_per_block: Uint128::new(4),
            total_amount: Uint128::new(16000),
            denom: Denom::Native(denom.clone()),
            distribution_token: stakeable_token.to_string(),
            payment_block_delta: 1000,
        };
        app.borrow_mut().update_block(next_block);

        let reward =
            instantiate_rewards(&mut app, instantiate_msg, &init_funds).unwrap();

        let res: stake_cw20::msg::StakedBalanceAtHeightResponse = app
            .wrap()
            .query_wasm_smart(
                stakeable_token,
                &stake_cw20::msg::QueryMsg::StakedBalanceAtHeight {
                    address: ADDR1.to_string(),
                    height: None,
                },
            )
            .unwrap();
        assert_eq!(res.balance, Uint128::new(1000));

        // No claim before start block
        app.borrow_mut().update_block(|block| block.height = 500);
        let err: ContractError = app
            .execute_contract(Addr::unchecked(ADDR1), reward.clone(), &Claim {}, &[])
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(
            err,
            RewardsNotStarted {
                current_block: 500,
                start_block: 1000
            }
        );

        let native_balance = app
            .wrap()
            .query_balance(Addr::unchecked(ADDR1), denom.clone())
            .unwrap()
            .amount;
        assert_eq!(native_balance, Uint128::zero());

        // Fist claim
        app.borrow_mut().update_block(|block| block.height = 1001);
        // Test query works
        let res: ClaimableRewardsResponse = app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(
                reward.clone(),
                &ClaimableRewards {
                    address: Addr::unchecked(ADDR1),
                },
            )
            .unwrap();
        assert_eq!(res.amount, Uint128::new(1000));
        let _res = app
            .execute_contract(Addr::unchecked(ADDR1), reward.clone(), &Claim {}, &[])
            .unwrap();
        let native_balance = app
            .wrap()
            .query_balance(Addr::unchecked(ADDR1), denom.clone())
            .unwrap()
            .amount;
        assert_eq!(native_balance, Uint128::new(1000));

        // Can't claim twice
        let res: ClaimableRewardsResponse = app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(
                reward.clone(),
                &ClaimableRewards {
                    address: Addr::unchecked(ADDR1),
                },
            )
            .unwrap();
        assert_eq!(res.amount, Uint128::new(0));
        app.borrow_mut().update_block(|block| block.height = 1020);
        let err = app
            .execute_contract(Addr::unchecked(ADDR1), reward.clone(), &Claim {}, &[])
            .unwrap_err();
        assert_eq!(ContractError::ZeroClaimable {}, err.downcast().unwrap());

        // Second claim
        app.borrow_mut().update_block(|block| block.height = 2011);
        let res: ClaimableRewardsResponse = app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(
                reward.clone(),
                &ClaimableRewards {
                    address: Addr::unchecked(ADDR1),
                },
            )
            .unwrap();
        assert_eq!(res.amount, Uint128::new(1000));
        let _res = app
            .execute_contract(Addr::unchecked(ADDR1), reward.clone(), &Claim {}, &[])
            .unwrap();
        let native_balance = app
            .wrap()
            .query_balance(Addr::unchecked(ADDR1), denom.clone())
            .unwrap()
            .amount;
        assert_eq!(native_balance, Uint128::new(2000));

        // Addr2 claims two epochs at once
        let native_balance = app
            .wrap()
            .query_balance(Addr::unchecked(ADDR2), denom.clone())
            .unwrap()
            .amount;
        assert_eq!(native_balance, Uint128::zero());
        let res: ClaimableRewardsResponse = app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(
                reward.clone(),
                &ClaimableRewards {
                    address: Addr::unchecked(ADDR2),
                },
            )
            .unwrap();
        assert_eq!(res.amount, Uint128::new(2000));
        let _res = app
            .execute_contract(Addr::unchecked(ADDR2), reward.clone(), &Claim {}, &[])
            .unwrap();
        let native_balance = app
            .wrap()
            .query_balance(Addr::unchecked(ADDR2), denom.clone())
            .unwrap()
            .amount;
        assert_eq!(native_balance, Uint128::new(2000));

        // Third claim
        app.borrow_mut().update_block(|block| block.height = 3001);
        let res: ClaimableRewardsResponse = app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(
                reward.clone(),
                &ClaimableRewards {
                    address: Addr::unchecked(ADDR1),
                },
            )
            .unwrap();
        assert_eq!(res.amount, Uint128::new(1000));
        let _res = app
            .execute_contract(Addr::unchecked(ADDR1), reward.clone(), &Claim {}, &[])
            .unwrap();
        let native_balance = app
            .wrap()
            .query_balance(Addr::unchecked(ADDR1), denom.clone())
            .unwrap()
            .amount;
        assert_eq!(native_balance, Uint128::new(3000));

        // 4th claim
        app.borrow_mut().update_block(|block| block.height = 4001);
        let res: ClaimableRewardsResponse = app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(
                reward.clone(),
                &ClaimableRewards {
                    address: Addr::unchecked(ADDR1),
                },
            )
            .unwrap();
        assert_eq!(res.amount, Uint128::new(1000));
        let _res = app
            .execute_contract(Addr::unchecked(ADDR1), reward.clone(), &Claim {}, &[])
            .unwrap();
        let native_balance = app
            .wrap()
            .query_balance(Addr::unchecked(ADDR1), denom.clone())
            .unwrap()
            .amount;
        assert_eq!(native_balance, Uint128::new(4000));

        // Rewards finished
        app.borrow_mut().update_block(|block| block.height = 5001);
        let res: ClaimableRewardsResponse = app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(
                reward.clone(),
                &ClaimableRewards {
                    address: Addr::unchecked(ADDR1),
                },
            )
            .unwrap();
        assert_eq!(res.amount, Uint128::new(0));
        let err = app
            .execute_contract(Addr::unchecked(ADDR1), reward.clone(), &Claim {}, &[])
            .unwrap_err();
        assert_eq!(ContractError::ZeroClaimable {}, err.downcast().unwrap());

        // Other addresses claim rewards
        let res: ClaimableRewardsResponse = app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(
                reward.clone(),
                &ClaimableRewards {
                    address: Addr::unchecked(ADDR2),
                },
            )
            .unwrap();
        assert_eq!(res.amount, Uint128::new(2000));
        let _res = app
            .execute_contract(Addr::unchecked(ADDR2), reward.clone(), &Claim {}, &[])
            .unwrap();
        let native_balance = app
            .wrap()
            .query_balance(Addr::unchecked(ADDR2), denom.clone())
            .unwrap()
            .amount;
        assert_eq!(native_balance, Uint128::new(4000));

        let res: ClaimableRewardsResponse = app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(
                reward.clone(),
                &ClaimableRewards {
                    address: Addr::unchecked(ADDR3),
                },
            )
            .unwrap();
        assert_eq!(res.amount, Uint128::new(4000));
        let _res = app
            .execute_contract(Addr::unchecked(ADDR3), reward.clone(), &Claim {}, &[])
            .unwrap();
        let native_balance = app
            .wrap()
            .query_balance(Addr::unchecked(ADDR3), denom)
            .unwrap()
            .amount;
        assert_eq!(native_balance, Uint128::new(4000));

        let err = app
            .execute_contract(Addr::unchecked(ADDR3), reward, &Claim {}, &[])
            .unwrap_err();
        assert_eq!(ContractError::ZeroClaimable {}, err.downcast().unwrap());
    }

    #[test]
    fn basic_cw20_distribution() {
        let mut app = mock_app();
        app.borrow_mut().update_block(|block| block.height = 0);
        let denom = "utest".to_string();
        let owner = Addr::unchecked(OWNER);

        let init_funds = vec![Coin {
            denom: denom.clone(),
            amount: Uint128::new(16000),
        }];
        app.borrow_mut().init_modules(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &owner, init_funds.clone())
                .unwrap()
        });

        println!(
            "native balance: {}",
            app.wrap().query_balance(owner, denom).unwrap().amount
        );
        let stakeable_token = instantiate_cw20(&mut app);

        // Instantiate reward token
        let cw20_id = app.store_code(contract_cw20());
        let msg = cw20_base::msg::InstantiateMsg {
            name: "test".to_string(),
            symbol: "STAKE".to_string(),
            decimals: 0,
            initial_balances: vec![Cw20Coin {
                address: Addr::unchecked(OWNER).to_string(),
                amount: Uint128::new(16000),
            }],
            mint: None,
            marketing: None,
        };
        let reward_contract_addr = app
            .instantiate_contract(
                cw20_id,
                Addr::unchecked(OWNER),
                &msg,
                &[],
                "cw20_reward",
                None,
            )
            .unwrap();
        let reward_contract = Cw20Contract(reward_contract_addr);
        app.borrow_mut().update_block(next_block);

        let instantiate_msg = InstantiateMsg {
            start_block: 1000,
            end_block: 4000,
            payment_per_block: Uint128::new(4),
            total_amount: Uint128::new(16000),
            denom: Denom::Cw20(reward_contract.addr()),
            distribution_token: stakeable_token.to_string(),
            payment_block_delta: 1000,
        };
        app.borrow_mut().update_block(next_block);

        let reward =
            instantiate_rewards(&mut app, instantiate_msg, &init_funds).unwrap();

        let res: stake_cw20::msg::StakedBalanceAtHeightResponse = app
            .wrap()
            .query_wasm_smart(
                stakeable_token,
                &stake_cw20::msg::QueryMsg::StakedBalanceAtHeight {
                    address: ADDR1.to_string(),
                    height: None,
                },
            )
            .unwrap();
        assert_eq!(res.balance, Uint128::new(1000));

        // Fund Contract
        let cw20_send = Cw20ExecuteMsg::Send {
            contract: reward.to_string(),
            amount: Uint128::new(16000),
            msg: to_binary(&ReceiveMsg::Fund {}).unwrap(),
        };
        let _res = app
            .execute_contract(
                Addr::unchecked(OWNER),
                reward_contract.addr(),
                &cw20_send,
                &[],
            )
            .unwrap();

        // No claim before start block
        app.borrow_mut().update_block(|block| block.height = 500);
        let res: ClaimableRewardsResponse = app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(
                reward.clone(),
                &ClaimableRewards {
                    address: Addr::unchecked(ADDR1),
                },
            )
            .unwrap();
        assert_eq!(res.amount, Uint128::new(0));
        let err: ContractError = app
            .execute_contract(Addr::unchecked(ADDR1), reward.clone(), &Claim {}, &[])
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(
            err,
            RewardsNotStarted {
                current_block: 500,
                start_block: 1000
            }
        );

        let cw20_balance = reward_contract
            .balance(&app, Addr::unchecked(ADDR1))
            .unwrap();
        assert_eq!(cw20_balance, Uint128::zero());

        // Fist claim
        app.borrow_mut().update_block(|block| block.height = 1001);
        let res: ClaimableRewardsResponse = app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(
                reward.clone(),
                &ClaimableRewards {
                    address: Addr::unchecked(ADDR1),
                },
            )
            .unwrap();
        assert_eq!(res.amount, Uint128::new(1000));
        let _res = app
            .execute_contract(Addr::unchecked(ADDR1), reward.clone(), &Claim {}, &[])
            .unwrap();
        let cw20_balance = reward_contract
            .balance(&app, Addr::unchecked(ADDR1))
            .unwrap();
        assert_eq!(cw20_balance, Uint128::new(1000));

        // Can't claim twice
        app.borrow_mut().update_block(|block| block.height = 1020);
        let res: ClaimableRewardsResponse = app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(
                reward.clone(),
                &ClaimableRewards {
                    address: Addr::unchecked(ADDR1),
                },
            )
            .unwrap();
        assert_eq!(res.amount, Uint128::new(0));
        let err = app
            .execute_contract(Addr::unchecked(ADDR1), reward.clone(), &Claim {}, &[])
            .unwrap_err();
        assert_eq!(ContractError::ZeroClaimable {}, err.downcast().unwrap());

        // Second claim
        app.borrow_mut().update_block(|block| block.height = 2011);
        let res: ClaimableRewardsResponse = app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(
                reward.clone(),
                &ClaimableRewards {
                    address: Addr::unchecked(ADDR1),
                },
            )
            .unwrap();
        assert_eq!(res.amount, Uint128::new(1000));
        let _res = app
            .execute_contract(Addr::unchecked(ADDR1), reward.clone(), &Claim {}, &[])
            .unwrap();
        let cw20_balance = reward_contract
            .balance(&app, Addr::unchecked(ADDR1))
            .unwrap();
        assert_eq!(cw20_balance, Uint128::new(2000));

        // Addr2 claims two epochs at once
        let res: ClaimableRewardsResponse = app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(
                reward.clone(),
                &ClaimableRewards {
                    address: Addr::unchecked(ADDR2),
                },
            )
            .unwrap();
        assert_eq!(res.amount, Uint128::new(2000));
        let cw20_balance = reward_contract
            .balance(&app, Addr::unchecked(ADDR2))
            .unwrap();
        assert_eq!(cw20_balance, Uint128::zero());
        let _res = app
            .execute_contract(Addr::unchecked(ADDR2), reward.clone(), &Claim {}, &[])
            .unwrap();
        let cw20_balance = reward_contract
            .balance(&app, Addr::unchecked(ADDR2))
            .unwrap();
        assert_eq!(cw20_balance, Uint128::new(2000));

        // Third claim
        app.borrow_mut().update_block(|block| block.height = 3001);
        let res: ClaimableRewardsResponse = app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(
                reward.clone(),
                &ClaimableRewards {
                    address: Addr::unchecked(ADDR1),
                },
            )
            .unwrap();
        assert_eq!(res.amount, Uint128::new(1000));
        let _res = app
            .execute_contract(Addr::unchecked(ADDR1), reward.clone(), &Claim {}, &[])
            .unwrap();
        let cw20_balance = reward_contract
            .balance(&app, Addr::unchecked(ADDR1))
            .unwrap();
        assert_eq!(cw20_balance, Uint128::new(3000));

        // 4th claim
        app.borrow_mut().update_block(|block| block.height = 4001);
        let res: ClaimableRewardsResponse = app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(
                reward.clone(),
                &ClaimableRewards {
                    address: Addr::unchecked(ADDR1),
                },
            )
            .unwrap();
        assert_eq!(res.amount, Uint128::new(1000));
        let _res = app
            .execute_contract(Addr::unchecked(ADDR1), reward.clone(), &Claim {}, &[])
            .unwrap();
        let cw20_balance = reward_contract
            .balance(&app, Addr::unchecked(ADDR1))
            .unwrap();
        assert_eq!(cw20_balance, Uint128::new(4000));

        // Rewards finished
        app.borrow_mut().update_block(|block| block.height = 5001);
        let res: ClaimableRewardsResponse = app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(
                reward.clone(),
                &ClaimableRewards {
                    address: Addr::unchecked(ADDR1),
                },
            )
            .unwrap();
        assert_eq!(res.amount, Uint128::new(0));
        let err = app
            .execute_contract(Addr::unchecked(ADDR1), reward.clone(), &Claim {}, &[])
            .unwrap_err();
        assert_eq!(ContractError::ZeroClaimable {}, err.downcast().unwrap());

        // Other addresses claim rewards
        let res: ClaimableRewardsResponse = app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(
                reward.clone(),
                &ClaimableRewards {
                    address: Addr::unchecked(ADDR2),
                },
            )
            .unwrap();
        assert_eq!(res.amount, Uint128::new(2000));
        let _res = app
            .execute_contract(Addr::unchecked(ADDR2), reward.clone(), &Claim {}, &[])
            .unwrap();
        let cw20_balance = reward_contract
            .balance(&app, Addr::unchecked(ADDR2))
            .unwrap();
        assert_eq!(cw20_balance, Uint128::new(4000));

        let res: ClaimableRewardsResponse = app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(
                reward.clone(),
                &ClaimableRewards {
                    address: Addr::unchecked(ADDR3),
                },
            )
            .unwrap();
        assert_eq!(res.amount, Uint128::new(4000));
        let _res = app
            .execute_contract(Addr::unchecked(ADDR3), reward.clone(), &Claim {}, &[])
            .unwrap();
        let cw20_balance = reward_contract
            .balance(&app, Addr::unchecked(ADDR3))
            .unwrap();
        assert_eq!(cw20_balance, Uint128::new(4000));

        let res: ClaimableRewardsResponse = app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(
                reward.clone(),
                &ClaimableRewards {
                    address: Addr::unchecked(ADDR3),
                },
            )
            .unwrap();
        assert_eq!(res.amount, Uint128::new(0));
        let err = app
            .execute_contract(Addr::unchecked(ADDR3), reward, &Claim {}, &[])
            .unwrap_err();
        assert_eq!(ContractError::ZeroClaimable {}, err.downcast().unwrap());
    }

    #[test]
    fn info_query() {
        let mut app = mock_app();
        app.borrow_mut().update_block(|block| block.height = 0);
        let denom = "utest".to_string();
        let owner = Addr::unchecked(OWNER);

        let init_funds = vec![Coin {
            denom: denom.clone(),
            amount: Uint128::new(16000),
        }];
        app.borrow_mut().init_modules(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &owner, init_funds.clone())
                .unwrap()
        });

        println!(
            "native balance: {}",
            app.wrap()
                .query_balance(owner, denom.clone())
                .unwrap()
                .amount
        );
        let stakeable_token = instantiate_cw20(&mut app);
        let instantiate_msg = InstantiateMsg {
            start_block: 1000,
            end_block: 4000,
            payment_per_block: Uint128::new(4),
            total_amount: Uint128::new(16000),
            denom: Denom::Native(denom),
            distribution_token: stakeable_token.to_string(),
            payment_block_delta: 1000,
        };
        app.borrow_mut().update_block(next_block);

        let reward =
            instantiate_rewards(&mut app, instantiate_msg.clone(), &init_funds)
                .unwrap();

        let expected_response = InfoResponse {
            start_block: instantiate_msg.start_block,
            end_block: instantiate_msg.end_block,
            payment_per_block: instantiate_msg.payment_per_block,
            total_amount: instantiate_msg.total_amount,
            denom: instantiate_msg.denom,
            staking_contract: instantiate_msg.distribution_token,
            payment_block_delta: 1000,
        };

        let res: InfoResponse = app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(reward, &Info {})
            .unwrap();
        assert_eq!(res, expected_response)
    }

    #[test]
    fn claim_up_to() {
        let mut app = mock_app();
        app.borrow_mut().update_block(|block| block.height = 0);
        let denom = "utest".to_string();
        let owner = Addr::unchecked(OWNER);

        let init_funds = vec![Coin {
            denom: denom.clone(),
            amount: Uint128::new(16000),
        }];
        app.borrow_mut().init_modules(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &owner, init_funds.clone())
                .unwrap()
        });

        println!(
            "native balance: {}",
            app.wrap()
                .query_balance(owner, denom.clone())
                .unwrap()
                .amount
        );
        let stakeable_token = instantiate_cw20(&mut app);
        let instantiate_msg = InstantiateMsg {
            start_block: 1000,
            end_block: 4000,
            payment_per_block: Uint128::new(4),
            total_amount: Uint128::new(16000),
            denom: Denom::Native(denom.clone()),
            distribution_token: stakeable_token.to_string(),
            payment_block_delta: 1000,
        };
        app.borrow_mut().update_block(next_block);

        let reward =
            instantiate_rewards(&mut app, instantiate_msg, &init_funds).unwrap();

        let res: stake_cw20::msg::StakedBalanceAtHeightResponse = app
            .wrap()
            .query_wasm_smart(
                stakeable_token,
                &stake_cw20::msg::QueryMsg::StakedBalanceAtHeight {
                    address: ADDR1.to_string(),
                    height: None,
                },
            )
            .unwrap();
        assert_eq!(res.balance, Uint128::new(1000));

        // Cannont claim up to blocks in the future
        let err: ContractError = app
            .execute_contract(
                Addr::unchecked(ADDR1),
                reward.clone(),
                &ClaimUpToBlock { block: 3001 },
                &[],
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(err, ContractError::InvalidFutureBlock {});

        // Claim Two blocks
        app.borrow_mut().update_block(|block| block.height = 3001);
        // Test query works
        let res: ClaimableRewardsResponse = app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(
                reward.clone(),
                &ClaimableRewards {
                    address: Addr::unchecked(ADDR1),
                },
            )
            .unwrap();
        assert_eq!(res.amount, Uint128::new(3000));
        let _res = app
            .execute_contract(
                Addr::unchecked(ADDR1),
                reward.clone(),
                &ClaimUpToBlock { block: 2001 },
                &[],
            )
            .unwrap();
        let native_balance = app
            .wrap()
            .query_balance(Addr::unchecked(ADDR1), denom.clone())
            .unwrap()
            .amount;
        assert_eq!(native_balance, Uint128::new(2000));

        let _res = app
            .execute_contract(
                Addr::unchecked(ADDR1),
                reward,
                &ClaimUpToBlock { block: 3001 },
                &[],
            )
            .unwrap();
        let native_balance = app
            .wrap()
            .query_balance(Addr::unchecked(ADDR1), denom)
            .unwrap()
            .amount;
        assert_eq!(native_balance, Uint128::new(3000));
    }

    #[test]
    fn validate_instantiate_msg() {
        let mut app = mock_app();
        app.borrow_mut().update_block(|b| b.height = 5000);
        let stakeable_token = instantiate_cw20(&mut app);
        let instantiate_msg = InstantiateMsg {
            start_block: 1000,
            end_block: 4000,
            payment_per_block: Uint128::new(4),
            total_amount: Uint128::new(16000),
            denom: Denom::Native("utest".to_string()),
            distribution_token: stakeable_token.to_string(),
            payment_block_delta: 1000,
        };
        let err: ContractError = instantiate_rewards(&mut app, instantiate_msg, &[])
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(err, ContractError::StartBlockAlreadyOccurred {});

        app.borrow_mut().update_block(|b| b.height = 0);
        let stakeable_token = instantiate_cw20(&mut app);
        let instantiate_msg = InstantiateMsg {
            start_block: 5000,
            end_block: 4000,
            payment_per_block: Uint128::new(4),
            total_amount: Uint128::new(16000),
            denom: Denom::Native("utest".to_string()),
            distribution_token: stakeable_token.to_string(),
            payment_block_delta: 1000,
        };
        let err: ContractError = instantiate_rewards(&mut app, instantiate_msg, &[])
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(err, ContractError::StartBlockAfterEndBlock {});

        app.borrow_mut().update_block(|b| b.height = 0);
        let stakeable_token = instantiate_cw20(&mut app);
        let instantiate_msg = InstantiateMsg {
            start_block: 1050,
            end_block: 4000,
            payment_per_block: Uint128::new(4),
            total_amount: Uint128::new(16000),
            denom: Denom::Native("utest".to_string()),
            distribution_token: stakeable_token.to_string(),
            payment_block_delta: 1000,
        };
        let err: ContractError = instantiate_rewards(&mut app, instantiate_msg, &[])
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(err, ContractError::StartBlockNotDivisibleByPaymentDelta {});

        app.borrow_mut().update_block(|b| b.height = 0);
        let stakeable_token = instantiate_cw20(&mut app);
        let instantiate_msg = InstantiateMsg {
            start_block: 1000,
            end_block: 3999,
            payment_per_block: Uint128::new(4),
            total_amount: Uint128::new(16000),
            denom: Denom::Native("utest".to_string()),
            distribution_token: stakeable_token.to_string(),
            payment_block_delta: 1000,
        };
        let err: ContractError = instantiate_rewards(&mut app, instantiate_msg, &[])
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(err, ContractError::EndBlockNotDivisibleByPaymentDelta {});

        app.borrow_mut().update_block(|b| b.height = 0);
        let stakeable_token = instantiate_cw20(&mut app);
        let instantiate_msg = InstantiateMsg {
            start_block: 1000,
            end_block: 4000,
            payment_per_block: Uint128::new(4),
            total_amount: Uint128::new(16001),
            denom: Denom::Native("utest".to_string()),
            distribution_token: stakeable_token.to_string(),
            payment_block_delta: 1000,
        };
        let err: ContractError = instantiate_rewards(&mut app, instantiate_msg, &[])
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(err, ContractError::InvalidTotalAmount {});
    }
}
