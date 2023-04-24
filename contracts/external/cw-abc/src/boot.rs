use crate::msg::*;
use boot_core::{ArtifactsDir, ContractWrapper, Daemon, Mock, MockState, TxHandler, Uploadable, WasmPath};
use boot_core::{contract, Contract, CwEnv};
use cosmwasm_std::Empty;
use token_bindings::{TokenFactoryMsg, TokenFactoryQuery};

#[contract(InstantiateMsg, ExecuteMsg, QueryMsg, Empty)]
pub struct CwAbc<Chain>;

impl<Chain: CwEnv> CwAbc<Chain> {
    pub fn new(name: &str, chain: Chain) -> Self {
        let mut contract = Contract::new(name, chain);
        Self(contract)
    }
}

type TokenFactoryMock = Mock<MockState, TokenFactoryMsg, TokenFactoryQuery>;

impl Uploadable<TokenFactoryMock> for CwAbc<TokenFactoryMock> {
    fn source(&self) -> <TokenFactoryMock as TxHandler>::ContractSource {
        Box::new(ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        ))
    }
}

impl Uploadable<Daemon> for CwAbc<Daemon> {
    fn source(&self) -> <Daemon as TxHandler>::ContractSource {
        ArtifactsDir::env().expect("Expected ARTIFACTS_DIR in env").find_wasm_path("cw_abc").unwrap()
    }
}