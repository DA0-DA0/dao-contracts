#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Order, QueryRequest,
    Response, StdResult, Uint128, WasmMsg, WasmQuery,
};
use cw2::set_contract_version;
use cw_core_interface::{
    voting::{Query as DaoQuery, VotingPowerAtHeightResponse},
    ExecuteMsg as DaoExecuteMsg,
};
use cw_storage_plus::Bound;
use cw_utils::ensure_from_older_version;
use wynd_stake::hook::MemberDiff;

use crate::error::ContractError;
use crate::msg::{
    AdapterQueryMsg, AllOptionsResponse, CheckOptionResponse, ExecuteMsg, GaugeConfig,
    GaugeResponse, InstantiateMsg, ListGaugesResponse, ListOptionsResponse, ListVotesResponse,
    MigrateMsg, QueryMsg, SampleGaugeMsgsResponse, SelectedSetResponse,
};
use crate::state::{
    fetch_last_id, update_tally, votes, Config, Gauge, GaugeId, CONFIG, GAUGES, OPTION_BY_POINTS,
    TALLY, TOTAL_CAST,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:gauge";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let voting_powers = deps.api.addr_validate(&msg.voting_powers)?;
    let owner = deps.api.addr_validate(&msg.owner)?;
    let config = Config {
        voting_powers,
        owner,
        dao_core: info.sender,
    };
    CONFIG.save(deps.storage, &config)?;

    for gauge in msg.gauges.unwrap_or_default() {
        execute::attach_gauge(deps.branch(), env.clone(), gauge)?;
    }

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("owner", &msg.owner)
        .add_attribute("voting_powers", &msg.voting_powers))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::MemberChangedHook(hook_msg) => {
            execute::member_changed(deps, info.sender, hook_msg.diffs)
        }
        ExecuteMsg::CreateGauge(options) => execute::create_gauge(deps, env, info.sender, options),
        ExecuteMsg::StopGauge { gauge } => execute::stop_gauge(deps, info.sender, gauge),
        ExecuteMsg::AddOption { gauge, option } => {
            execute::add_option(deps, info.sender, gauge, option, true)
        }
        ExecuteMsg::PlaceVotes { gauge, votes } => {
            execute::place_votes(deps, info.sender, gauge, votes)
        }
        ExecuteMsg::Execute { gauge } => execute::execute(deps, env, gauge),
    }
}

mod execute {
    use super::*;
    use crate::state::{update_tallies, Vote};
    use std::collections::HashMap;

    pub fn member_changed(
        deps: DepsMut,
        sender: Addr,
        diffs: Vec<MemberDiff>,
    ) -> Result<Response, ContractError> {
        // make sure only voting powers contract can activate this endpoint
        if sender != CONFIG.load(deps.storage)?.voting_powers {
            return Err(ContractError::Unauthorized {});
        }

        let mut response = Response::new().add_attribute("action", "member_changed_hook");

        for diff in diffs {
            response = response.add_attribute("member", &diff.key);
            let voter = deps.api.addr_validate(&diff.key)?;

            // for each gauge this user voted on,
            // update the tallies and update the users vote power
            for mut vote in
                votes().query_votes_by_voter(deps.as_ref(), &voter, None, Some(query::MAX_LIMIT))?
            {
                // find change of vote powers
                let old = diff.old.unwrap_or_default();
                let new = diff.new.unwrap_or_default();

                // calculate updates and adjust tallies
                let updates: Vec<_> = vote
                    .votes
                    .iter()
                    .map(|v| {
                        (
                            v.option.as_str(),
                            (old * v.weight).u128(),
                            (new * v.weight).u128(),
                        )
                    })
                    .collect();
                update_tallies(deps.storage, vote.gauge_id, updates)?;

                // store new vote power for this user
                vote.power = new;
                votes().save(deps.storage, &voter, vote.gauge_id, &vote)?;
            }
        }

        Ok(response)
    }

    pub fn create_gauge(
        deps: DepsMut,
        env: Env,
        sender: Addr,
        options: GaugeConfig,
    ) -> Result<Response, ContractError> {
        let config = CONFIG.load(deps.storage)?;
        if sender != config.owner {
            return Err(ContractError::Unauthorized {});
        }

        let adapter = attach_gauge(deps, env, options)?;

        Ok(Response::new()
            .add_attribute("action", "create_gauge")
            .add_attribute("adapter", &adapter))
    }

