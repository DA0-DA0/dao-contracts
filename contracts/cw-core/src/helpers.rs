use cosmwasm_std::{Addr, WasmMsg};

use crate::msg::{Admin, ModuleInstantiateInfo};

impl ModuleInstantiateInfo {
    pub fn into_wasm_msg(self, contract_address: Addr) -> WasmMsg {
        WasmMsg::Instantiate {
            admin: match self.admin {
                Admin::Address { addr } => Some(addr),
                Admin::GovernanceContract {} => Some(contract_address.to_string()),
                Admin::None {} => None,
            },
            code_id: self.code_id,
            msg: self.msg,
            funds: vec![],
            label: self.label,
        }
    }
}
