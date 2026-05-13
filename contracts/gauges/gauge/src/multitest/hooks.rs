//! Tests for the GaugeVoteHook surface: registration auth, hook firing on
//! PlaceVotes, payload contents, and auto-unregister on hook failure.

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Decimal, Deps, DepsMut, Empty, Env, MessageInfo, Response,
    StdError, StdResult,
};
use cw_multi_test::{Contract, ContractWrapper, Executor};
use cw_storage_plus::Item;

use crate::hooks::{GaugeVoteHookExecuteMsg, GaugeVoteHookMsg};
use crate::msg::{ExecuteMsg, GetHooksResponse, QueryMsg};
use crate::multitest::suite::SuiteBuilder;
use crate::state::Vote;
use crate::ContractError;

// ------------------------------------------------------ Recorder mock contract

const LAST_HOOK: Item<GaugeVoteHookMsg> = Item::new("last_hook");

#[cw_serde]
pub enum RecorderQueryMsg {
    Last {},
}

#[cw_serde]
pub struct LastHookResponse {
    pub hook: Option<GaugeVoteHookMsg>,
}

fn recorder_instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
    Ok(Response::new())
}

fn recorder_execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: GaugeVoteHookExecuteMsg,
) -> StdResult<Response> {
    let GaugeVoteHookExecuteMsg::GaugeVoteHook(payload) = msg;
    LAST_HOOK.save(deps.storage, &payload)?;
    Ok(Response::new())
}

fn recorder_query(deps: Deps, _env: Env, msg: RecorderQueryMsg) -> StdResult<Binary> {
    match msg {
        RecorderQueryMsg::Last {} => to_json_binary(&LastHookResponse {
            hook: LAST_HOOK.may_load(deps.storage)?,
        }),
    }
}

fn recorder_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new_with_empty(
        recorder_execute,
        recorder_instantiate,
        recorder_query,
    ))
}

// ------------------------------------------------------- Failing mock contract

fn failing_instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
    Ok(Response::new())
}

fn failing_execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: GaugeVoteHookExecuteMsg,
) -> StdResult<Response> {
    Err(StdError::generic_err("hook subscriber refused the call"))
}

fn failing_query(_deps: Deps, _env: Env, _msg: Empty) -> StdResult<Binary> {
    Err(StdError::generic_err("no queries"))
}

fn failing_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new_with_empty(
        failing_execute,
        failing_instantiate,
        failing_query,
    ))
}

// --------------------------------------------------------- Helpers shared

fn setup_gauge_and_voter() -> (
    crate::multitest::suite::Suite,
    Addr,   // gauge contract
    String, // voter
) {
    let voter1 = "voter1";
    let voter2 = "voter2";
    let mut suite = SuiteBuilder::new()
        .with_voting_members(&[(voter1, 100), (voter2, 200)])
        .build();

    suite.next_block();
    suite
        .propose_update_proposal_module(voter1.to_string(), None)
        .unwrap();
    suite.next_block();
    let proposal = suite.list_proposals().unwrap()[0];
    suite
        .place_vote_single(voter1, proposal, dao_voting::voting::Vote::Yes)
        .unwrap();
    suite
        .place_vote_single(voter2, proposal, dao_voting::voting::Vote::Yes)
        .unwrap();
    suite.next_block();
    suite
        .execute_single_proposal(voter1.to_string(), proposal)
        .unwrap();

    let proposal_modules = suite.query_proposal_modules().unwrap();
    let gauge_contract = proposal_modules[1].clone();
    suite
        .instantiate_adapter_and_create_gauge(
            gauge_contract.clone(),
            &[voter1, voter2],
            (1000, "ujuno"),
            None,
            None,
        )
        .unwrap();

    (suite, gauge_contract, voter1.to_string())
}

fn store_recorder(suite: &mut crate::multitest::suite::Suite) -> u64 {
    suite.app.store_code(recorder_contract())
}

fn store_failing(suite: &mut crate::multitest::suite::Suite) -> u64 {
    suite.app.store_code(failing_contract())
}

fn instantiate_recorder(
    suite: &mut crate::multitest::suite::Suite,
    code_id: u64,
    label: &str,
) -> Addr {
    suite
        .app
        .instantiate_contract(
            code_id,
            Addr::unchecked(&suite.owner),
            &Empty {},
            &[],
            label,
            None,
        )
        .unwrap()
}

fn add_hook_as_owner(
    suite: &mut crate::multitest::suite::Suite,
    gauge_contract: &Addr,
    hook: &Addr,
) -> anyhow::Result<cw_multi_test::AppResponse> {
    suite.app.execute_contract(
        Addr::unchecked(suite.owner.clone()),
        gauge_contract.clone(),
        &ExecuteMsg::AddHook {
            addr: hook.to_string(),
        },
        &[],
    )
}

// ---------------------------------------------------------------- Tests

#[test]
fn add_hook_requires_owner() {
    let (mut suite, gauge_contract, voter) = setup_gauge_and_voter();
    let code = store_recorder(&mut suite);
    let recorder = instantiate_recorder(&mut suite, code, "recorder");

    let err = suite
        .app
        .execute_contract(
            Addr::unchecked(voter),
            gauge_contract,
            &ExecuteMsg::AddHook {
                addr: recorder.to_string(),
            },
            &[],
        )
        .unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());
}