    pub fn attach_gauge(
        mut deps: DepsMut,
        env: Env,
        GaugeConfig {
            title,
            adapter,
            epoch_size,
            min_percent_selected,
            max_options_selected,
        }: GaugeConfig,
    ) -> Result<Addr, ContractError> {
        let adapter = deps.api.addr_validate(&adapter)?;
        let gauge = Gauge {
            title,
            adapter: adapter.clone(),
            epoch: epoch_size,
            min_percent_selected,
            max_options_selected,
            is_stopped: false,
            next_epoch: env.block.time.seconds() + epoch_size,
        };
        let last_id: GaugeId = fetch_last_id(deps.storage)?;
        GAUGES.save(deps.storage, last_id, &gauge)?;

        // fetch adapter options
        let adapter_options: AllOptionsResponse =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: adapter.to_string(),
                msg: to_binary(&AdapterQueryMsg::AllOptions {})?,
            }))?;
        adapter_options.options.into_iter().try_for_each(|option| {
            execute::add_option(deps.branch(), adapter.clone(), last_id, option, false)?;
            Ok::<_, ContractError>(())
        })?;

        Ok(adapter)
    }

    pub fn stop_gauge(
        deps: DepsMut,
        sender: Addr,
        gauge_id: GaugeId,
    ) -> Result<Response, ContractError> {
        let config = CONFIG.load(deps.storage)?;
        if sender != config.owner {
            return Err(ContractError::Unauthorized {});
        }

        let gauge = GAUGES.load(deps.storage, gauge_id)?;
        let gauge = Gauge {
            is_stopped: true,
            ..gauge
        };
        GAUGES.save(deps.storage, gauge_id, &gauge)?;

        Ok(Response::new()
            .add_attribute("action", "stop_gauge")
            .add_attribute("gauge_id", gauge_id.to_string()))
    }

    pub fn add_option(
        deps: DepsMut,
        sender: Addr,
        gauge_id: GaugeId,
        option: String,
        // must be true if option is added by execute message
        check_option: bool,
    ) -> Result<Response, ContractError> {
        // check is such option already exists
        if TALLY.has(deps.as_ref().storage, (gauge_id, &option)) {
            return Err(ContractError::OptionAlreadyExists { option, gauge_id });
        };

        // only options added from gauge creation level should not be validated and can
        // have 0 points as assigned voting power.
        if check_option {
            let gauge = GAUGES.load(deps.storage, gauge_id)?;
            // query gauge adapter if it is valid
            let adapter_option: CheckOptionResponse = deps
                .querier
                .query_wasm_smart(
                    gauge.adapter,
                    &AdapterQueryMsg::CheckOption {
                        option: option.clone(),
                    },
                )
                .map_err(|_| ContractError::OptionInvalidByAdapter {
                    option: option.clone(),
                    gauge_id,
                })?;
            if !adapter_option.valid {
                return Err(ContractError::OptionInvalidByAdapter { option, gauge_id });
            }
            // If it is a user adding option, query him for voting power in order to prevent
            // spam from nonvoting users
            let voting_power = deps
                .querier
                .query::<VotingPowerAtHeightResponse>(&QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: CONFIG.load(deps.storage)?.voting_powers.to_string(),
                    msg: to_binary(&DaoQuery::VotingPowerAtHeight {
                        address: sender.to_string(),
                        height: None,
                    })?,
                }))?
                .power;
            if voting_power.is_zero() {
                return Err(ContractError::NoVotingPower(sender.to_string()));
            }
        }

        update_tally(deps.storage, gauge_id, &option, 0u128, 0u128)?;

        Ok(Response::new()
            .add_attribute("action", "add_option")
            .add_attribute("sender", &sender)
            .add_attribute("gauge_id", gauge_id.to_string())
            .add_attribute("option", option))
    }

    pub fn place_votes(
        deps: DepsMut,
        sender: Addr,
        gauge_id: GaugeId,
        new_votes: Option<Vec<Vote>>,
    ) -> Result<Response, ContractError> {
        if !GAUGES.has(deps.storage, gauge_id) {
            return Err(ContractError::GaugeMissing(gauge_id));
        }

        // make sure sums work out
        let new_votes = new_votes.unwrap_or_default();
        let total_weight = new_votes.iter().map(|v| v.weight).sum();
        if total_weight > Decimal::one() {
            return Err(ContractError::TooMuchVotingWeight(total_weight));
        }

        // load voter power from voting powers contract (DAO)
        let voting_power = deps
            .querier
            .query::<VotingPowerAtHeightResponse>(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: CONFIG.load(deps.storage)?.voting_powers.to_string(),
                msg: to_binary(&DaoQuery::VotingPowerAtHeight {
                    address: sender.to_string(),
                    height: None,
                })?,
            }))?
            .power;
        if voting_power.is_zero() {
            return Err(ContractError::NoVotingPower(sender.to_string()));
        }

        let previous_vote = votes().may_load(deps.storage, &sender, gauge_id)?;
        if previous_vote.is_none() && new_votes.is_empty() {
            return Err(ContractError::CannotRemoveNonexistingVote {});
        }

        // first, calculate a diff between new_vote and previous_vote (option -> (old, new))
        let previous_vote = previous_vote.unwrap_or_default();
        let power = previous_vote.power;
        let mut diff: HashMap<&str, (u128, u128)> = previous_vote
            .votes
            .iter()
            .map(|v| (v.option.as_str(), ((power * v.weight).u128(), 0u128)))
            .collect();
        for v in new_votes.iter() {
            let new = (voting_power * v.weight).u128();
            let add = match diff.remove(v.option.as_str()) {
                Some((old, _)) => (old, new),
                None => (0, new),
            };
            diff.insert(&v.option, add);
        }

        // second, test any new options are valid,
        // only for those voted for first time (others have already been checked)
        for new_opt in diff
            .iter()
            .filter(|(_, (old, _))| *old == 0)
            .map(|(&k, _)| k)
        {
            if !TALLY.has(deps.storage, (gauge_id, new_opt)) {
                return Err(ContractError::OptionDoesNotExists {
                    option: new_opt.to_string(),
                    gauge_id,
                });
            }
        }

        // third, update tally based on diff
        let updates: Vec<(&str, u128, u128)> = diff
            .iter()
            .map(|(&k, (old, new))| (k, *old, *new))
            .collect();
        update_tallies(deps.storage, gauge_id, updates)?;

        // finally, update the votes for this user
        if new_votes.is_empty() {
            // completely remove sender's votes
            votes().remove_votes(deps.storage, &sender, gauge_id)?;
        } else {
            // store sender's new votes (overwriting old votes)
            votes().set_votes(deps.storage, &sender, gauge_id, new_votes, voting_power)?;
        }

        let response = Response::new()
            .add_attribute("action", "place_vote")
            .add_attribute("sender", &sender)
            .add_attribute("gauge_id", gauge_id.to_string());
        Ok(response)
    }

    pub fn execute(deps: DepsMut, env: Env, gauge_id: u64) -> Result<Response, ContractError> {
        let gauge = GAUGES.load(deps.storage, gauge_id)?;

        if gauge.is_stopped {
            return Err(ContractError::GaugeStopped(gauge_id));
        }

        let current_epoch = env.block.time.seconds();
        if current_epoch < gauge.next_epoch {
            return Err(ContractError::EpochNotReached {
                gauge_id,
                current_epoch,
                next_epoch: gauge.next_epoch,
            });
        }

        // this set contains tuple (option, total_voted_power)
        // for adapter query, this needs to be transformed into (option, voted_weight)
        let selected_set_with_powers = query::selected_set(deps.as_ref(), gauge_id)?.votes;

        // This is a bit hacky solution to accomplish 3 things:
        // - remove executed option from TALLY
        // - remove executed option from OPTION_BY_POINTS
        // - summarizing all power to be subtracted from TOTAL_CAST
        // Placed here to avoid copy of either whole iterator or its elements in second loop
        // down below.
        let selected_powers_sum = selected_set_with_powers
            .iter()
            .map(|(option, power)| {
                let power = power.u128();
                TALLY.remove(deps.storage, (gauge_id, option));
                OPTION_BY_POINTS.remove(deps.storage, (gauge_id, power, option));
                power
            })
            .sum::<u128>();

        // calculate "local" ratios of voted options per total power of all selected options
        let selected = selected_set_with_powers
            .into_iter()
            .map(|(option, power)| Ok((option, Decimal::from_ratio(power, selected_powers_sum))))
            .collect::<StdResult<Vec<(String, Decimal)>>>()?;

        // query gauge adapter for execute messages for DAO
        let execute_messages: SampleGaugeMsgsResponse = deps.querier.query_wasm_smart(
            gauge.adapter,
            &AdapterQueryMsg::SampleGaugeMsgs { selected },
        )?;

        let config = CONFIG.load(deps.storage)?;
        let execute_msg = WasmMsg::Execute {
            contract_addr: config.dao_core.to_string(),
            msg: to_binary(&DaoExecuteMsg::ExecuteProposalHook {
                msgs: execute_messages.execute,
            })?,
            funds: vec![],
        };

        // update total cast to reflect executed messages
        TOTAL_CAST.update(deps.storage, gauge_id, |total_cast| -> StdResult<_> {
            Ok(total_cast.unwrap_or_default() - selected_powers_sum)
        })?;

        Ok(Response::new()
            .add_attribute("action", "execute_tally")
            .add_message(execute_msg))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Info {} => Ok(to_binary(&query::info(deps)?)?),
        QueryMsg::Gauge { id } => Ok(to_binary(&query::gauge(deps, id)?)?),
        QueryMsg::ListGauges { start_after, limit } => {
            Ok(to_binary(&query::list_gauges(deps, start_after, limit)?)?)
        }
        QueryMsg::Vote { gauge, voter } => Ok(to_binary(&query::vote(deps, gauge, voter)?)?),
        QueryMsg::ListVotes {
            gauge,
            start_after,
            limit,
        } => Ok(to_binary(&query::list_votes(
            deps,
            gauge,
            start_after,
            limit,
        )?)?),
        QueryMsg::ListOptions {
            gauge,
            start_after,
            limit,
        } => Ok(to_binary(&query::list_options(
            deps,
            gauge,
            start_after,
            limit,
        )?)?),
        QueryMsg::SelectedSet { gauge } => Ok(to_binary(&query::selected_set(deps, gauge)?)?),
    }
}

