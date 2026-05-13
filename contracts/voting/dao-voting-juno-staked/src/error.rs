use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    HookError(#[from] cw_hooks::HookError),

    #[error("Only the DAO may call this method")]
    Unauthorized {},

    #[error("Contract has no execute variants for end users; delegation happens via x/staking")]
    NoExecute {},

    #[error(
        "auto_register_staking_hooks is not yet supported; register out-of-band via x/cw-hooks tx"
    )]
    AutoRegisterNotYetSupported {},

    #[error("Voting power query returned a value larger than Uint128")]
    PowerOverflow {},
}
