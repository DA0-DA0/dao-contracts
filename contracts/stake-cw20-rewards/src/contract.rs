#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use std::cmp::min;

use cosmwasm_std::{
    from_binary, to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError,
    StdResult, Uint128,
};

use cw20::Cw20ReceiveMsg;

use crate::msg::{
    ExecuteMsg, GetConfigResponse, GetPendingRewardsResponse, InstantiateMsg, QueryMsg, ReceiveMsg,
};
use crate::state::{Config, CONFIG, LAST_CLAIM};
use crate::ContractError;
use cw2::set_contract_version;
pub use cw20_base::enumerable::{query_all_accounts, query_all_allowances};

const CONTRACT_NAME: &str = "crates.io:stake_cw20_rewards";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<Empty>, ContractError> {
    // Validate config
    let blocks = Uint128::from(msg.end_block - msg.start_block);
    let calculated_total = msg
        .payment_per_block
        .checked_mul(blocks)
        .map_err(StdError::overflow)?;
    if calculated_total != msg.total_payment {
        return Err(ContractError::ConfigInvalid {});
    };

    let config = Config {
        token_address: msg.token_address,
        staking_contract: msg.staking_contract,
        payment_per_block: msg.payment_per_block,
        total_payment: msg.total_payment,
        start_block: msg.start_block,
        end_block: msg.end_block,
        funded: false,
    };
    CONFIG.save(deps.storage, &config)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<Empty>, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::Claim {} => execute_claim(deps, env),
    }
}

pub fn execute_receive(
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
        ReceiveMsg::Fund {} => execute_fund(deps, env, wrapper.amount),
    }
}

pub fn execute_fund(deps: DepsMut, _env: Env, amount: Uint128) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if config.funded {
        return Err(ContractError::AlreadyFunded {});
    };
    if config.total_payment != amount {
        return Err(ContractError::IncorrectFundingAmount {});
    };
    config.funded = true;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "funded")
        .add_attribute("amount", amount))
}

pub fn execute_claim(deps: DepsMut, env: Env) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let last_claim = LAST_CLAIM.load(deps.storage).unwrap_or(config.start_block);

    if env.block.height < config.start_block {
        return Err(ContractError::RewardsNotStarted {});
    };
    if last_claim >= config.end_block {
        return Err(ContractError::RewardsFinished {});
    };
    if last_claim == env.block.height {
        return Err(ContractError::RewardsAlreadyClaimed {});
    };
    if !config.funded {
        return Err(ContractError::RewardsNotFunded {});
    };

    let blocks = Uint128::from(min(&env.block.height, &config.end_block) - last_claim);
    let reward_to_disburse = blocks
        .checked_mul(config.payment_per_block)
        .map_err(StdError::overflow)?;

    let sub_msg = to_binary(&stake_cw20::msg::ReceiveMsg::Fund {})?;
    let payment_msg = cw20::Cw20ExecuteMsg::Send {
        contract: config.staking_contract.to_string(),
        amount: reward_to_disburse,
        msg: sub_msg,
    };

    let cosmos_msg = cosmwasm_std::WasmMsg::Execute {
        contract_addr: config.token_address.to_string(),
        msg: to_binary(&payment_msg)?,
        funds: vec![],
    };

    LAST_CLAIM.save(deps.storage, &min(env.block.height, config.end_block))?;

    Ok(Response::new()
        .add_message(cosmos_msg)
        .add_attribute("action", "claim")
        .add_attribute("amount", reward_to_disburse))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
        QueryMsg::GetPendingRewards {} => to_binary(&query_rewards(deps, env)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<GetConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(GetConfigResponse {
        token_address: config.token_address,
        staking_contract: config.staking_contract,
        payment_per_block: config.payment_per_block,
        total_payment: config.total_payment,
        start_block: config.start_block,
        end_block: config.end_block,
        funded: config.funded,
    })
}

pub fn query_rewards(deps: Deps, env: Env) -> StdResult<GetPendingRewardsResponse> {
    let config = CONFIG.load(deps.storage)?;
    let last_claim = LAST_CLAIM.load(deps.storage).unwrap_or(config.start_block);
    let blocks = Uint128::from(min(&env.block.height, &config.end_block) - last_claim);
    let pending_rewards = if config.funded {
        blocks
            .checked_mul(config.payment_per_block)
            .map_err(StdError::overflow)?
    } else {
        Uint128::zero()
    };

    Ok(GetPendingRewardsResponse {
        amount: pending_rewards,
    })
}

#[cfg(test)]
mod tests {
    use crate::msg::{
        ExecuteMsg, GetConfigResponse, GetPendingRewardsResponse, InstantiateMsg, QueryMsg,
        ReceiveMsg,
    };
    use anyhow::Result as AnyResult;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{to_binary, Addr, Decimal, Empty, Uint128};
    use cw20::{Cw20Coin, Cw20ReceiveMsg};
    use cw_multi_test::{next_block, App, AppResponse, Contract, ContractWrapper, Executor};
    use cw_utils::Duration;