mod query {
    use super::*;

    use crate::msg::{VoteInfo, VoteResponse};
    use cw_core_interface::voting::InfoResponse;

    pub fn info(deps: Deps) -> StdResult<InfoResponse> {
        let info = cw2::get_contract_version(deps.storage)?;
        Ok(InfoResponse { info })
    }

    fn to_gauge_response(gauge_id: GaugeId, gauge: Gauge) -> GaugeResponse {
        GaugeResponse {
            id: gauge_id,
            title: gauge.title,
            adapter: gauge.adapter.to_string(),
            epoch_size: gauge.epoch,
            min_percent_selected: gauge.min_percent_selected,
            max_options_selected: gauge.max_options_selected,
            is_stopped: gauge.is_stopped,
            next_epoch: gauge.next_epoch,
        }
    }

    pub fn gauge(deps: Deps, gauge_id: GaugeId) -> StdResult<GaugeResponse> {
        let gauge = GAUGES.load(deps.storage, gauge_id)?;
        Ok(to_gauge_response(gauge_id, gauge))
    }

    // settings for pagination
    pub const MAX_LIMIT: u32 = 100;
    pub const DEFAULT_LIMIT: u32 = 30;

    pub fn list_gauges(
        deps: Deps,
        start_after: Option<u64>,
        limit: Option<u32>,
    ) -> StdResult<ListGaugesResponse> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start = start_after.map(Bound::exclusive);

