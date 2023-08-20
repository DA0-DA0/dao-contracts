use cosmwasm_std::StdError;
use cw_utils::ParseReplyError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    StdError(#[from] StdError),

    #[error(transparent)]
    ParseReplyError(#[from] ParseReplyError),

    #[error("unauthorized")]
    Unauthorized,

    #[error("Error querying ContractInfo at address: {address}")]
    NoContractInfo { address: String },

    #[error("Can't migrate module, code id is not recognized. code_id: {code_id}")]
    CantMigrateModule { code_id: u64 },

    #[error("unrecognised reply ID")]
    UnrecognisedReplyId,

    #[error("Test failed! New DAO state doesn't match the old DAO state.")]
    TestFailed,

    #[error("Failed to confirm migration of cw20_stake")]
    DontMigrateCw20,

    #[error("Failed to verify DAO voting module address")]
    VotingModuleNotFound,

    #[error("Failed to verify any DAO proposal single module address")]
    DaoProposalSingleNotFound,

    #[error("We couldn't find the proposal modules in provided migration params: {addr}")]
    ProposalModuleNotFoundInParams { addr: String },

    #[error("Failed to verify proposal in {module_addr}")]
    NoProposalsOnModule { module_addr: String },

    #[error("Duplicate params found for the same module")]
    DuplicateProposalParams,

    #[error("Proposal migration params length is not equal to proposal modules length")]
    MigrationParamsNotEqualProposalModulesLength,
}