#[test]
fn remove_hook_requires_owner() {
    let (mut suite, gauge_contract, voter) = setup_gauge_and_voter();
    let code = store_recorder(&mut suite);
    let recorder = instantiate_recorder(&mut suite, code, "recorder");

    add_hook_as_owner(&mut suite, &gauge_contract, &recorder).unwrap();

    let err = suite
        .app
        .execute_contract(
            Addr::unchecked(voter),
            gauge_contract,
            &ExecuteMsg::RemoveHook {
                addr: recorder.to_string(),
            },
            &[],
        )
        .unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());
}

#[test]
fn hook_fires_on_place_votes_with_expected_payload() {
    let (mut suite, gauge_contract, voter) = setup_gauge_and_voter();
    let code = store_recorder(&mut suite);
    let recorder = instantiate_recorder(&mut suite, code, "recorder");
    add_hook_as_owner(&mut suite, &gauge_contract, &recorder).unwrap();

    // List query reflects the registration.
    let hooks: GetHooksResponse = suite
        .app
        .wrap()
        .query_wasm_smart(&gauge_contract, &QueryMsg::GetHooks {})
        .unwrap();
    assert_eq!(hooks.hooks, vec![recorder.to_string()]);

    // Place a vote — should fire the hook.
    suite
        .place_votes(
            &gauge_contract,
            voter.clone(),
            0,
            Some(vec![(voter.clone(), Decimal::percent(90))]),
        )
        .unwrap();

    let response: LastHookResponse = suite
        .app
        .wrap()
        .query_wasm_smart(&recorder, &RecorderQueryMsg::Last {})
        .unwrap();
    let payload = response.hook.expect("hook payload not recorded");
    let GaugeVoteHookMsg::NewVotes {
        gauge_id,
        voter: hook_voter,
        votes,
        voting_power,
        height,
    } = payload;
    assert_eq!(gauge_id, 0);
    assert_eq!(hook_voter, voter);
    assert_eq!(
        votes,
        vec![Vote {
            option: voter.clone(),
            weight: Decimal::percent(90)
        }]
    );
    // cw4 weight 100 → voting power 100.
    assert_eq!(voting_power.u128(), 100);
    assert!(height > 0);
}

#[test]
fn hook_fires_on_abstain_with_empty_votes() {
    let (mut suite, gauge_contract, voter) = setup_gauge_and_voter();
    let code = store_recorder(&mut suite);
    let recorder = instantiate_recorder(&mut suite, code, "recorder");
    add_hook_as_owner(&mut suite, &gauge_contract, &recorder).unwrap();

    // First cast a real vote so there's something to clear.
    suite
        .place_votes(
            &gauge_contract,
            voter.clone(),
            0,
            Some(vec![(voter.clone(), Decimal::percent(100))]),
        )
        .unwrap();

    // Then abstain (None ≈ clear my votes).
    suite
        .place_votes(&gauge_contract, voter.clone(), 0, None)
        .unwrap();

    let response: LastHookResponse = suite
        .app
        .wrap()
        .query_wasm_smart(&recorder, &RecorderQueryMsg::Last {})
        .unwrap();
    let GaugeVoteHookMsg::NewVotes { votes, .. } = response.hook.unwrap();
    assert!(votes.is_empty(), "abstain should ship an empty votes Vec");
}

#[test]
fn failing_hook_is_auto_unregistered_on_place_votes() {
    let (mut suite, gauge_contract, voter) = setup_gauge_and_voter();
    let recorder_code = store_recorder(&mut suite);
    let failing_code = store_failing(&mut suite);
    let recorder = instantiate_recorder(&mut suite, recorder_code, "recorder");
    let failing = instantiate_recorder(&mut suite, failing_code, "failing");

    // Register the FAILING one first so it has reply ID 0 — failures on
    // index N drop the hook at that index; we want to verify the recorder
    // survives by being at a *different* index.
    add_hook_as_owner(&mut suite, &gauge_contract, &failing).unwrap();
    add_hook_as_owner(&mut suite, &gauge_contract, &recorder).unwrap();

    let hooks: GetHooksResponse = suite
        .app
        .wrap()
        .query_wasm_smart(&gauge_contract, &QueryMsg::GetHooks {})
        .unwrap();
    assert_eq!(hooks.hooks.len(), 2);

    // Place a vote. The failing hook errors → reply runs → it is removed.
    // PlaceVotes itself must still succeed.
    suite
        .place_votes(
            &gauge_contract,
            voter.clone(),
            0,
            Some(vec![(voter.clone(), Decimal::percent(100))]),
        )
        .unwrap();

    // Failing hook is gone, recorder still there.
    let hooks: GetHooksResponse = suite
        .app
        .wrap()
        .query_wasm_smart(&gauge_contract, &QueryMsg::GetHooks {})
        .unwrap();
    assert_eq!(hooks.hooks, vec![recorder.to_string()]);

    // And the recorder did record the vote.
    let response: LastHookResponse = suite
        .app
        .wrap()
        .query_wasm_smart(&recorder, &RecorderQueryMsg::Last {})
        .unwrap();
    assert!(
        response.hook.is_some(),
        "recorder should still observe votes"
    );
}
