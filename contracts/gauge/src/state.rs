use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Deps, Order, StdResult, Storage, Uint128};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use cw_utils::maybe_addr;

use crate::msg::VoteInfo;

/// Type alias for u64 to make the map types a bit more self-explanatory
pub type GaugeId = u64;

pub const CONFIG: Item<Config> = Item::new("config");
pub const GAUGES: Map<GaugeId, Gauge> = Map::new("gauges");
const LAST_ID: Item<GaugeId> = Item::new("last_id");

/// Get ID for gauge registration and increment value in storage
pub fn fetch_last_id(storage: &mut dyn Storage) -> StdResult<u64> {
    let last_id = LAST_ID.load(storage).unwrap_or_default();
    LAST_ID.save(storage, &(last_id + 1u64))?;
    Ok(last_id)
}

/// This lets us find and update any vote given both voter and gauge.
/// It also lets us iterate over all votes by a given voter on all gauges
/// or by a given gauge id. This is needed when a voter weight changes
/// in order to update the guage.
pub fn votes() -> Votes<'static> {
    Votes::new("votes", "votes__gaugeid")
}

// settings for pagination
const MAX_LIMIT: u32 = 100;
const DEFAULT_LIMIT: u32 = 30;

#[cw_serde]
pub struct Config {
    /// Address of contract to that contains all voting powers (where we query and listen to hooks)
    pub voting_powers: Addr,
    /// Address that can add new gauges or stop them
    pub owner: Addr,
    /// Address of DAO core module resposible for instantiation and execution of messages
    pub dao_core: Addr,
}

#[cw_serde]
pub struct Gauge {
    /// Descriptory label of gauge
    pub title: String,
    /// Address of contract to serve gauge-specific info (AdapterQueryMsg)
    pub adapter: Addr,
    /// Frequency (in seconds) the gauge executes messages, typically something like 7*86400
    pub epoch: u64,
    /// Minimum percentage of votes needed by a given option to be in the selected set
    pub min_percent_selected: Option<Decimal>,
    /// Maximum number of Options to make the selected set. Needed even with
    /// `min_percent_selected` to provide some guarantees on gas usage of this query.
    pub max_options_selected: u32,
    /// True if the gauge is stopped
    pub is_stopped: bool,
    /// UNIX time (seconds) when next epoch can be executed. If < env.block.time then Execute can be called
    pub next_epoch: u64,
    /// The last set of options selected by the gauge, `None` before the first execution
    pub last_executed_set: Option<Vec<(String, Uint128)>>,
}

#[cw_serde]
pub struct WeightedVotes {
    /// The gauge these votes are for
    pub gauge_id: GaugeId,
    /// The voting power behind the vote.
    pub power: Uint128,
    /// the user's votes for this gauge
    pub votes: Vec<Vote>,
}

impl Default for WeightedVotes {
    fn default() -> Self {
        WeightedVotes {
            gauge_id: 0,
            power: Uint128::zero(),
            votes: vec![],
        }
    }
}

#[cw_serde]
pub struct Vote {
    /// Option voted for.
    pub option: String,
    /// The weight of the power given to this vote
    pub weight: Decimal,
}

struct VoteIndexes<'a> {
    // Last type param defines the pk deserialization type
    pub vote: MultiIndex<'a, GaugeId, WeightedVotes, (&'a Addr, GaugeId)>,
}

impl<'a> IndexList<WeightedVotes> for VoteIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<WeightedVotes>> + '_> {
        Box::new(std::iter::once(&self.vote as &dyn Index<WeightedVotes>))
    }
}

pub struct Votes<'a> {
    // Votes are indexed by `(addr, gauge_id, weight)` triplet
    votes: IndexedMap<'a, (&'a Addr, GaugeId), WeightedVotes, VoteIndexes<'a>>,
}

impl<'a> Votes<'a> {
    pub fn new(storage_key: &'a str, vote_subkey: &'a str) -> Self {
        let indexes = VoteIndexes {
            vote: MultiIndex::new(|_, vote| vote.gauge_id, storage_key, vote_subkey),
        };
        let votes = IndexedMap::new(storage_key, indexes);
        Self { votes }
    }

    pub fn save(
        &self,
        storage: &mut dyn Storage,
        voter: &'a Addr,
        gauge_id: GaugeId,
        vote: &WeightedVotes,
    ) -> StdResult<()> {
        self.votes.save(storage, (voter, gauge_id), vote)
    }

