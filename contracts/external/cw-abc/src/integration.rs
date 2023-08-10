use anyhow::Result as AnyResult;
use cosmwasm_schema::{schemars::JsonSchema, serde::de::DeserializeOwned};
use cosmwasm_std::{
    coin, coins,
    testing::{MockApi, MockStorage},
    Addr, Api, Binary, BlockInfo, Coin, CustomQuery, Decimal, Empty, GovMsg, IbcMsg, IbcQuery,
    Querier, StdResult, Storage, Uint128,
};
use cw_multi_test::{
    custom_app,
    custom_handler::{CachingCustomHandler, CachingCustomHandlerState},
    next_block, App, AppBuilder, AppResponse, BankKeeper, BankSudo, BasicAppBuilder, Contract,
    ContractWrapper, CosmosRouter, DistributionKeeper, Executor, FailingModule, Module,
    StakeKeeper, SudoMsg, WasmKeeper,
};
use token_bindings::{Metadata, TokenFactoryMsg, TokenFactoryQuery, TokenMsg};

use crate::{
    abc::{
        ClosedConfig, CommonsPhaseConfig, HatchConfig, MinMax, OpenConfig, ReserveToken,
        SupplyToken,
    },
    msg::{CurveInfoResponse, InstantiateMsg},
};

pub struct CustomHandler {}

impl Module for CustomHandler {
    type ExecT = TokenFactoryMsg;
    type QueryT = TokenFactoryQuery;
    type SudoT = Empty;

    fn execute<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        _sender: Addr,
        msg: Self::ExecT,
    ) -> AnyResult<AppResponse>
    where
        ExecC: std::fmt::Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        println!("msg {:?}", msg);
        match msg {
            TokenFactoryMsg::Token(TokenMsg::MintTokens {
                denom,
                amount,
                mint_to_address,
            }) => {
                println!("minting tokens");

                // mint new tokens
                let mint = SudoMsg::Bank(BankSudo::Mint {
                    to_address: mint_to_address,
                    amount: vec![Coin {
                        denom: denom.clone(),
                        amount: amount.clone(),
                    }],
                });
                return Ok(router.sudo(api, storage, block, mint)?);
            }
            TokenFactoryMsg::Token(TokenMsg::CreateDenom { subdenom, metadata }) => {
                println!("creating denom");
                return Ok(AppResponse::default());
            }
            _ => unimplemented!(),
        };
    }

    fn sudo<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _msg: Self::SudoT,
    ) -> AnyResult<AppResponse>
    where
        ExecC: std::fmt::Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        unimplemented!()
    }

    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        _request: Self::QueryT,
    ) -> AnyResult<Binary> {
        unimplemented!()
    }
}

// impl CustomHandler {
//     // this is a custom initialization method
//     pub fn set_payout(
//         &self,
//         storage: &mut dyn Storage,
//         lottery: Coin,
//         pity: Coin,
//     ) -> AnyResult<()> {
//         LOTTERY.save(storage, &lottery)?;
//         PITY.save(storage, &pity)?;
//         Ok(())
//     }
// }

pub struct Test {
    pub app: App<
        BankKeeper,
        MockApi,
        MockStorage,
        // CachingCustomHandler<TokenFactoryMsg, TokenFactoryQuery>,
        // FailingModule<TokenFactoryMsg, TokenFactoryQuery, Empty>,
        CustomHandler,
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

        // let mut app = AppBuilder::new_custom()
        //     .with_custom(custom_handler)
        //     .build(|_, _, _| {});

        // let mut app = custom_app::<TokenFactoryMsg, TokenFactoryQuery, _>(|router, _, storage| {
        //     router
        //         .bank
        //         .init_balance(storage, &owner, coins(10000, "ujuno"))
        //         .unwrap();
        // });

        let mut app = BasicAppBuilder::<TokenFactoryMsg, TokenFactoryQuery>::new_custom()
            .with_custom(CustomHandler {})
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

    pub fn burn(&mut self, amount: Vec<Coin>) -> Result<AppResponse, anyhow::Error> {
        let msg = crate::msg::ExecuteMsg::Burn {};
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

    // // Update block
    // test.app.update_block(next_block);

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

    // Assert balance
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

    // TODO get denom
    let tf_balance = test
        .app
        .wrap()
        .query_balance(test.addr.clone(), "factory/contract0/subdenom")
        .unwrap();
    // TODO how to handle this?
    println!("{:?}", tf_balance);

    // // Burn some coins
    // test.burn(coins(100, "ujuno")).unwrap();

    // let curve_info = test.query_curve_info().unwrap();
    // println!("{:?}", curve_info);
}
