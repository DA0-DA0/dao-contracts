use cosmwasm_std::to_json_binary;

use super::*;

pub struct DaoTestingSuiteCw4<'a> {
    pub base: &'a mut DaoTestingSuiteBase,

    pub members: Vec<cw4::Member>,
}

impl<'a> DaoTestingSuiteCw4<'a> {
    pub fn new(base: &'a mut DaoTestingSuiteBase) -> Self {
        Self {
            base,
            members: vec![
                cw4::Member {
                    addr: MEMBER1.to_string(),
                    weight: 1,
                },
                cw4::Member {
                    addr: MEMBER2.to_string(),
                    weight: 2,
                },
                cw4::Member {
                    addr: MEMBER3.to_string(),
                    weight: 3,
                },
                cw4::Member {
                    addr: MEMBER4.to_string(),
                    weight: 3,
                },
                cw4::Member {
                    addr: MEMBER5.to_string(),
                    weight: 1,
                },
            ],
        }
    }

    pub fn with_members(&mut self, members: Vec<cw4::Member>) -> &mut Self {
        self.members = members;
        self
    }
}

impl<'a> DaoTestingSuite for DaoTestingSuiteCw4<'a> {
    fn base(&self) -> &DaoTestingSuiteBase {
        self.base
    }

    fn base_mut(&mut self) -> &mut DaoTestingSuiteBase {
        self.base
    }

    fn get_voting_module_info(&self) -> dao_interface::state::ModuleInstantiateInfo {
        dao_interface::state::ModuleInstantiateInfo {
            code_id: self.base.voting_cw4_id,
            msg: to_json_binary(&dao_voting_cw4::msg::InstantiateMsg {
                group_contract: dao_voting_cw4::msg::GroupContract::New {
                    cw4_group_code_id: self.base.cw4_group_id,
                    initial_members: self.members.clone(),
                },
            })
            .unwrap(),
            admin: Some(dao_interface::state::Admin::CoreModule {}),
            funds: vec![],
            label: "voting module".to_string(),
        }
    }
}