    const ADDR1: &str = "addr0001";
    const ADDR2: &str = "addr0002";

    const TOTAL_PAYMENT: u128 = 2000;
    const TOTAL_REWARDS_DURATION: u64 = 10;

    pub fn contract_staking_rewards() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        Box::new(contract)
    }

    pub fn contract_staking() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            stake_cw20::contract::execute,
            stake_cw20::contract::instantiate,
            stake_cw20::contract::query,
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

    fn mock_app() -> App {
        App::default()
    }

    fn instantiate_cw20(app: &mut App, initial_balances: Vec<Cw20Coin>) -> Addr {
        let cw20_id = app.store_code(contract_cw20());
        let msg = cw20_base::msg::InstantiateMsg {
            name: String::from("Test"),
            symbol: String::from("TEST"),
            decimals: 6,
            initial_balances,
            mint: None,
            marketing: None,
        };

        app.instantiate_contract(cw20_id, Addr::unchecked(ADDR1), &msg, &[], "cw20", None)
            .unwrap()
    }

    fn instantiate_staking_rewards(
        app: &mut App,
        cw20: Addr,
        stake: Addr,
        total_payment: Uint128,
        start_block: u64,
        end_block: u64,
    ) -> Addr {
        let staking_rewards_code_id = app.store_code(contract_staking_rewards());
        let duration = end_block - start_block;
        let payment_per_block = total_payment
            .checked_div(Uint128::from(duration))
            .unwrap_or_default();
        let msg = InstantiateMsg {
            token_address: cw20,
            staking_contract: stake,
            payment_per_block,
            total_payment,
            start_block,
            end_block,
        };
        app.instantiate_contract(
            staking_rewards_code_id,
            Addr::unchecked(ADDR1),
            &msg,
            &[],
            "staking_rewards",
            None,
        )
        .unwrap()
    }

    fn instantiate_staking(
        app: &mut App,
        cw20: Addr,
        unstaking_duration: Option<Duration>,
    ) -> Addr {
        let staking_code_id = app.store_code(contract_staking());
        let msg = stake_cw20::msg::InstantiateMsg {
            admin: Some(Addr::unchecked("owner")),
            token_address: cw20,
            unstaking_duration,
        };
        app.instantiate_contract(
            staking_code_id,
            Addr::unchecked(ADDR1),
            &msg,
            &[],
            "staking",
            None,
        )
        .unwrap()
    }

    fn setup_test_case(
        app: &mut App,
        initial_balances: Vec<Cw20Coin>,
        total_payment: Uint128,
        start_block: u64,
        end_block: u64,
    ) -> (Addr, Addr, Addr) {
        // Instantiate cw20 contract and staking contract for that cw20
        let cw20_addr = instantiate_cw20(app, initial_balances);
        let staking_addr = instantiate_staking(app, cw20_addr.clone(), None);
        // app.update_block(next_block);
        // Instantiate staking rewards contract
        let staking_rewards_addr = instantiate_staking_rewards(
            app,
            cw20_addr.clone(),
            staking_addr.clone(),
            total_payment,
            start_block,
            end_block,
        );
        // app.update_block(next_block);
        (staking_rewards_addr, staking_addr, cw20_addr)
    }

    fn query_config<T: Into<String>>(app: &App, contract_addr: T) -> GetConfigResponse {
        let msg = QueryMsg::GetConfig {};
        app.wrap().query_wasm_smart(contract_addr, &msg).unwrap()
    }

    fn query_pending_rewards<T: Into<String>>(
        app: &App,
        contract_addr: T,
    ) -> GetPendingRewardsResponse {
        let msg = QueryMsg::GetPendingRewards {};
        app.wrap().query_wasm_smart(contract_addr, &msg).unwrap()
    }

    fn invalid_receive(
        app: &mut App,
        staking_rewards_addr: &Addr,
        amount: Uint128,
    ) -> AnyResult<AppResponse> {
        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: ADDR1.to_string(),
            amount,
            msg: to_binary(&ReceiveMsg::Fund {})?,
        });
        // Not a token address
        app.execute_contract(
            Addr::unchecked(ADDR1),
            staking_rewards_addr.clone(),
            &msg,
            &[],
        )
    }

    fn fund_rewards(
        app: &mut App,
        staking_rewards_addr: &Addr,
        cw20_addr: &Addr,
        amount: Uint128,
    ) -> AnyResult<AppResponse> {
        let msg = cw20::Cw20ExecuteMsg::Send {
            contract: staking_rewards_addr.to_string(),
            amount,
            msg: to_binary(&ReceiveMsg::Fund {})?,
        };
        app.execute_contract(Addr::unchecked(ADDR1), cw20_addr.clone(), &msg, &[])
    }

    fn claim_rewards(app: &mut App, staking_rewards_addr: &Addr) -> AnyResult<AppResponse> {
        let msg = ExecuteMsg::Claim {};
        app.execute_contract(
            Addr::unchecked(ADDR1),
            staking_rewards_addr.clone(),
            &msg,
            &[],
        )
    }

    fn stake_tokens(
        app: &mut App,
        staking_addr: &Addr,
        cw20_addr: &Addr,
        sender: Addr,
        amount: Uint128,
    ) -> AnyResult<AppResponse> {
        let msg = cw20::Cw20ExecuteMsg::Send {
            contract: staking_addr.to_string(),
            amount,
            msg: to_binary(&stake_cw20::msg::ReceiveMsg::Stake {}).unwrap(),
        };
        app.execute_contract(sender, cw20_addr.clone(), &msg, &[])
    }

    fn query_staked_value<T: Into<String>, U: Into<String>>(
        app: &App,
        contract_addr: T,
        address: U,
    ) -> Uint128 {
        let msg = stake_cw20::msg::QueryMsg::StakedValue {
            address: address.into(),
        };
        let result: stake_cw20::msg::StakedValueResponse =
            app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
        result.value
    }

    fn get_balance<T: Into<String>, U: Into<String>>(
        app: &App,
        contract_addr: T,
        address: U,
    ) -> Uint128 {
        let msg = cw20::Cw20QueryMsg::Balance {
            address: address.into(),
        };
        let result: cw20::BalanceResponse =
            app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
        result.balance
    }

    #[test]
    fn test_query_config() {
        let mut app = mock_app();
        let total_payment = Uint128::from(TOTAL_PAYMENT);
        let start_block = app.block_info().height + 2; // We need to add 2 due to setup contract advancing the block twice
        let end_block = app.block_info().height + 2 + TOTAL_REWARDS_DURATION;
        let (rewards, stake, token) =
            setup_test_case(&mut app, vec![], total_payment, start_block, end_block);
        let resp = query_config(&app, rewards);
        assert_eq!(
            GetConfigResponse {
                token_address: token,
                staking_contract: stake,
                payment_per_block: total_payment
                    .checked_div(Uint128::from(end_block - start_block))
                    .unwrap_or_default(),
                total_payment,
                start_block,
                end_block,
                funded: false
            },
            resp
        );
    }

    #[test]
    fn test_fund() {
        let _env = mock_env();
        let _deps = mock_dependencies();
        let mut app = mock_app();
        let total_payment = Uint128::from(TOTAL_PAYMENT);
        let start_block = app.block_info().height;
        let end_block = app.block_info().height + TOTAL_REWARDS_DURATION;
        let initial_balances = vec![Cw20Coin {
            address: ADDR1.to_string(),
            amount: Uint128::from(4000u64),
        }];
        let (rewards, _stake, token) = setup_test_case(
            &mut app,
            initial_balances,
            total_payment,
            start_block,
            end_block,
        );

        // Non token address sends receive msg
        let _err = invalid_receive(&mut app, &rewards, total_payment).unwrap_err();

        // Partially fund
        let _err = fund_rewards(
            &mut app,
            &rewards,
            &token,
            total_payment.checked_div(Uint128::from(2u64)).unwrap(),
        )
        .unwrap_err();

        // Fund fully
        let _res = fund_rewards(&mut app, &rewards, &token, total_payment).unwrap();
        let rewards_bal = get_balance(&app, &token, &rewards);
        let addr1_bal = get_balance(&app, &token, Addr::unchecked(ADDR1));
        assert_eq!(rewards_bal, Uint128::from(2000u64));
        assert_eq!(addr1_bal, Uint128::from(2000u64));

        let config = query_config(&app, rewards.clone());
        assert!(config.funded);

        // Fund again, now fails
        let _err = fund_rewards(&mut app, &rewards, &token, total_payment).unwrap_err();
    }

    #[test]
    fn test_query_pending_rewards() {
        let mut app = mock_app();
        let total_payment = Uint128::from(TOTAL_PAYMENT);
        let start_block = app.block_info().height;
        let end_block = app.block_info().height + TOTAL_REWARDS_DURATION;
        let rewards_per_block = total_payment
            .checked_div(Uint128::from(TOTAL_REWARDS_DURATION))
            .unwrap();
        let initial_balances = vec![Cw20Coin {
            address: ADDR1.to_string(),
            amount: Uint128::from(4000u64),
        }];
        let (rewards, _stake, token) = setup_test_case(
            &mut app,
            initial_balances,
            total_payment,
            start_block,
            end_block,
        );

        // While not funded this will return 0
        let resp = query_pending_rewards(&app, rewards.clone());
        assert_eq!(resp.amount, Uint128::zero());

        // Fund fully
        let _res = fund_rewards(&mut app, &rewards, &token, total_payment).unwrap();

        // No blocks since start so still 0
        let resp = query_pending_rewards(&app, rewards.clone());
        assert_eq!(resp.amount, Uint128::zero());

        let mut total_blocks_since_start: u64 = 0;
        // Test whole duration at each block, with no claims
        while total_blocks_since_start < TOTAL_REWARDS_DURATION {
            let expected_rewards = rewards_per_block
                .checked_mul(Uint128::from(total_blocks_since_start))
                .unwrap();

            let resp = query_pending_rewards(&app, rewards.clone());
            assert_eq!(resp.amount, expected_rewards);

            app.update_block(next_block);
            total_blocks_since_start += 1;
        }
    }

    #[test]
    fn test_claim() {
        let mut app = mock_app();
        let total_payment = Uint128::from(TOTAL_PAYMENT);
        let start_block = app.block_info().height + 1; // We add 1 to make sure rewards do not start instantly so we can check for the error : )
        let end_block = app.block_info().height + 1 + TOTAL_REWARDS_DURATION;
        let rewards_per_block = total_payment
            .checked_div(Uint128::from(TOTAL_REWARDS_DURATION))
            .unwrap();
        let (rewards, stake, token) = setup_test_case(
            &mut app,
            vec![
                Cw20Coin {
                    address: ADDR1.to_string(),
                    amount: Uint128::from(5000u64),
                },
                Cw20Coin {
                    address: ADDR2.to_string(),
                    amount: Uint128::from(1000u64),
                },
            ],
            total_payment,
            start_block,
            end_block,
        );

        // CHECKING ALL ERRORS PRIOR TO FUNDING OR THE REWARDS STARTING
        // Should error as rewards not started
        let _err = claim_rewards(&mut app, &rewards).unwrap_err();
        // Now rewards have started
        app.update_block(next_block);

        // Should error as rewards already claimed AKA we are at start block
        // This is caused due to the unwrap_or(config.start_block) in claim
        // when checking last claim
        let _err = claim_rewards(&mut app, &rewards).unwrap_err();
        app.update_block(next_block);

        // Should error as contract is not funded
        let _err = claim_rewards(&mut app, &rewards).unwrap_err();

        // Fund fully
        let _res = fund_rewards(&mut app, &rewards, &token, total_payment).unwrap();

        // ADDR1 stake 2000 tokens
        // ADDR2 stake 1000 tokens
        let addr1_stake = Uint128::from(2000u64);
        let addr2_stake = Uint128::from(1000u64);
        let initial_stake = addr1_stake + addr2_stake;
        stake_tokens(
            &mut app,
            &stake,
            &token,
            Addr::unchecked(ADDR1),
            addr1_stake,
        )
        .unwrap();
        stake_tokens(
            &mut app,
            &stake,
            &token,
            Addr::unchecked(ADDR2),
            addr2_stake,
        )
        .unwrap();

        // We are now 1 block into rewards, so we should expect 200 rewards
        // Querying for sanity check
        let mut total_blocks: u64 = 1;
        let expected_rewards = rewards_per_block
            .checked_mul(Uint128::from(total_blocks))
            .unwrap();

        let resp = query_pending_rewards(&app, rewards.clone());
        assert_eq!(resp.amount, expected_rewards);

        // Lets compound every block until the end
        let mut staking_running_bal = initial_stake;
        while total_blocks <= TOTAL_REWARDS_DURATION {
            let _res = claim_rewards(&mut app, &rewards).unwrap();
            // Update our running balance
            staking_running_bal += rewards_per_block;

            // Check overall staked balance
            let stake_bal = get_balance(&app, &token, &stake);
            assert_eq!(stake_bal, staking_running_bal);

            // Check pending rewards is now zero
            let resp = query_pending_rewards(&app, rewards.clone());
            assert_eq!(resp.amount, Uint128::zero());

            total_blocks += 1;
            app.update_block(next_block);
        }

        // Now let's check if the users rewards all line up
        let addr1_rewards = total_payment * Decimal::from_ratio(addr1_stake, initial_stake);
        let addr2_rewards = total_payment * Decimal::from_ratio(addr2_stake, initial_stake);

        let addr1_staked_val = query_staked_value(&app, &stake, Addr::unchecked(ADDR1));
        let addr2_staked_val = query_staked_value(&app, &stake, Addr::unchecked(ADDR2));
        assert_eq!(addr1_staked_val, addr1_stake + addr1_rewards);
        assert_eq!(addr2_staked_val, addr2_stake + addr2_rewards);

        // Should error as rewards are now over
        let _err = claim_rewards(&mut app, &rewards).unwrap_err();
    }
}
