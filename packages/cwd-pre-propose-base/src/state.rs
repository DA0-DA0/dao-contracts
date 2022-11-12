use std::marker::PhantomData;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

use cwd_voting::deposit::CheckedDepositInfo;

#[cw_serde]
pub struct Config {
    /// Information about the deposit required to create a
    /// proposal. If `None`, no deposit is required.
    pub deposit_info: Option<CheckedDepositInfo>,
    /// If false, only members (addresses with voting power) may create
    /// proposals in the DAO. Otherwise, any address may create a
    /// proposal so long as they pay the deposit.
    pub open_proposal_submission: bool,
}

pub struct PreProposeContract<InstantiateExt, ExecuteExt, QueryExt, ProposalMessage> {
    /// The proposal module that this module is associated with.
    pub proposal_module: Item<'static, Addr>,
    /// The DAO (cw-dao-core module) that this module is associated
    /// with.
    pub dao: Item<'static, Addr>,
    /// The configuration for this module.
    pub config: Item<'static, Config>,
    /// Map between proposal IDs and (deposit, proposer) pairs.
    pub deposits: Map<'static, u64, (Option<CheckedDepositInfo>, Addr)>,

    // These types are used in associated functions, but not
    // assocaited data. To stop the compiler complaining about unused
    // generics, we build this phantom data.
    instantiate_type: PhantomData<InstantiateExt>,
    execute_type: PhantomData<ExecuteExt>,
    query_type: PhantomData<QueryExt>,
    proposal_type: PhantomData<ProposalMessage>,
}

impl<InstantiateExt, ExecuteExt, QueryExt, ProposalMessage>
    PreProposeContract<InstantiateExt, ExecuteExt, QueryExt, ProposalMessage>
{
    const fn new(
        proposal_key: &'static str,
        dao_key: &'static str,
        config_key: &'static str,
        deposits_key: &'static str,
    ) -> Self {
        Self {
            proposal_module: Item::new(proposal_key),
            dao: Item::new(dao_key),
            config: Item::new(config_key),
            deposits: Map::new(deposits_key),
            execute_type: PhantomData,
            instantiate_type: PhantomData,
            query_type: PhantomData,
            proposal_type: PhantomData,
        }
    }
}

impl<InstantiateExt, ExecuteExt, QueryExt, ProposalMessage> Default
    for PreProposeContract<InstantiateExt, ExecuteExt, QueryExt, ProposalMessage>
{
    fn default() -> Self {
        // Call into constant function here. Presumably, the compiler
        // is clever enough to inline this. This gives us
        // "more-or-less" constant evaluation for our default method.
        Self::new("proposal_module", "dao", "config", "deposits")
    }
}
