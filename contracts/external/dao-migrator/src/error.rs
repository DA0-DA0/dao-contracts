use cosmwasm_std::StdError;
use cw_utils::ParseReplyError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    ParseReplyError(#[from] ParseReplyError),

    #[error("unauthorized")]
    Unauthorized,

    #[error("Error querying ContractInfo from contract: {prefix} at address: {address}")]
    NoContractInfo{prefix: String, address: String},

    #[error("Can't migrate module: {prefix}, code id is not recognized. code_id: {code_id}")]
    CantMigrateModule{prefix: String, code_id: u64},
}
