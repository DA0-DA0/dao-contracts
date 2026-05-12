use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Ownership(#[from] cw_ownable::OwnershipError),

    #[error("Invalid Cw20")]
    InvalidCw20 {},

    #[error("Invalid Staking Contract")]
    InvalidStakingContract {},

    #[error("Zero eligible rewards")]
    ZeroRewards {},

    #[error("Rewards have already been distributed for this block")]
    RewardsDistributedForBlock {},

    #[error("can not migrate. current version is up to date")]
    AlreadyMigrated {},

    #[error(
        "Direct v1 -> v2.9+ migration is not supported in this binary. Migrate v1 contracts via the v2.4.1 release first, then re-migrate to v2.9+."
    )]
    V1MigrationUnsupported {},
}
