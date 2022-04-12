use cosmwasm_std::{to_binary, Addr, Empty, Uint128};
use cw2::ContractVersion;
use cw20::{BalanceResponse, Cw20Coin, MinterResponse, TokenInfoResponse};
use cw20_staked_balance_voting::msg::TokenInfo;
use cw_core_interface::voting::{InfoResponse, VotingPowerAtHeightResponse};
use cw_multi_test::{next_block, App, Contract, ContractWrapper, Executor};

use crate::msg::{InstantiateMsg, QueryMsg};

const DAO_ADDR: &str = "dao";
const CREATOR_ADDR: &str = "creator";

fn cw20_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

fn staking_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        stake_cw20::contract::execute,
        stake_cw20::contract::instantiate,
        stake_cw20::contract::query,
    );
    Box::new(contract)
}

fn shuffle_voting_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_reply(crate::contract::reply);
    Box::new(contract)
}

fn instantiate_voting(app: &mut App, voting_id: u64, msg: InstantiateMsg) -> Addr {
    app.instantiate_contract(
        voting_id,
        Addr::unchecked(DAO_ADDR),
        &msg,
        &[],
        "voting module",
        None,
    )
    .unwrap()
}

fn stake_tokens(app: &mut App, staking_addr: Addr, cw20_addr: Addr, sender: &str, amount: u128) {
    let msg = cw20::Cw20ExecuteMsg::Send {
        contract: staking_addr.to_string(),
        amount: Uint128::new(amount),
        msg: to_binary(&stake_cw20::msg::ReceiveMsg::Stake {}).unwrap(),
    };
    app.execute_contract(Addr::unchecked(sender), cw20_addr, &msg, &[])
        .unwrap();
}