        Ok(ListGaugesResponse {
            gauges: GAUGES
                .range(deps.storage, start, None, Order::Ascending)
                .map(|item| {
                    let (id, gauge) = item?;
                    Ok(to_gauge_response(id, gauge))
                })
                .take(limit)
                .collect::<StdResult<Vec<GaugeResponse>>>()?,
        })
    }

    pub fn vote(deps: Deps, gauge_id: u64, voter: String) -> StdResult<VoteResponse> {
        let voter_addr = deps.api.addr_validate(&voter)?;
        let vote = votes()
            .may_load(deps.storage, &voter_addr, gauge_id)?
            .map(|v| VoteInfo {
                voter,
                votes: v.votes,
            });
        Ok(VoteResponse { vote })
    }

    pub fn list_votes(
        deps: Deps,
        gauge_id: u64,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<ListVotesResponse> {
        Ok(ListVotesResponse {
            votes: votes().query_votes_by_gauge(deps, gauge_id, start_after, limit)?,
        })
    }

    pub fn list_options(
        deps: Deps,
        gauge_id: u64,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<ListOptionsResponse> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let start_after = start_after.as_ref().map(|s| Bound::exclusive(s.as_str()));

        Ok(ListOptionsResponse {
            options: TALLY
                .prefix(gauge_id)
                .range(deps.storage, start_after, None, Order::Ascending)
                .map(|option| {
                    let (option, power) = option?;
                    Ok((option, Uint128::new(power)))
                })
                .take(limit)
                .collect::<StdResult<Vec<(String, Uint128)>>>()?,
        })
    }

    pub fn selected_set(deps: Deps, gauge_id: u64) -> StdResult<SelectedSetResponse> {
        let gauge = GAUGES.load(deps.storage, gauge_id)?;
        let total_cast = TOTAL_CAST.load(deps.storage, gauge_id)?;

        // This is sorted index, but requires manual filtering - cannot be prefixed
        // given our requirements
        let votes = OPTION_BY_POINTS
            .sub_prefix(gauge_id)
            .range(deps.storage, None, None, Order::Descending)
            .filter(|o| {
                let ((power, _), _) = o.as_ref().unwrap();
                if let Some(min_percent_selected) = gauge.min_percent_selected {
                    Decimal::from_ratio(*power, total_cast) >= min_percent_selected
                } else {
                    // filter out options without a vote
                    *power != 0u128
                }
            })
            .map(|o| {
                let ((power, option), _) = o?;
                Ok((option, Uint128::new(power)))
            })
            .take(gauge.max_options_selected as usize)
            .collect::<StdResult<Vec<(String, Uint128)>>>()?;

        Ok(SelectedSetResponse { votes })
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new())
}
