use cosmwasm_std::{
    coin, coins,
    testing::{MockApi, MockStorage},
    Addr, Api, Coin, Decimal, Empty, GovMsg, IbcMsg, IbcQuery, StdResult, Storage, Uint128,
};
use cw_multi_test::{
    custom_app,
    custom_handler::{CachingCustomHandler, CachingCustomHandlerState},
    App, AppBuilder, AppResponse, BankKeeper, BankSudo, Contract, ContractWrapper,
    DistributionKeeper, Executor, FailingModule, Router, StakeKeeper, WasmKeeper,
};
use cw_utils::PaymentError;
use token_bindings::{Metadata, TokenFactoryMsg, TokenFactoryQuery};

use crate::{
    abc::{
        ClosedConfig, CommonsPhaseConfig, HatchConfig, MinMax, OpenConfig, ReserveToken,
        SupplyToken,
    },
    msg::{CurveInfoResponse, InstantiateMsg},
    ContractError,
};

pub struct Test {
    pub app: App<
        BankKeeper,
        MockApi,
        MockStorage,
        CachingCustomHandler<TokenFactoryMsg, TokenFactoryQuery>,
        WasmKeeper<TokenFactoryMsg, TokenFactoryQuery>,
        StakeKeeper,
        DistributionKeeper,
        FailingModule<IbcMsg, IbcQuery, Empty>,
        FailingModule<GovMsg, Empty, Empty>,
    >,
    pub addr: Addr,
    pub owner: Addr,
    pub recipient: Addr,
    pub custom_handler_state: CachingCustomHandlerState<TokenFactoryMsg, TokenFactoryQuery>,
}

impl Test {
    pub fn new() -> Self {
        let owner = Addr::unchecked("owner");
        let recipient = Addr::unchecked("recipient");

        let custom_handler = CachingCustomHandler::<TokenFactoryMsg, TokenFactoryQuery>::new();
        let custom_handler_state = custom_handler.state();

        let mut app = AppBuilder::new_custom()
            .with_custom(custom_handler)
            .build(|_, _, _| {});

        app.sudo(cw_multi_test::SudoMsg::Bank(BankSudo::Mint {
            to_address: owner.to_string(),
            amount: vec![coin(10000, "ujuno"), coin(10000, "uatom")],
        }))
        .unwrap();
        app.sudo(cw_multi_test::SudoMsg::Bank(BankSudo::Mint {
            to_address: recipient.to_string(),
            amount: vec![coin(10000, "ujuno"), coin(10000, "uatom")],
        }))
        .unwrap();

        let code_id = app.store_code(abc_countract());
        let addr = app
            .instantiate_contract(
                code_id,
                owner.clone(),
                &InstantiateMsg {
                    supply: SupplyToken {
                        subdenom: "subdenom".to_string(),
                        metadata: Metadata {
                            description: None,
                            denom_units: vec![],
                            base: None,
                            display: None,
                            name: None,
                            symbol: None,
                        },
                        decimals: 6,
                    },
                    reserve: ReserveToken {
                        denom: "ujuno".to_string(),
                        decimals: 6,
                    },
                    curve_type: crate::abc::CurveType::Linear {
                        slope: Uint128::new(1),
                        scale: 2,
                    },
                    phase_config: CommonsPhaseConfig {
                        hatch: HatchConfig {
                            initial_raise: MinMax {
                                min: Uint128::new(100),
                                max: Uint128::new(1000),
                            },
                            initial_price: Uint128::new(1),
                            initial_allocation_ratio: Decimal::percent(10),
                            exit_tax: Decimal::percent(10),
                        },
                        open: OpenConfig {
                            allocation_percentage: Decimal::percent(10),
                            exit_tax: Decimal::percent(10),
                        },
                        closed: ClosedConfig {},
                    },
                    hatcher_allowlist: None,
                },
                &[],
                "cw-bounties",
                None,
            )
            .unwrap();
        Self {
            app,
            addr,
            owner,
            recipient,
            custom_handler_state,
        }
    }

    pub fn buy(&mut self, amount: Vec<Coin>) -> Result<AppResponse, anyhow::Error> {
        let msg = crate::msg::ExecuteMsg::Buy {};
        let res =
            self.app
                .execute_contract(self.owner.clone(), self.addr.clone(), &msg, &amount)?;
        Ok(res)
    }

    pub fn query_curve_info(&self) -> StdResult<CurveInfoResponse> {
        let msg = crate::msg::QueryMsg::CurveInfo {};
        self.app.wrap().query_wasm_smart(&self.addr, &msg)
    }
}

fn abc_countract() -> Box<dyn Contract<TokenFactoryMsg, TokenFactoryQuery>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

#[test]
pub fn test_happy_path() {
    let mut test = Test::new();

    // Curve has been initialized
    let curve_info = test.query_curve_info().unwrap();
    assert_eq!(
        curve_info,
        CurveInfoResponse {
            reserve: Uint128::zero(),
            supply: Uint128::zero(),
            funding: Uint128::zero(),
            spot_price: Decimal::zero(),
            reserve_denom: "ujuno".to_string(),
        }
    );

    let balance_before = test
        .app
        .wrap()
        .query_balance(test.owner.clone(), "ujuno")
        .unwrap();

    // Buy some coins
    test.buy(coins(100, "ujuno")).unwrap();

    // Curve has been updated
    let curve_info = test.query_curve_info().unwrap();
    assert_eq!(
        curve_info,
        CurveInfoResponse {
            reserve: Uint128::new(90),
            supply: Uint128::new(134164),
            funding: Uint128::new(10),
            // TODO investigate why does this take 8 for decimals?
            spot_price: Decimal::from_atomics(Uint128::new(134164), 8).unwrap(),
            reserve_denom: "ujuno".to_string(),
        }
    );

    // assert balance
    let balance_after = test
        .app
        .wrap()
        .query_balance(test.owner.clone(), "ujuno")
        .unwrap();
    assert!(
        balance_before.amount.u128() == (balance_after.amount.u128() + 100),
        "before: {}, after: {}",
        balance_before.amount.u128(),
        balance_after.amount.u128()
    );
}
