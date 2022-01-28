#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128,
};
use cw20::Cw20ReceiveMsg;

use crate::msg::{
    DelegationResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg,
    VotingPowerAtHeightResponse,
};
use crate::state::{DELEGATIONS, VOTING_POWER};
use stake_cw20::state::{CONFIG, STAKED_BALANCES};
use stake_cw20::ContractError;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    stake_cw20::contract::instantiate(deps, _env, _info, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::Unstake { amount } => execute_unstake(deps, env, info, amount),
        ExecuteMsg::Claim {} => stake_cw20::contract::execute_claim(deps, env, info),
        ExecuteMsg::UpdateConfig { admin, duration } => {
            stake_cw20::contract::execute_update_config(info, deps, admin, duration)
        }
        ExecuteMsg::DelegateVotes { recipient } => {
            execute_delegate_votes(deps, env, info, recipient)
        }
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
    let sender = deps.api.addr_validate(&wrapper.sender)?;
    match msg {
        ReceiveMsg::Stake {} => execute_stake(deps, env, &sender, wrapper.amount),
    }
}

pub fn execute_stake(
    deps: DepsMut,
    env: Env,
    sender: &Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let delegation = DELEGATIONS
        .may_load(deps.storage, sender)?
        .unwrap_or_else(|| sender.clone());
    VOTING_POWER.update(
        deps.storage,
        &delegation,
        env.block.height,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_add(amount)?)
        },
    )?;
    stake_cw20::contract::execute_stake(deps, env, sender, amount)
}

pub fn execute_unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let delegation = DELEGATIONS
        .may_load(deps.storage, &info.sender)?
        .unwrap_or_else(|| info.sender.clone());
    VOTING_POWER.update(
        deps.storage,
        &delegation,
        env.block.height,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    stake_cw20::contract::execute_unstake(deps, env, info, amount)
}

