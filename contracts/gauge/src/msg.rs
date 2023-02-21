use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{CosmosMsg, Decimal, Uint128};

use crate::state::Vote;
use wynd_stake::hook::MemberChangedHookMsg;

type GaugeId = u64;

#[cw_serde]
pub struct InstantiateMsg {
    /// Address of contract to that contains all voting powers (where we query and listen to hooks)
    pub voting_powers: String,
    /// Address that can add new gauges or stop them
    pub owner: String,
    /// Allow attaching multiple adaptors during instantiation.
    /// Important, as instantiation and CreateGauge both come from DAO proposals
    /// and without this argument, you need 2 cycles to create and configure a gauge
    pub gauges: Option<Vec<GaugeConfig>>,
}

#[cw_serde]
pub struct GaugeConfig {
    /// Name of the gauge (for UI)
    pub title: String,
    /// Address of contract to serve gauge-specific info (AdapterQueryMsg)
    pub adapter: String,
    /// Frequency (in seconds) the gauge executes messages, typically something like 7*86400
    pub epoch_size: u64,
    /// Minimum percentage of votes needed by a given option to be in the selected set.
    /// If unset, there is no minimum percentage, just the `max_options_selected` limit.
    pub min_percent_selected: Option<Decimal>,
    /// Maximum number of Options to make the selected set. Needed even with
    /// `min_percent_selected` to provide some guarantees on gas usage of this query.
    pub max_options_selected: u32,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Must be compatible with MemberChangedExecuteMsg from wynd-stake.
    /// Use this to update
    MemberChangedHook(MemberChangedHookMsg),
    /// This creates a new Gauge, returns CreateGaugeReply JSON-encoded in the data field.
    /// Can only be called by owner
    CreateGauge(GaugeConfig),
    /// Allows owner to update certain parameters of GaugeConfig.
    /// If you want to change next_epoch value, you need to use migration.
    UpdateGauge {
        gauge_id: u64,
        epoch_size: Option<u64>,
        // Some<0> would set min_percent_selected to None
        min_percent_selected: Option<Decimal>,
        max_options_selected: Option<u32>,
    },
    /// Stops a given gauge, meaning it will not execute any more messages,
    /// Or receive any more updates on MemberChangedHook.
    /// Ideally, this will allow for eventual deletion of all data on that gauge
    StopGauge { gauge: u64 },
    // WISH: make this implicit - call it inside PlaceVote.
    // If not, I would just make it invisible to user in UI (smart client adds it if needed)
    /// Try to add an option. Error if no such gauge, or option already registered.
    /// Otherwise check adapter and error if invalid.
    /// Can be called by anyone, not just owner
    AddOption { gauge: u64, option: String },
    /// Place your vote on the gauge. Can be updated anytime
    PlaceVotes {
        /// Gauge to vote on
        gauge: u64,
        /// The options to put my vote on, along with proper weights (must sum up to 1.0)
        /// "None" means remove existing votes and abstain
        votes: Option<Vec<Vote>>,
    },
    /// Takes a sample of the current tally and execute the proper messages to make it work
    Execute { gauge: u64 },
}

#[cw_serde]
pub struct CreateGaugeReply {
    /// Id of the gauge that was just created
    pub id: u64,
}

/// Queries the gauge exposes
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(cw_core_interface::voting::InfoResponse)]
    Info {},
    #[returns(GaugeResponse)]
    Gauge { id: u64 },
    #[returns(ListGaugesResponse)]
    ListGauges {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    #[returns(VoteResponse)]
    Vote { gauge: u64, voter: String },
    #[returns(ListVotesResponse)]
    ListVotes {
        gauge: u64,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(ListOptionsResponse)]
    ListOptions {
        gauge: u64,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(SelectedSetResponse)]
    SelectedSet { gauge: u64 },
    #[returns(LastExecutedSetResponse)]
    LastExecutedSet { gauge: u64 },
}

/// Information about one gauge
#[cw_serde]
pub struct GaugeResponse {
    pub id: u64,
    /// Name of the gauge (for UI)
    pub title: String,
    /// Address of contract to serve gauge-specific info (AdapterQueryMsg)
    pub adapter: String,
    /// Frequency (in seconds) the gauge executes messages, typically something like 7*86400
    pub epoch_size: u64,
    /// Minimum percentage of votes needed by a given option to be in the selected set.
    /// If unset, there is no minimum percentage, just the `max_options_selected` limit.
    pub min_percent_selected: Option<Decimal>,
    /// Maximum number of Options to make the selected set. Needed even with
    /// `min_percent_selected` to provide some guarantees on gas usage of this query.
    pub max_options_selected: u32,
    /// True if the gauge is stopped
    pub is_stopped: bool,
    /// UNIX time (seconds) when next epoch may be executed. May be future or past
    pub next_epoch: u64,
}

/// Information about one gauge
#[cw_serde]
pub struct ListGaugesResponse {
    pub gauges: Vec<GaugeResponse>,
}

/// Information about a vote that was cast.
#[cw_serde]
pub struct VoteInfo {
    /// The address that voted.
    pub voter: String,
    /// List of all votes with power
    pub votes: Vec<Vote>,
}

/// Information about a vote.
#[cw_serde]
pub struct VoteResponse {
    /// None if no such vote, Some otherwise.
    pub vote: Option<VoteInfo>,
}

/// Information about all votes on the gauge
#[cw_serde]
pub struct ListVotesResponse {
    pub votes: Vec<VoteInfo>,
}

/// List all available options ordered by the option string.
/// Also returns the current voting power assigned to that option.
/// You will need to paginate to collect them all.
#[cw_serde]
pub struct ListOptionsResponse {
    pub options: Vec<(String, Uint128)>,
}

/// List the options that were selected in the last executed set.
#[cw_serde]
pub struct LastExecutedSetResponse {
    /// `None` if no vote has been executed yet
    pub votes: Option<Vec<(String, Uint128)>>,
}

/// List the top options by power that would make it into the selected set.
/// Ordered from highest votes to lowest
#[cw_serde]
pub struct SelectedSetResponse {
    pub votes: Vec<(String, Uint128)>,
}

/// Queries the gauge requires from the adapter contract in order to function
#[cw_serde]
#[derive(QueryResponses)]
pub enum AdapterQueryMsg {
    #[returns(AllOptionsResponse)]
    AllOptions {},
    #[returns(CheckOptionResponse)]
    CheckOption { option: String },
    #[returns(SampleGaugeMsgsResponse)]
    SampleGaugeMsgs {
        /// option along with weight
        /// sum of all weights should be 1.0 (within rounding error)
        selected: Vec<(String, Decimal)>,
    },
}

#[cw_serde]
pub struct AllOptionsResponse {
    pub options: Vec<String>,
}

#[cw_serde]
pub struct CheckOptionResponse {
    pub valid: bool,
}

#[cw_serde]
pub struct SampleGaugeMsgsResponse {
    // NOTE: I think we will never need CustomMsg here, any reason we should include??
    pub execute: Vec<CosmosMsg>,
}

#[cw_serde]
pub struct MigrateMsg {
    pub next_epochs: Option<Vec<(GaugeId, u64)>>,
}
