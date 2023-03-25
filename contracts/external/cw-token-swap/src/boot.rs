use crate::msg::{InstantiateMsg, ExecuteMsg, QueryMsg, MigrateMsg};

use boot_core::{
    boot_contract, BootEnvironment, Contract, IndexResponse, TxResponse,
    {BootQuery, ContractInstance},
};
use cw_multi_test::ContractWrapper;


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