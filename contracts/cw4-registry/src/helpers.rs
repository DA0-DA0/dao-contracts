use crate::msg::{ExecuteMsg, ListGroupsResponse, QueryMsg};
use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, Empty, Querier, QuerierWrapper, StdResult, WasmMsg, WasmQuery,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Cw4RegistryContract(pub Addr);

impl Cw4RegistryContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn new(addr: Addr) -> Self {
        Cw4RegistryContract(addr)
    }

    fn encode_msg(&self, msg: ExecuteMsg) -> StdResult<CosmosMsg> {
        Ok(WasmMsg::Execute {
            contract_addr: self.0.clone().into_string(),
            msg: to_binary(&msg)?,
            funds: vec![],
        }
        .into())
    }

    /// Get token balance for the given address
    pub fn list_group<Q: Querier, T: Into<String>>(
        &self,
        querier: &Q,
        address: T,
    ) -> StdResult<ListGroupsResponse> {
        let msg = QueryMsg::ListGroups {
            user_addr: address.into(),
            start_after: None,
            limit: None,
        };
        let query = WasmQuery::Smart {
            contract_addr: self.0.clone().into(),
            msg: to_binary(&msg)?,
        }
        .into();
        let res: ListGroupsResponse = QuerierWrapper::<Empty>::new(querier).query(&query)?;
        Ok(res)
    }
}