pub fn execute_delegate_votes(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
) -> Result<Response, ContractError> {
    let rcpt_addr = deps.api.addr_validate(&recipient)?;
    let amount = STAKED_BALANCES
        .may_load(deps.storage, &info.sender)?
        .unwrap_or_default();
    let old_delegation = DELEGATIONS
        .may_load(deps.storage, &info.sender)?
        .unwrap_or_else(|| info.sender.clone());
    DELEGATIONS.update(deps.storage, &info.sender, |_| -> StdResult<_> {
        Ok(rcpt_addr.clone())
    })?;
    VOTING_POWER.update(
        deps.storage,
        &old_delegation,
        env.block.height,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    VOTING_POWER.update(
        deps.storage,
        &rcpt_addr,
        env.block.height,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // Custom queries
        QueryMsg::VotingPowerAtHeight { address, height } => {
            to_binary(&query_voting_power_at_height(deps, env, address, height)?)
        }
        QueryMsg::Delegation { address } => to_binary(&query_delegation(deps, address)?),
        QueryMsg::TotalStakedAtHeight { height } => to_binary(
            &stake_cw20::contract::query_total_staked_at_height(deps, env, height)?,
        ),
        QueryMsg::StakedBalanceAtHeight { address, height } => to_binary(
            &stake_cw20::contract::query_staked_balance_at_height(deps, env, address, height)?,
        ),
        QueryMsg::GetConfig {} => to_binary(&stake_cw20::contract::query_config(deps)?),
        QueryMsg::Claims { address } => {
            to_binary(&stake_cw20::contract::query_claims(deps, address)?)
        }
    }
}

pub fn query_voting_power_at_height(
    deps: Deps,
    env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<VotingPowerAtHeightResponse> {
    let address = deps.api.addr_validate(&address)?;
    let height = height.unwrap_or(env.block.height);
    let balance = VOTING_POWER
        .may_load_at_height(deps.storage, &address, height)?
        .unwrap_or_default();
    Ok(VotingPowerAtHeightResponse { balance, height })
}

pub fn query_delegation(deps: Deps, address: String) -> StdResult<DelegationResponse> {
    let address_addr = deps.api.addr_validate(&address)?;
    let delegation = DELEGATIONS
        .may_load(deps.storage, &address_addr)?
        .unwrap_or(address_addr);
    Ok(DelegationResponse {
        delegation: delegation.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cw20::Cw20Coin;

    use crate::msg::{
        ExecuteMsg, QueryMsg, ReceiveMsg, StakedBalanceAtHeightResponse,
        TotalStakedAtHeightResponse,
    };
    use anyhow::Result as AnyResult;
    use cosmwasm_std::testing::mock_info;
    use cosmwasm_std::{to_binary, Addr, Empty, MessageInfo, Uint128};
    use cw_multi_test::{next_block, App, AppResponse, Contract, ContractWrapper, Executor};

    const ADDR1: &str = "addr0001";
    const ADDR2: &str = "addr0002";

    pub fn contract_staking_gov() -> Box<dyn Contract<Empty>> {
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

    fn instantiate_staking(app: &mut App, cw20: Addr) -> Addr {
        let staking_code_id = app.store_code(contract_staking_gov());
        let msg = crate::msg::InstantiateMsg {
            admin: Addr::unchecked("owner"),
            token_address: cw20,
            unstaking_duration: None,
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

    fn setup_test_case(app: &mut App, initial_balances: Vec<Cw20Coin>) -> (Addr, Addr) {
        // Instantiate cw20 contract
        let cw20_addr = instantiate_cw20(app, initial_balances);
        app.update_block(next_block);
        // Instantiate staking contract
        let staking_addr = instantiate_staking(app, cw20_addr.clone());
        app.update_block(next_block);
        (staking_addr, cw20_addr)
    }

    fn query_staked_balance<T: Into<String>, U: Into<String>>(
        app: &App,
        contract_addr: T,
        address: U,
    ) -> Uint128 {
        let msg = QueryMsg::StakedBalanceAtHeight {
            address: address.into(),
            height: None,
        };
        let result: StakedBalanceAtHeightResponse =
            app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
        result.balance
    }

    fn query_total_staked<T: Into<String>>(app: &App, contract_addr: T) -> Uint128 {
        let msg = QueryMsg::TotalStakedAtHeight { height: None };
        let result: TotalStakedAtHeightResponse =
            app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
        result.total
    }

    fn query_voting_power<T: Into<String>, U: Into<String>>(
        app: &App,
        contract_addr: T,
        address: U,
    ) -> Uint128 {
        let msg = QueryMsg::VotingPowerAtHeight {
            address: address.into(),
            height: None,
        };
        let result: VotingPowerAtHeightResponse =
            app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
        result.balance
    }

    fn stake_tokens(
        app: &mut App,
        staking_addr: &Addr,
        cw20_addr: &Addr,
        info: MessageInfo,
        amount: Uint128,
    ) -> AnyResult<AppResponse> {
        let msg = cw20::Cw20ExecuteMsg::Send {
            contract: staking_addr.to_string(),
            amount,
            msg: to_binary(&ReceiveMsg::Stake {}).unwrap(),
        };
        app.execute_contract(info.sender, cw20_addr.clone(), &msg, &[])
    }

    fn delegate<T: Into<String>>(
        app: &mut App,
        staking_addr: &Addr,
        info: MessageInfo,
        delegate_adder: T,
    ) -> AnyResult<AppResponse> {
        let msg = ExecuteMsg::DelegateVotes {
            recipient: delegate_adder.into(),
        };
        app.execute_contract(info.sender, staking_addr.clone(), &msg, &[])
    }

    fn unstake_tokens(
        app: &mut App,
        staking_addr: &Addr,
        info: MessageInfo,
        amount: Uint128,
    ) -> AnyResult<AppResponse> {
        let msg = ExecuteMsg::Unstake { amount };
        app.execute_contract(info.sender, staking_addr.clone(), &msg, &[])
    }

    #[test]
    fn test_contract() {
        let mut app = mock_app();
        let amount1 = Uint128::from(100u128);
        let initial_balances = vec![Cw20Coin {
            address: ADDR1.to_string(),
            amount: amount1,
        }];
        let (staking_addr, cw20_addr) = setup_test_case(&mut app, initial_balances);

        let _info = mock_info(ADDR1, &[]);

        assert_eq!(
            Uint128::zero(),
            query_voting_power(&app, &staking_addr, ADDR1)
        );

        // Stake tokens
        let info = mock_info(ADDR1, &[]);
        let amount = Uint128::new(50);
        let _res = stake_tokens(&mut app, &staking_addr, &cw20_addr, info, amount).unwrap();
        app.update_block(next_block);
        assert_eq!(amount, query_staked_balance(&app, &staking_addr, ADDR1));
        assert_eq!(amount, query_total_staked(&app, &staking_addr));
        assert_eq!(amount, query_voting_power(&app, &staking_addr, ADDR1));
        assert_eq!(
            Uint128::zero(),
            query_voting_power(&app, &staking_addr, ADDR2)
        );

        // Delegate votes
        let info = mock_info(ADDR1, &[]);
        let _res = delegate(&mut app, &staking_addr, info, ADDR2).unwrap();
        app.update_block(next_block);
        assert_eq!(
            Uint128::zero(),
            query_voting_power(&app, &staking_addr, ADDR1)
        );
        assert_eq!(amount, query_voting_power(&app, &staking_addr, ADDR2));

        // Partially unstake
        let info = mock_info(ADDR1, &[]);
        let amount2 = Uint128::new(10);
        let _res = unstake_tokens(&mut app, &staking_addr, info, amount2).unwrap();
        app.update_block(next_block);
        assert_eq!(
            Uint128::zero(),
            query_voting_power(&app, &staking_addr, ADDR1)
        );
        assert_eq!(
            Uint128::new(40),
            query_voting_power(&app, &staking_addr, ADDR2)
        );

        // Fully unstake
        let info = mock_info(ADDR1, &[]);
        let amount3 = Uint128::new(40);
        let _res = unstake_tokens(&mut app, &staking_addr, info, amount3).unwrap();
        app.update_block(next_block);
        assert_eq!(
            Uint128::zero(),
            query_voting_power(&app, &staking_addr, ADDR1)
        );
        assert_eq!(
            Uint128::zero(),
            query_voting_power(&app, &staking_addr, ADDR2)
        );
    }
}
