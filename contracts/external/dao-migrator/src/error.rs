use cosmwasm_std::StdError;
use cw_utils::ParseReplyError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error(transparent)]
    StdError(#[from] StdError),

    #[error(transparent)]
    ParseReplyError(#[from] ParseReplyError),

    #[error("unauthorized")]
    Unauthorized,

    #[error("Error querying ContractInfo at address: {address}")]
    NoContractInfo{address: String},

    #[error("Can't migrate module, code id is not recognized. code_id: {code_id}")]
    CantMigrateModule{code_id: u64},

    #[error("unrecognised reply ID")]
    UnrecognisedReplyId,
    
    #[error("Test failed! New DAO state doesn't match the old DAO state.")]
    TestFailed,
    
    #[error("Failed to confirm migration of cw20_stake")]
    DontMigrateCw20,
    
    #[error("Failed to verify DAO core module address")]
    DaoCoreNotFound,
    
    #[error("Failed to verify any DAO proposal single module address")]
    DaoProposalSingleNotFound,
}