    pub fn set_votes(
        &self,
        storage: &mut dyn Storage,
        voter: &'a Addr,
        gauge_id: GaugeId,
        votes: Vec<Vote>,
        power: impl Into<Uint128>,
    ) -> StdResult<()> {
        let power = power.into();
        self.votes.save(
            storage,
            (voter, gauge_id),
            &WeightedVotes {
                gauge_id,
                power,
                votes,
            },
        )
    }

    pub fn remove_votes(
        &self,
        storage: &mut dyn Storage,
        voter: &'a Addr,
        gauge_id: GaugeId,
    ) -> StdResult<()> {
        self.votes.remove(storage, (voter, gauge_id))
    }

    pub fn load(
        &self,
        storage: &dyn Storage,
        voter: &'a Addr,
        gauge_id: GaugeId,
    ) -> StdResult<WeightedVotes> {
        self.votes.load(storage, (voter, gauge_id))
    }

    pub fn may_load(
        &self,
        storage: &dyn Storage,
        voter: &'a Addr,
        gauge_id: GaugeId,
    ) -> StdResult<Option<WeightedVotes>> {
        self.votes.may_load(storage, (voter, gauge_id))
    }

    pub fn query_votes_by_voter(
        &self,
        deps: Deps,
        voter_addr: &'a Addr,
        start_after: Option<GaugeId>,
        limit: Option<u32>,
    ) -> StdResult<Vec<WeightedVotes>> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start = start_after.map(Bound::exclusive);

        self.votes
            .prefix(voter_addr)
            .range(deps.storage, start, None, Order::Ascending)
            .map(|index| {
                let (_, vote) = index?;
                Ok(vote)
            })
            .take(limit)
            .collect()
    }

    pub fn query_votes_by_gauge(
        &self,
        deps: Deps,
        gauge_id: GaugeId,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<Vec<VoteInfo>> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let addr = maybe_addr(deps.api, start_after)?;
        let start = addr.as_ref().map(|a| Bound::exclusive((a, gauge_id)));

        self.votes
            .idx
            .vote
            .prefix(gauge_id)
            .range(deps.storage, start, None, Order::Ascending)
            .take(limit)
            .map(|r| {
                let ((voter, _gauge), votes) = r?;
                Ok(VoteInfo {
                    voter: voter.into_string(),
                    votes: votes.votes,
                })
            })
            // NIT: collect and into_iter is a bit inefficient... guess it was too complex/confusing otherwise, so fine
            .collect()
    }
}

/// Total amount of votes in all options, used to calculate min percentage.
pub const TOTAL_CAST: Map<GaugeId, u128> = Map::new("total_power");

/// Count how many points each option has per gauge
pub const TALLY: Map<(GaugeId, &str), u128> = Map::new("tally");
/// Sorted index of options by points, separated by gauge - data field is a placeholder
pub const OPTION_BY_POINTS: Map<(GaugeId, u128, &str), u8> = Map::new("tally_points");

/// Updates the tally for one option.
/// The first time a user votes, they get `{old_vote: 0, new_vote: power}`
/// If they change options, call old option with `{old_vote: power, new_vote: 0}` and new option with `{old_vote: 0, new_vote: power}`
/// If a user changes power (member update hook), call existing option with `{old_vote: old_power, new_vote: new_power}`
pub fn update_tally(
    storage: &mut dyn Storage,
    gauge: GaugeId,
    option: &str,
    old_vote: u128,
    new_vote: u128,
) -> StdResult<()> {
    update_tallies(storage, gauge, vec![(option, old_vote, new_vote)])
}

