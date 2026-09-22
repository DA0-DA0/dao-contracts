use thiserror::Error;

/// Only an explicit application rejection belongs in TxError. In particular,
/// networking, decoding and timeout errors must not pass rejection assertions.
#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("configuration: {0}")]
    Config(String),
    #[error("signing: {0}")]
    Signing(String),
    #[error("RPC transport/protocol: {0}")]
    Rpc(String),
    #[error("broadcast outcome unknown for {hash}: {message}; do not automatically resubmit")]
    Broadcast { hash: String, message: String },
    #[error("invalid chain response: {0}")]
    Protocol(String),
    #[error("ABCI query rejected ({codespace}/{code}): {log}")]
    Query {
        code: u32,
        codespace: String,
        log: String,
    },
    #[error(transparent)]
    CosmwasmError(#[from] CosmwasmError),
    #[error("operation deadline exceeded; transaction outcome may be unknown (hash: {hash:?}); do not automatically resubmit")]
    Timeout { hash: Option<String> },
    #[error("contract registry: {0}")]
    Registry(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum CosmwasmError {
    #[error(transparent)]
    TxError(#[from] TxError),
}

#[derive(Debug, Error)]
pub enum TxError {
    #[error("simulation rejected ({codespace}/{code}): {log}")]
    Simulation {
        code: u32,
        codespace: String,
        log: String,
    },
    #[error("CheckTx rejected {hash} ({codespace}/{code}): {log}")]
    CheckTx {
        code: u32,
        codespace: String,
        log: String,
        hash: String,
    },
    #[error("execution rejected {hash} ({codespace}/{code}): {log}")]
    Execution {
        code: u32,
        codespace: String,
        log: String,
        hash: String,
        /// Preserve the committed rejection's data, events and gas for diagnosis.
        response: Box<crate::rpc::Receipt>,
    },
}

impl From<TxError> for ProcessError {
    fn from(error: TxError) -> Self {
        CosmwasmError::TxError(error).into()
    }
}

impl From<tendermint_rpc::Error> for ProcessError {
    fn from(error: tendermint_rpc::Error) -> Self {
        Self::Rpc(error.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ProcessError>;

pub(crate) fn protocol(error: impl std::fmt::Display) -> ProcessError {
    ProcessError::Protocol(error.to_string())
}
