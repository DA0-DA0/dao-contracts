use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use voting::deposit::UncheckedDepositInfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg<InstantiateExt> {
    /// Information about the deposit requirements for this
    /// module. None if no deposit.
    pub deposit_info: Option<UncheckedDepositInfo>,
    /// If false, only members (addresses with voting power) may create
    /// proposals in the DAO. Otherwise, any address may create a
    /// proposal so long as they pay the deposit.
    pub open_proposal_submission: bool,
    /// Extension for instantiation. The default implementation will
    /// do nothing with this data.
    pub ext: InstantiateExt,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg<ProposalMessage, ExecuteExt> {
    /// Creates a new proposal in the pre-propose module. MSG will be
    /// serialized and used as the proposal creation message.
    Propose { msg: ProposalMessage },

    /// Extension message. Contracts that extend this one should put
    /// their custom execute logic here. The default implementation
    /// will do nothing if this variant is executed.
    Ext { msg: ExecuteExt },

    /// Handles proposal hooks fired by the associated proposal
    /// module. By default, the base contract will return deposits
    /// when proposals are executed, or, if it is refunding failed
    /// proposals, when they are closed.
    ProposalHook(proposal_hooks::ProposalHookMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg<QueryExt> {
    /// Gets the proposal module that this pre propose module is
    /// associated with. Returns `Addr`.
    ProposalModule {},
    /// Gets the DAO (cw-dao-core) module this contract is associated
    /// with. Returns `Addr`.
    Dao {},
    /// Gets the module's configuration. Returns `state::Config`.
    Config {},
    /// Extension for queries. The default implementation will do
    /// nothing if queried for this and will return
    /// `Binary::default()`.
    Ext { msg: QueryExt },
}
