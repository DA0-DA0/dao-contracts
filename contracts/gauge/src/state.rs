use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map};

use crate::msg::VoteInfo;

/// Type alias for u64 to make the map types a bit more self-explanatorys
pub type GaugeId = u64;

pub const CONFIG: Item<Config> = Item::new("config");
pub const GAUGES: Map<GaugeId, Gauge> = Map::new("gauges");

/// This lets us find and update any vote given both voter and gauge.
/// It also lets us iterate over all votes by a given voter on all gauges quite easily.
/// This is needed when a voter weight changes in order to update the guage.
/// It makes it very expensive to show all votes on a given gauge, but that is
/// only used for off-chain UI logic anyway, and we should start work on indexers for that.
pub const VOTES: Map<(&Addr, GaugeId), Vote> = Map::new("votes");

#[cw_serde]
pub struct Config {
    /// Address of contract to that contains all voting powers (where we query and listen to hooks)
    pub voting_powers: Addr,
    /// Address that can add new gauges or stop them
    pub owner: Addr,
}

#[cw_serde]
pub struct Gauge {
    /// Address of contract to serve gauge-specific info (AdapterQueryMsg)
    pub adapter: Addr,
    /// Frequency (in seconds) the gauge executes messages, typically something like 7*86400
    pub epoch: u64,
    /// (Optional) Minimum percentage of votes needed by a given option to be in the selected set
    pub min_percent_selected: Option<Decimal>,
    /// (Required) Maximum number of Options to make the selected set. Needed even with
    /// `min_percent_selected` to provide some guarantees on gas usage of this query.
    pub max_options_selected: u32,
    /// True if the gauge is stopped
    pub is_stopped: bool,
    /// UNIX time (seconds) when next epoch can be executed. If < env.block.time then Execute can be called
    pub next_epoch: u64,
}

#[cw_serde]
pub struct Vote {
    /// Option voted for.
    pub option: String,
    /// The voting power behind the vote.
    pub power: Uint128,
}

/// This can be used on range queries over the votes
pub fn to_vote_info((voter, vote): (Addr, Vote)) -> VoteInfo {
    VoteInfo {
        voter: voter.into(),
        option: vote.option,
        power: vote.power,
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

    // update total count
    let total = TOTAL_CAST.may_load(storage, gauge)?.unwrap_or_default();
    let total = total + new_vote - old_vote;
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
}
