use cosmwasm_std::{Addr, StdError, Timestamp};
use cw_ownable::Action;
use cw_protobuf_registry::ContractError;
use dao_interface::proposal::InfoResponse;
use dao_testing::{DaoTestingSuiteBase, OWNER};
use prost::Message;
use prost_types::FileDescriptorSet;

use crate::msg::{
    DecodeResponse, ExecuteMsg, FileDescriptorSetResponse, InstantiateMsg, ListPreparedResponse,
    ListProtobufFilesResponse, ListProtobufMessagesResponse, PreparedResponse, QueryMsg,
};

pub struct SuiteBuilder {}

impl SuiteBuilder {
    pub fn base() -> Self {
        Self {}
    }

    pub fn build(self) -> Suite {
        let mut suite = Suite {
            base: DaoTestingSuiteBase::base(),
            registry_addr: Addr::unchecked(""),
        };

        // start at 0 height and time
        suite.base.app.update_block(|b| {
            b.height = 0;
            b.time = Timestamp::from_seconds(0);
        });

        // initialize the contract
        suite.registry_addr = suite.base.instantiate(
            suite.base.protobuf_registry_id,
            OWNER,
            &InstantiateMsg { owner: None },
            &[],
            "protobuf-registry",
            None,
        );

        suite
    }
}

pub struct Suite {
    pub base: DaoTestingSuiteBase,
    pub registry_addr: Addr,
}

// SUITE QUERIES
impl Suite {
    pub fn get_ownership(&mut self) -> cw_ownable::Ownership<Addr> {
        self.base
            .app
            .wrap()
            .query_wasm_smart(self.registry_addr.clone(), &QueryMsg::Ownership {})
            .unwrap()
    }

    pub fn get_info(&mut self) -> InfoResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(self.registry_addr.clone(), &QueryMsg::Info {})
            .unwrap()
    }

    pub fn list_protobuf_files(
        &mut self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> ListProtobufFilesResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                self.registry_addr.clone(),
                &QueryMsg::ListFiles { start_after, limit },
            )
            .unwrap()
    }

    pub fn list_protobuf_messages(
        &mut self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> ListProtobufMessagesResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                self.registry_addr.clone(),
                &QueryMsg::ListMessages { start_after, limit },
            )
            .unwrap()
    }

    pub fn list_protobuf_messages_by_file(
        &mut self,
        file_name: String,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> ListProtobufMessagesResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                self.registry_addr.clone(),
                &QueryMsg::ListMessagesByFile {
                    file_name,
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn list_prepared(
        &mut self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> ListPreparedResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                self.registry_addr.clone(),
                &QueryMsg::ListPrepared { start_after, limit },
            )
            .unwrap()
    }

    pub fn prepared(&mut self, message_name: String) -> PreparedResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                self.registry_addr.clone(),
                &QueryMsg::Prepared { message_name },
            )
            .unwrap()
    }

    pub fn decode(&mut self, message_name: impl Into<String>, value: Vec<u8>) -> DecodeResponse {
        self.base
            .app
            .wrap()
            .query_wasm_smart(
                self.registry_addr.clone(),
                &QueryMsg::Decode {
                    message_name: message_name.into(),
                    value,
                },
            )
            .unwrap()
    }

    pub fn decode_err(&mut self, message_name: impl Into<String>, value: Vec<u8>) -> StdError {
        self.base
            .app
            .wrap()
            .query_wasm_smart::<DecodeResponse>(
                self.registry_addr.clone(),
                &QueryMsg::Decode {
                    message_name: message_name.into(),
                    value,
                },
            )
            .unwrap_err()
    }

    pub fn file_descriptor_set(&mut self, messages: Vec<String>) -> FileDescriptorSet {
        let response: FileDescriptorSetResponse = self
            .base
            .app
            .wrap()
            .query_wasm_smart(
                self.registry_addr.clone(),
                &QueryMsg::FileDescriptorSet { messages },
            )
            .unwrap();

        FileDescriptorSet::decode(response.file_descriptor_set.as_slice()).unwrap()
    }

    pub fn file_descriptor_set_err(&mut self, messages: Vec<String>) -> StdError {
        self.base
            .app
            .wrap()
            .query_wasm_smart::<FileDescriptorSetResponse>(
                self.registry_addr.clone(),
                &QueryMsg::FileDescriptorSet { messages },
            )
            .unwrap_err()
    }
}

// SUITE ASSERTIONS
impl Suite {
    pub fn assert_protobuf_files(&mut self, expected: Vec<String>) {
        let response = self.list_protobuf_files(None, None);
        assert_eq!(response.files, expected);
    }

    pub fn assert_protobuf_messages(&mut self, expected: Vec<String>) {
        let response = self.list_protobuf_messages(None, None);
        assert_eq!(response.messages, expected);
    }

    pub fn assert_protobuf_messages_by_file(&mut self, file_name: &str, expected: Vec<String>) {
        let response = self.list_protobuf_messages_by_file(file_name.to_string(), None, None);
        assert_eq!(response.messages, expected);
    }

    pub fn assert_prepared(&mut self, message_name: &str, expected: bool) {
        let response = self.prepared(message_name.to_string());
        assert_eq!(response.prepared, expected);
    }

    pub fn assert_list_prepared(&mut self, expected: Vec<String>) {
        let response = self.list_prepared(None, None);
        assert_eq!(response.messages, expected);
    }

    pub fn assert_decode(
        &mut self,
        message_name: &str,
        value: Vec<u8>,
        expected: serde_json::Value,
    ) {
        let response = self.decode(message_name.to_string(), value);
        assert_eq!(response.value, expected);
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
            .execute_smart_ok(old_owner, &self.registry_addr, &msg, &[]);

        self.base.execute_smart_ok(
            new_owner,
            &self.registry_addr,
            &ExecuteMsg::UpdateOwnership(Action::AcceptOwnership {}),
            &[],
        );
    }

    pub fn register_protobufs(
        &mut self,
        sender: impl Into<String>,
        file_descriptor_sets: Vec<Vec<u8>>,
    ) {
        self.base.execute_smart_ok(
            sender,
            &self.registry_addr,
            &ExecuteMsg::Register {
                file_descriptor_sets,
            },
            &[],
        );
    }

    pub fn unregister_protobufs(
        &mut self,
        sender: impl Into<String>,
        file_names: Vec<String>,
        message_limit: Option<u32>,
    ) {
        self.base.execute_smart_ok(
            sender,
            &self.registry_addr,
            &ExecuteMsg::Unregister {
                file_names,
                message_limit,
            },
            &[],
        );
    }

    pub fn unregister_protobufs_err(
        &mut self,
        sender: impl Into<String>,
        file_names: Vec<String>,
        message_limit: Option<u32>,
    ) -> ContractError {
        self.base.execute_smart_err(
            sender,
            &self.registry_addr,
            &ExecuteMsg::Unregister {
                file_names,
                message_limit,
            },
            &[],
        )
    }

    pub fn prepare(&mut self, sender: impl Into<String>, messages: Vec<String>) {
        self.base.execute_smart_ok(
            sender,
            &self.registry_addr,
            &ExecuteMsg::Prepare { messages },
            &[],
        );
    }

    pub fn unprepare(&mut self, sender: impl Into<String>, messages: Vec<String>) {
        self.base.execute_smart_ok(
            sender,
            &self.registry_addr,
            &ExecuteMsg::Unprepare { messages },
            &[],
        );
    }
}