/// Updates the tally for one option.
/// The first time a user votes, they get `{old_vote: 0, new_vote: power}`
/// If they change options, call old option with `{old_vote: power, new_vote: 0}` and new option with `{old_vote: 0, new_vote: power}`
/// If a user changes power (member update hook), call existing option with `{old_vote: old_power, new_vote: new_power}`
pub fn update_tallies(
    storage: &mut dyn Storage,
    gauge: GaugeId,
    // (option, old, new)
    updates: Vec<(&str, u128, u128)>,
) -> StdResult<()> {
    let mut old_votes = 0u128;
    let mut new_votes = 0u128;

    for (option, old_vote, new_vote) in updates {
        old_votes += old_vote;
        new_votes += new_vote;

        // get old and new values
        let old_count = TALLY.may_load(storage, (gauge, option))?;
        let count = old_count.unwrap_or_default() + new_vote - old_vote;

        // update main index
        TALLY.save(storage, (gauge, option), &count)?;

        // delete old secondary index (if any)
        if let Some(old) = old_count {
            OPTION_BY_POINTS.remove(storage, (gauge, old, option));
        }
        // add new secondary index
        OPTION_BY_POINTS.save(storage, (gauge, count, option), &1u8)?;
    }

    // update total count
    let total = TOTAL_CAST.may_load(storage, gauge)?.unwrap_or_default();
    let total = total + new_votes - old_votes;
    TOTAL_CAST.save(storage, gauge, &total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::Order;

    use cosmwasm_std::testing::mock_dependencies;

    const GAUGE: u64 = 2;

    /// Let's keep them all the same length for less surprising iteration
    const OPTION1: &str = "one";
    const OPTION2: &str = "two";
    // make sure it is alphabetically last
    const OPTION3: &str = "zzz";

    // demonstate how to call update tally and how to query the tallies,
    // both by pk and by secondary index
    #[test]
    fn update_tally_initial_votes_work() {
        let mut mock_deps = mock_dependencies();
        let deps = mock_deps.as_mut();

        update_tally(deps.storage, GAUGE, OPTION1, 0, 250).unwrap();
        update_tally(deps.storage, GAUGE, OPTION2, 0, 400).unwrap();
        update_tally(deps.storage, GAUGE, OPTION3, 0, 100).unwrap();

        // data in some other tally shouldn't mix with this gauge
        update_tally(deps.storage, 17, OPTION3, 0, 55).unwrap();
        update_tally(deps.storage, 16, OPTION1, 0, 123).unwrap();

        // get all options with primary index (ordered by string value of option)
        let options: Vec<_> = TALLY
            .prefix(GAUGE)
            .range(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        let expected = vec![
            (OPTION1.to_string(), 250u128),
            (OPTION2.to_string(), 400u128),
            (OPTION3.to_string(), 100u128),
        ];
        assert_eq!(options, expected);

        // get them by secondary index, top to bottom
        let options: Vec<_> = OPTION_BY_POINTS
            .sub_prefix(GAUGE)
            .keys(deps.storage, None, None, Order::Descending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        let expected = vec![
            (400u128, OPTION2.to_string()),
            (250u128, OPTION1.to_string()),
            (100u128, OPTION3.to_string()),
        ];
        assert_eq!(options, expected);

        // total is properly set
        let total = TOTAL_CAST.load(deps.storage, GAUGE).unwrap();
        assert_eq!(total, 750u128);
    }

    fn to_vote_info(voter: &Addr, votes: &[Vote]) -> VoteInfo {
        VoteInfo {
            voter: voter.to_string(),
            votes: votes.to_vec(),
        }
    }

    #[test]
    fn votes_works() {
        let mut deps = mock_dependencies();
        let votes = votes();

        let user1 = Addr::unchecked("user1");
        let votes1 = vec![Vote {
            option: "someoption".to_owned(),
            weight: Decimal::percent(100),
        }];
        let vote1 = WeightedVotes {
            gauge_id: 1,
            power: Uint128::new(3),
            votes: votes1.clone(),
        };
        votes
            .votes
            .save(&mut deps.storage, (&user1, 1), &vote1)
            .unwrap();

        let user2 = Addr::unchecked("user2");
        let votes2 = vec![Vote {
            option: "otheroption".to_owned(),
            weight: Decimal::percent(50),
        }];
        let vote2 = WeightedVotes {
            gauge_id: 1,
            power: Uint128::new(6),
            votes: votes2.clone(),
        };
        votes
            .votes
            .save(&mut deps.storage, (&user2, 1), &vote2)
            .unwrap();

        let user3 = Addr::unchecked("user3");
        let votes3 = vec![Vote {
            option: "otheroption".to_owned(),
            weight: Decimal::percent(70),
        }];
        let vote3 = WeightedVotes {
            gauge_id: 1,
            power: Uint128::new(9),
            votes: votes3.clone(),
        };
        votes
            .votes
            .save(&mut deps.storage, (&user3, 1), &vote3)
            .unwrap();

        let votes4 = vec![Vote {
            option: "otheroption".to_owned(),
            weight: Decimal::percent(75),
        }];
        let vote4 = WeightedVotes {
            gauge_id: 2,
            power: Uint128::new(12),
            votes: votes4,
        };
        votes
            .votes
            .save(&mut deps.storage, (&user1, 2), &vote4)
            .unwrap();

        let votes5 = vec![Vote {
            option: "otheroption".to_owned(),
            weight: Decimal::percent(100),
        }];
        let vote5 = WeightedVotes {
            gauge_id: 3,
            power: Uint128::new(15),
            votes: votes5,
        };
        votes
            .votes
            .save(&mut deps.storage, (&user1, 3), &vote5)
            .unwrap();

        let vote_result = votes.votes.load(&deps.storage, (&user2, 1)).unwrap();
        assert_eq!(vote_result, vote2);

        let result = votes
            .query_votes_by_gauge(deps.as_ref(), 1, None, None)
            .unwrap();
        assert_eq!(
            result,
            vec![
                to_vote_info(&user1, &votes1),
                to_vote_info(&user2, &votes2),
                to_vote_info(&user3, &votes3),
            ]
        );

        let result = votes
            .query_votes_by_voter(deps.as_ref(), &user1, None, None)
            .unwrap();
        assert_eq!(result, vec![vote1, vote4, vote5]);
    }

    #[test]
    fn query_votes_by_gauge_paginated() {
        let mut deps = mock_dependencies();
        let votes = votes();

        let gauge_id = 1;

        let user1 = Addr::unchecked("user1");
        let votes1 = vec![Vote {
            option: "someoption".to_owned(),
            weight: Decimal::percent(100),
        }];
        let vote1 = WeightedVotes {
            gauge_id: 1,
            power: Uint128::new(3),
            votes: votes1.clone(),
        };
        votes
            .votes
            .save(&mut deps.storage, (&user1, 1), &vote1)
            .unwrap();

        let user2 = Addr::unchecked("user2");
        let votes2 = vec![Vote {
            option: "otheroption".to_owned(),
            weight: Decimal::percent(50),
        }];
        let vote2 = WeightedVotes {
            gauge_id: 1,
            power: Uint128::new(6),
            votes: votes2.clone(),
        };
        votes
            .votes
            .save(&mut deps.storage, (&user2, 1), &vote2)
            .unwrap();

        let user3 = Addr::unchecked("user3");
        let votes3 = vec![Vote {
            option: "otheroption".to_owned(),
            weight: Decimal::percent(70),
        }];
        let vote3 = WeightedVotes {
            gauge_id: 1,
            power: Uint128::new(9),
            votes: votes3.clone(),
        };
        votes
            .votes
            .save(&mut deps.storage, (&user3, 1), &vote3)
            .unwrap();

        // limit to 2 results
        let result = votes
            .query_votes_by_gauge(deps.as_ref(), gauge_id, None, Some(2))
            .unwrap();
        assert_eq!(
            result,
            vec![to_vote_info(&user1, &votes1), to_vote_info(&user2, &votes2)]
        );

        // start from second user (start_after user1)
        let result = votes
            .query_votes_by_gauge(deps.as_ref(), gauge_id, Some(user1.to_string()), None)
            .unwrap();
        assert_eq!(
            result,
            vec![to_vote_info(&user2, &votes2), to_vote_info(&user3, &votes3)]
        );
    }

    #[test]
    fn query_votes_by_user_paginated() {
        let mut deps = mock_dependencies();
        let votes = votes();
        let user1 = Addr::unchecked("user1");

        let vote1 = WeightedVotes {
            gauge_id: 2,
            power: Uint128::new(3),
            votes: vec![Vote {
                option: "someoption".to_owned(),
                weight: Decimal::percent(100),
            }],
        };
        votes
            .votes
            .save(&mut deps.storage, (&user1, 2), &vote1)
            .unwrap();

        let vote2 = WeightedVotes {
            gauge_id: 3,
            power: Uint128::new(6),
            votes: vec![Vote {
                option: "otheroption".to_owned(),
                weight: Decimal::percent(100),
            }],
        };
        votes
            .votes
            .save(&mut deps.storage, (&user1, 3), &vote2)
            .unwrap();

        let vote3 = WeightedVotes {
            gauge_id: 4,
            power: Uint128::new(9),
            votes: vec![Vote {
                option: "otheroption".to_owned(),
                weight: Decimal::percent(100),
            }],
        };
        votes
            .votes
            .save(&mut deps.storage, (&user1, 4), &vote3)
            .unwrap();

        // limit to 2 results
        let result = votes
            .query_votes_by_voter(deps.as_ref(), &user1, None, Some(2))
            .unwrap();
        assert_eq!(result, vec![vote1, vote2.clone()]);

        // start from second user (start_after gauge_id 2)
        let result = votes
            .query_votes_by_voter(deps.as_ref(), &user1, Some(2), None)
            .unwrap();
        assert_eq!(result, vec![vote2, vote3]);
    }
}
