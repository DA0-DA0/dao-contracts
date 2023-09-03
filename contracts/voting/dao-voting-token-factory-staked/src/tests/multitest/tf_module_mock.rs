use anyhow::{bail, Result as AnyResult};
use cosmwasm_schema::schemars::JsonSchema;
use cosmwasm_std::testing::{MockApi, MockStorage};
use cosmwasm_std::{
    Addr, Api, Binary, BlockInfo, CustomQuery, Empty, Querier, QuerierResult, StdError, Storage,
};
use cw_multi_test::{
    App, AppResponse, BankKeeper, BasicAppBuilder, CosmosRouter, Module, WasmKeeper,
};
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use token_bindings::TokenFactoryMsg;

// A Mock cw-multi-test module for token factory
pub struct TokenFactoryModule {}

impl Module for TokenFactoryModule {
    type ExecT = TokenFactoryMsg;
    type QueryT = Empty;
    type SudoT = Empty;

    // Builds a mock rust implementation of the expected Token Factory functionality for testing
    fn execute<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _sender: Addr,
        msg: Self::ExecT,
    ) -> AnyResult<AppResponse>
    where
        ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        match msg {
            _ => bail!("execute not implemented for TokenFactoryModule"),
        }
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
        ExecC: Debug + Clone + PartialEq + JsonSchema + DeserializeOwned + 'static,
        QueryC: CustomQuery + DeserializeOwned + 'static,
    {
        bail!("sudo not implemented for TokenFactoryModule")
    }

    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        _request: Self::QueryT,
    ) -> anyhow::Result<Binary> {
        bail!("query not implemented for TokenFactoryModule")
    }
}

pub type TokenFactoryAppWrapped =
    App<BankKeeper, MockApi, MockStorage, TokenFactoryModule, WasmKeeper<TokenFactoryMsg, Empty>>;

pub struct TokenFactoryApp(TokenFactoryAppWrapped);

impl Deref for TokenFactoryApp {
    type Target = TokenFactoryAppWrapped;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TokenFactoryApp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Querier for TokenFactoryApp {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        self.0.raw_query(bin_request)
    }
}

impl Default for TokenFactoryApp {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenFactoryApp {
    pub fn new() -> Self {
        Self(
            BasicAppBuilder::<TokenFactoryMsg, Empty>::new_custom()
                .with_custom(TokenFactoryModule {})
                .build(|_router, _, _storage| {}),
        )
    }
}
