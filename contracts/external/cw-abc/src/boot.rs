use crate::msg::*;
use cosmwasm_std::Empty;
use cw_orch::{contract, Contract, CwEnv};
#[cfg(feature = "daemon")]
use cw_orch::{ArtifactsDir, Daemon, WasmPath};
use cw_orch::{ContractWrapper, Mock, MockState, TxHandler, Uploadable};
use token_bindings::{TokenFactoryMsg, TokenFactoryQuery};

#[contract(InstantiateMsg, ExecuteMsg, QueryMsg, Empty)]
pub struct CwAbc<Chain>;

impl<Chain: CwEnv> CwAbc<Chain> {
    pub fn new(name: &str, chain: Chain) -> Self {
        let contract = Contract::new(name, chain);
        Self(contract)
    }
}

/// Basic app for the token factory contract
/// TODO: should be in the bindings, along with custom handler for multi-test
pub(crate) type TokenFactoryBasicApp = cw_orch::BasicApp<TokenFactoryMsg, TokenFactoryQuery>;

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

#[cfg(feature = "daemon")]
impl Uploadable<Daemon> for CwAbc<Daemon> {
    fn source(&self) -> <Daemon as TxHandler>::ContractSource {
        ArtifactsDir::env()
            .expect("Expected ARTIFACTS_DIR in env")
            .find_wasm_path("cw_abc")
            .unwrap()
    }
}
