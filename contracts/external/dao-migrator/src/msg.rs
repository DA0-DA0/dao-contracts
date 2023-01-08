use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::WasmMsg;

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    /// Detects the current DAO configuration and performs a migration
    /// checking state before and after to smoke test the migration's
    /// success. This module will remove itself on this message's
    /// completion regardless of the migration's success.
    MigrateV1ToV2 {
        /// Rather or not to migrate the stake_cw20 contract and its
        /// manager. If this is not set to true and a stake_cw20
        /// contract is detected in the DAO's configuration the
        /// migration will be aborted.
        migrate_stake_cw20_manager: Option<bool>,
    },
    /// Callable only by this contract.
    ///
    /// In submessage terms, say a message that results in an error
    /// "returns false" and one that succedes "returns true". Returns
    /// the logical conjunction (&&) of all the messages in operands.
    ///
    /// Under the hood this just executes them in order. We use this
    /// to respond with a single ACK when a message calls for the
    /// execution of both `CreateVouchers` and `RedeemVouchers`.
    Conjunction { operands: Vec<WasmMsg> },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}    
