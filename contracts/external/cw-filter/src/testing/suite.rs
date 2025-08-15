use cosmwasm_std::{to_json_binary, Addr, CosmosMsg, Timestamp};
use cw_filter::ContractError;
use cw_ownable::Action;
use dao_interface::{
    proposal::InfoResponse,
    state::{ModuleInstantiateInfo, ModuleUpdate},
};
use dao_testing::{DaoTestingSuiteBase, OWNER};

use crate::msg::{ExecuteMsg, FilterResponse, InstantiateMsg, ProtobufRegistryResponse, QueryMsg};

pub struct SuiteBuilder {}

impl SuiteBuilder {
    pub fn base() -> Self {
        Self {}
    }

    pub fn build(self) -> Suite {
        let mut suite = Suite {
            base: DaoTestingSuiteBase::base(),
            filter_addr: Addr::unchecked(""),
            protobuf_registry_addr: Addr::unchecked(""),
        };

        // start at 0 height and time
        suite.base.app.update_block(|b| {
            b.height = 0;
            b.time = Timestamp::from_seconds(0);
        });

        // initialize the contract
        suite.filter_addr = suite.base.instantiate(
            suite.base.filter_id,
            OWNER,
            &InstantiateMsg {
                owner: None,
                protobuf_registry: Some(ModuleUpdate::New(ModuleInstantiateInfo {
                    code_id: suite.base.filter_id,
                    msg: to_json_binary(&cw_protobuf_registry::msg::InstantiateMsg { owner: None })
                        .unwrap(),
                    admin: Some(dao_interface::state::Admin::CoreModule {}),
                    funds: None,
                    label: "filter".to_string(),
                    salt: None,
                })),
            },
            &[],
            "filter",
            None,
        );

        suite.protobuf_registry_addr = suite.get_protobuf_registry().unwrap();

        suite
    }
}

pub struct Suite {
    pub base: DaoTestingSuiteBase,
    pub filter_addr: Addr,
    pub protobuf_registry_addr: Addr,
}

// SUITE QUERIES
impl Suite {
    pub fn get_ownership(&mut self) -> cw_ownable::Ownership<Addr> {
        self.base
            .app
            .wrap()
            .query_wasm_smart(self.filter_addr.clone(), &QueryMsg::Ownership {})
            .unwrap()
    }

    pub fn get_info(&mut self) -> InfoResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(self.filter_addr.clone(), &QueryMsg::Info {})
            .unwrap()
    }

    pub fn get_protobuf_registry(&mut self) -> Option<Addr> {
        self.base
            .app
            .wrap()
            .query_wasm_smart::<ProtobufRegistryResponse>(
                self.filter_addr.clone(),
                &QueryMsg::ProtobufRegistry {},
            )
            .unwrap()
            .protobuf_registry
    }

    pub fn filter(&mut self, filter: serde_json::Value, msg: CosmosMsg) -> FilterResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(self.filter_addr.clone(), &QueryMsg::Filter { filter, msg })
            .unwrap()
    }
}

// SUITE ASSERTIONS
impl Suite {
    pub fn assert_protobuf_registry(&mut self, expected_protobuf_registry: Option<Addr>) {
        let protobuf_registry = self.get_protobuf_registry();
        assert_eq!(protobuf_registry, expected_protobuf_registry);
    }

    pub fn assert_filter(
        &mut self,
        filter: serde_json::Value,
        msg: CosmosMsg,
        expected_response: FilterResponse,
    ) {
        let response = self.filter(filter, msg);
        assert_eq!(response, expected_response);
    }
}

// SUITE ACTIONS
impl Suite {
    pub fn update_owner(&mut self, old_owner: impl Into<String>, new_owner: impl Into<String>) {
        let new_owner = new_owner.into();

        let msg = ExecuteMsg::UpdateOwnership(Action::TransferOwnership {
            new_owner: new_owner.clone(),
            expiry: None,
        });

        self.base
            .execute_smart_ok(old_owner, &self.filter_addr, &msg, &[]);

        self.base.execute_smart_ok(
            new_owner,
            &self.filter_addr,
            &ExecuteMsg::UpdateOwnership(Action::AcceptOwnership {}),
            &[],
        );
    }

    pub fn update_protobuf_registry(&mut self, protobuf_registry: Option<ModuleUpdate>) {
        self.base.execute_smart_ok(
            OWNER,
            &self.filter_addr,
            &ExecuteMsg::UpdateProtobufRegistry { protobuf_registry },
            &[],
        );
    }

    pub fn update_protobuf_registry_err(
        &mut self,
        owner: impl Into<String>,
        protobuf_registry: Option<ModuleUpdate>,
    ) -> ContractError {
        self.base.execute_smart_err(
            owner,
            &self.filter_addr,
            &ExecuteMsg::UpdateProtobufRegistry { protobuf_registry },
            &[],
        )
    }
}
