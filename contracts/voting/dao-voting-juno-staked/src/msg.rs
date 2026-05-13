use cosmwasm_schema::{cw_serde, QueryResponses};
use dao_dao_macros::voting_module_query;

#[cw_serde]
pub struct InstantiateMsg {
    /// Reserved for future single-tx registration with `x/cw-hooks`.
    /// **Currently must be `None` or `Some(false)`** — passing
    /// `Some(true)` fails with `AutoRegisterNotYetSupported`.
    ///
    /// Production deploys register the instantiated contract address with
    /// `x/cw-hooks` out-of-band: `junod tx cw-hooks register-staking
    /// <contract_addr>`. Field is kept so the in-contract path can be
    /// added later without an API break.
    pub auto_register_staking_hooks: Option<bool>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Adds a subscriber that will receive
    /// `dao_hooks::stake::StakeChangedHookMsg::{Stake, Unstake}` execute
    /// messages whenever the chain reports a delegation change. Only the
    /// DAO may call this.
    AddHook { addr: String },
    /// Removes a previously-registered subscriber. Only the DAO may call
    /// this.
    RemoveHook { addr: String },
}

/// Messages routed via `x/cw-hooks` when a staking event lands.
///
/// Variant names and field names mirror the JSON emitted by Juno's
/// `x/cw-hooks` `staking_hook_types.go` exactly — anything else would
/// mean we never receive the sudo at all.
#[cw_serde]
pub enum SudoMsg {
    AfterDelegationModified {
        after_delegation_modified: DelegationEvent,
    },
    BeforeDelegationCreated {
        before_delegation_created: DelegationEvent,
    },
    BeforeDelegationSharesModified {
        before_delegation_shares_modified: DelegationEvent,
    },
    BeforeDelegationRemoved {
        before_delegation_removed: DelegationEvent,
    },
    BeforeValidatorSlashed {
        before_validator_slashed: ValidatorSlashEvent,
    },
    // Validator-lifecycle hooks fire but do not change any single
    // delegator's bonded amount in a way the snapshot module wouldn't
    // already have written through delegation events; we silently
    // ignore them so cw-hooks doesn't drop us from the registry.
    AfterValidatorCreated {
        after_validator_created: ValidatorEvent,
    },
    AfterValidatorRemoved {
        after_validator_removed: ValidatorEvent,
    },
    BeforeValidatorModified {
        before_validator_modified: ValidatorEvent,
    },
    AfterValidatorModified {
        after_validator_modified: ValidatorEvent,
    },
    AfterValidatorBonded {
        after_validator_bonded: ValidatorEvent,
    },
    AfterValidatorBeginUnbonding {
        after_validator_begin_unbonding: ValidatorEvent,
    },
}

#[cw_serde]
pub struct DelegationEvent {
    pub delegator_address: String,
    pub validator_address: String,
    pub shares: String,
}

#[cw_serde]
pub struct ValidatorEvent {
    pub moniker: String,
    pub validator_address: String,
    pub commission: String,
    pub validator_tokens: String,
    pub bonded_tokens: String,
    pub bond_status: String,
}

#[cw_serde]
pub struct ValidatorSlashEvent {
    pub moniker: String,
    pub validator_address: String,
    pub slashed_amount: String,
}

#[voting_module_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the currently-registered stake-change hook subscribers.
    #[returns(GetHooksResponse)]
    GetHooks {},
}

#[cw_serde]
pub struct GetHooksResponse {
    pub hooks: Vec<String>,
}

#[cw_serde]
pub struct MigrateMsg {}
