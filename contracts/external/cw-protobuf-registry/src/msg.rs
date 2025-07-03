use cosmwasm_schema::{cw_serde, QueryResponses};

pub use cw_ownable::Ownership;
use cw_ownable::{cw_ownable_execute, cw_ownable_query};
use dao_interface::proposal::InfoResponse;

#[cw_serde]
pub struct InstantiateMsg {
    /// The address of the initial owner of the contract. Defaults to the
    /// sender.
    pub owner: Option<String>,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    /// Register protobuf file descriptor sets.
    Register {
        /// The protobuf file descriptor sets to register. This will override
        /// existing files with the same names.
        file_descriptor_sets: Vec<Vec<u8>>,
    },
    /// Unregister protobuf files and their message descriptors.
    Unregister {
        /// The names of the protobuf files to unregister.
        file_names: Vec<String>,
        /// The maximum number of message descriptors to unregister. If not
        /// provided, it will attempt to unregister all message descriptors,
        /// running out of gas if there are too many. If the limit is too low
        /// such that we never progress to and delete the last file, it will
        /// return an error.
        message_limit: Option<u32>,
    },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(InfoResponse)]
    Info {},
    #[returns(ListProtobufFilesResponse)]
    ListFiles {
        /// The file name to start after. If not provided, the query will start
        /// from the beginning.
        start_after: Option<String>,
        /// The maximum number of files to return. Defaults to 10, max is 100.
        limit: Option<u32>,
    },
    #[returns(ListProtobufMessagesResponse)]
    ListMessages {
        /// The message name to start after. If not provided, the query will
        /// start from the beginning.
        start_after: Option<String>,
        /// The maximum number of messages to return. Defaults to 10, max is
        /// 100.
        limit: Option<u32>,
    },
    #[returns(ListProtobufMessagesResponse)]
    ListMessagesByFile {
        /// The file name to list messages for.
        file_name: String,
        /// The messages name to start after. If not provided, the query will
        /// start from the beginning.
        start_after: Option<String>,
        /// The maximum number of messages to return. Defaults to 10, max is
        /// 100.
        limit: Option<u32>,
    },
    #[returns(FileDescriptorSetResponse)]
    FileDescriptorSet {
        /// The messages to include in the file descriptor set.
        messages: Vec<String>,
    },
}

#[cw_serde]
pub struct MigrateMsg {}

// Response types

#[cw_serde]
pub struct ListProtobufFilesResponse {
    pub files: Vec<String>,
}

#[cw_serde]
pub struct ListProtobufMessagesResponse {
    pub messages: Vec<String>,
}

#[cw_serde]
pub struct FileDescriptorSetResponse {
    pub file_descriptor_set: Vec<u8>,
}
