use crate::msg::{InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg, Counterparty, TokenInfo, ExecuteMsgFns, QueryMsgFns};

use boot_core::{boot_contract, BootEnvironment, Contract, IndexResponse, TxResponse, {BootQuery, ContractInstance}, instantiate_default_mock_env, BootUpload, BootInstantiate, ContractWrapper, BootError, CallAs};
use cosmwasm_std::{Addr, Coin, StdResult, Uint128};
use speculoos::prelude::*;

#[boot_contract(InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg)]
pub struct CwTokenSwap<Chain>;

impl<Chain: BootEnvironment> CwTokenSwap<Chain>
    where
        TxResponse<Chain>: IndexResponse,
{

    pub fn new_mock(chain: Chain) -> Self {
        Self(
            Contract::new("cw-token-swap", chain)
                .with_mock(Box::new(
                    ContractWrapper::new(
                        crate::contract::execute,
                        crate::contract::instantiate,
                        crate::contract::query,
                    )
                ))
        )
    }
}

const DAO1: &str = "dao1";
const DAO2: &str = "dao2";


const UJUNO: &'static str = "ujuno";

fn mock_init<Chain: BootEnvironment>(chain: Chain) -> Result<CwTokenSwap<Chain>, BootError> {
    let mut token_swap =  CwTokenSwap::new_mock(chain.clone());
    token_swap.upload()?;

    token_swap.instantiate(&InstantiateMsg {
        counterparty_one: Counterparty {
            address: DAO1.to_string(),
            promise: TokenInfo::Native {
                denom: UJUNO.to_string(),
                amount: Uint128::new(100),
            },
        },
        counterparty_two: Counterparty {
            address: DAO2.to_string(),
            promise: TokenInfo::Native {
                denom: UJUNO.to_string(),
                amount: Uint128::new(50),
            },
        },
    }, None, None)?;

    Ok(token_swap)

}

pub(crate) type AResult = anyhow::Result<()>; // alias for Result<(), anyhow::Error>

#[test]
fn basic_fund_test() -> AResult {
    let sender = Addr::unchecked("root");
    let (_state, chain) = instantiate_default_mock_env(&sender)?;

    let mut staking_contract = mock_init(chain.clone())?;

    // Set dao1 balance to 100
    let hundred_coins = Coin::new(100, UJUNO);
    let dao1_addr = Addr::unchecked(DAO1);

    chain.set_balance(
        &dao1_addr,
        vec![hundred_coins.clone()],
    )?;

    // Set dao1 balance to 100
    let fifty_coins = Coin::new(50, UJUNO);
    let dao2_addr = Addr::unchecked(DAO2);
    chain.set_balance(
        &dao2_addr,
        vec![fifty_coins.clone()],
    )?;

    // dao1 should send 100
    staking_contract.call_as(&dao1_addr).fund(&vec![hundred_coins])?;

    // dao2 with 50
    staking_contract.call_as(&dao2_addr).fund(&vec![fifty_coins])?;

    // balances should have swapped
    let dao1_balance = chain.query_balance(&dao1_addr, UJUNO)?;
    assert_that!(dao1_balance).is_equal_to(Uint128::new(51));

    let dao2_balance = chain.query_balance(&dao2_addr, UJUNO)?;
    assert_that!(dao2_balance).is_equal_to(Uint128::new(100));

    Ok(())
}

