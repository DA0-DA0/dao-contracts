use std::marker::PhantomData;

use cosmwasm_std::{Addr, StdResult, Uint128};
use cw_storage_plus::{Item, Map, Prefixer};
use cw_utils::Duration;

use indexable_hooks::Hooks;
use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use voting::{deposit::CheckedDepositInfo, threshold::Threshold, voting::Vote};

use crate::proposal::SingleChoiceProposal;

/// A vote cast for a proposal.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Ballot {
    /// The amount of voting power behind the vote.
    pub power: Uint128,
    /// The position.
    pub vote: Vote,
}
/// The governance module's configuration.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// The threshold a proposal must reach to complete.
    pub threshold: Threshold,
    /// The default maximum amount of time a proposal may be voted on
    /// before expiring.
    pub max_voting_period: Duration,
    /// The minimum amount of time a proposal must be open before
    /// passing. A proposal may fail before this amount of time has
    /// elapsed, but it will not pass. This can be useful for
    /// preventing governance attacks wherein an attacker aquires a
    /// large number of tokens and forces a proposal through.
    pub min_voting_period: Option<Duration>,
    /// If set to true only members may execute passed
    /// proposals. Otherwise, any address may execute a passed
    /// proposal.
    pub only_members_execute: bool,
    /// Allows changing votes before the proposal expires. If this is
    /// enabled proposals will not be able to complete early as final
    /// vote information is not known until the time of proposal
    /// expiration.
    pub allow_revoting: bool,
    /// The address of the DAO that this governance module is
    /// associated with.
    pub dao: Addr,
    /// Information about the depost required to create a
    /// proposal. None if no deposit is required, Some otherwise.
    pub deposit_info: Option<CheckedDepositInfo>,
    /// If set to true proposals will be closed if their execution
    /// fails. Otherwise, proposals will remain open after execution
    /// failure. For example, with this enabled a proposal to send 5
    /// tokens out of a DAO's treasury with 4 tokens would be closed when
    /// it is executed. With this disabled, that same proposal would
    /// remain open until the DAO's treasury was large enough for it to be
    /// executed.
    pub close_proposal_on_execution_failure: bool,
}

/// The current top level config for the module.
pub const CONFIG: Item<Config> = Item::new("config");
/// The number of proposals that have been created.
pub const PROPOSAL_COUNT: Item<u64> = Item::new("proposal_count");
pub const PROPOSALS: Map<u64, SingleChoiceProposal> = Map::new("proposals_v2");
const BALLOTS_INTERNAL: Map<(u64, Addr), Ballot> = Map::new("ballots");
/// Consumers of proposal state change hooks.
pub const PROPOSAL_HOOKS: Hooks = Hooks::new("proposal_hooks");
/// Consumers of vote hooks.
pub const VOTE_HOOKS: Hooks = Hooks::new("vote_hooks");

pub struct Ballots<K, T> {
    // wallet_voted_on: Map<u64, Ballot>,
    key_type: PhantomData<K>,
    data_type: PhantomData<T>,
}

impl<K, T> Ballots<K, T> {
    pub fn update<A, E>(
        &self,
        store: &mut dyn cosmwasm_std::Storage,
        k: (u64, Addr),
        action: A,
    ) -> Result<Ballot, E>
    where
        A: FnOnce(Option<Ballot>) -> Result<Ballot, E>,
        E: From<cosmwasm_std::StdError>,
    {
        let vote = BALLOTS_INTERNAL.update(store, k.clone(), action)?;
        let wallet_voted_on: Map<u64, Ballot> = Map::new(k.1.as_str());
        wallet_voted_on.save(store, k.0, &vote)?;
        Ok(vote)
    }

    pub fn may_load(
        &self,
        store: &dyn cosmwasm_std::Storage,
        k: (u64, Addr),
    ) -> StdResult<Option<Ballot>> {
        BALLOTS_INTERNAL.may_load(store, k)
    }

    pub fn range_by_wallet<'a>(
        &'a self,
        store: &'a dyn cosmwasm_std::Storage,
        k: &str,
        min: Option<cw_storage_plus::Bound<u64>>,
        max: Option<cw_storage_plus::Bound<u64>>,
        order: cosmwasm_std::Order,
    ) -> Box<dyn Iterator<Item = Result<(u64, Ballot), cosmwasm_std::StdError>> + '_> {
        let wallet_voted_on: Map<u64, Ballot> = Map::new(k);
        wallet_voted_on.range(store, min, max, order)
    }
}

impl<'a, K, T> Ballots<K, T>
where
    T: Serialize + DeserializeOwned,
    K: cw_storage_plus::PrimaryKey<'a>,
{
    pub fn prefix(&self, p: K::Prefix) -> cw_storage_plus::Prefix<K::Suffix, T, K::Suffix> {
        cw_storage_plus::Prefix::new(BALLOTS_INTERNAL.namespace(), &p.prefix())
    }
}

pub const BALLOTS: Ballots<(u64, Addr), Ballot> = Ballots {
    key_type: PhantomData,
    data_type: PhantomData,
};
