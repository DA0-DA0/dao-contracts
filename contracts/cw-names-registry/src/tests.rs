use crate::msg::InstantiateMsg;
use cosmwasm_std::{to_binary, Addr, Decimal, Empty, Uint128};
use cw2::ContractVersion;
use cw20::{BalanceResponse, Cw20Coin, MinterResponse, TokenInfoResponse};
use cw_multi_test::{next_block, App, Contract, ContractWrapper, Executor};

const DAO_ADDR: &str = "dao";
const ADMIN_ADDR: &str = "admin";
const NON_ADMIN_ADDR: &str = "nonadmin";

fn cw20_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

fn names_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn setup_test_case(app: &mut App, payment_amount: Uint128) -> (Addr, Addr) {
    let cw20_id = app.store_code(cw20_contract());
    let names_id = app.store_code(names_contract());

    let token_addr = app
        .instantiate_contract(
            cw20_id,
            Addr::unchecked(ADMIN_ADDR),
            &cw20_base::msg::InstantiateMsg {
                name: "Name Registry Token".to_string(),
                symbol: "NAME".to_string(),
                decimals: 6,
                initial_balances: vec![
                    Cw20Coin {
                        address: DAO_ADDR.to_string(),
                        amount: Uint128::new(1000),
                    },
                    Cw20Coin {
                        address: ADMIN_ADDR.to_string(),
                        amount: Uint128::new(1000),
                    },
                    Cw20Coin {
                        address: NON_ADMIN_ADDR.to_string(),
                        amount: Uint128::new(1000),
                    },
                ],
                mint: None,
                marketing: None,
            },
            &[],
            "name cw20",
            None,
        )
        .unwrap();

    let names_addr = app
        .instantiate_contract(
            names_id,
            Addr::unchecked(ADMIN_ADDR),
            &InstantiateMsg {
                admin: ADMIN_ADDR.to_string(),
                payment_token_address: token_addr.to_string(),
                payment_amount,
            },
            &[],
            "DAO Names Registry",
            None,
        )
        .unwrap();

    (names_addr, token_addr)
}

#[test]
fn test_instantiate() {
    let mut app = App::default();
    let (_names, _token) = setup_test_case(&mut app, Uint128::new(50));
}
