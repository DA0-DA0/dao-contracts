//! cw-multi-test integration tests for dao-proposal-wavs.
//!
//! Envelopes follow the canonical Solidity-ABI shape that `cw-middleware` v0.3.0
//! expects: `Envelope { bytes20 eventId; bytes12 ordering; bytes payload }` head-tail
//! encoded as raw bytes wrapped by `WavsEnvelope(Binary)`. Signatures travel as hex
//! in `WavsSignatureData`.
//!
//! Coverage:
//!   1. instantiate
//!   2. single-operator authorization (positive)
//!   3. WavsHandleSignedEnvelope full flow → proposal stored
//!   4. replay protection (second submission with same eventId rejected)
//!   5. payload-decode failure rejected
//!   6. unauthorized operator rejected
//!   7. mandate filter Pass / Fail / Fatal
//!   8. auto-execute (no veto / with timelock)
//!   9. Veto lifecycle
//!  10. Close lifecycle

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_json_binary, Addr, Binary, CosmosMsg, Empty, HexBinary, StdError};
use cw_multi_test::{App, ContractWrapper, Executor};
use cw_storage_plus::Item;
use cw_utils::Duration;
use dao_proposal_wavs::filter::{FilterQueryMsg, FilterResponse};
use dao_proposal_wavs::msg::{ExecuteMsg, InstantiateMsg, ProposalPayload, QueryMsg};
use dao_proposal_wavs::state::{AuthorizedService, Config, MandateFilterConfig, WavsProposal};
use dao_proposal_wavs::wavs_compat::{
    ServiceHandlerExecuteMessages, ServiceManagerQueryMessages, WavsEnvelope, WavsSignatureData,
    WavsValidateResult,
};
use dao_voting::status::Status;
use dao_voting::veto::VetoConfig;

const OPERATOR: &str = "wasm1operator";
const NOT_OPERATOR: &str = "wasm1stranger";
const DAO: &str = "wasm1dao";
const VETOER: &str = "wasm1vetoer";

// ----------------------------------------------------------------------------
// Mock service-manager — always returns Ok. Stores no state.
// ----------------------------------------------------------------------------

fn mock_sm_instantiate(
    _deps: cosmwasm_std::DepsMut,
    _env: cosmwasm_std::Env,
    _info: cosmwasm_std::MessageInfo,
    _msg: Empty,
) -> Result<cosmwasm_std::Response, cosmwasm_std::StdError> {
    Ok(cosmwasm_std::Response::default())
}

fn mock_sm_execute(
    _deps: cosmwasm_std::DepsMut,
    _env: cosmwasm_std::Env,
    _info: cosmwasm_std::MessageInfo,
    _msg: Empty,
) -> Result<cosmwasm_std::Response, cosmwasm_std::StdError> {
    Ok(cosmwasm_std::Response::default())
}

fn mock_sm_query(
    _deps: cosmwasm_std::Deps,
    _env: cosmwasm_std::Env,
    msg: ServiceManagerQueryMessages,
) -> Result<Binary, cosmwasm_std::StdError> {
    match msg {
        ServiceManagerQueryMessages::WavsValidate { .. } => to_json_binary(&WavsValidateResult::Ok),
    }
}

fn mock_sm_contract() -> Box<dyn cw_multi_test::Contract<Empty>> {
    Box::new(ContractWrapper::new(
        mock_sm_execute,
        mock_sm_instantiate,
        mock_sm_query,
    ))
}

// ----------------------------------------------------------------------------
// Mock DAO core — accepts ExecuteProposalHook { msgs }, records call count.
// ----------------------------------------------------------------------------

const MOCK_DAO_CALLS: Item<u32> = Item::new("mock_dao_calls");

#[cw_serde]
pub enum MockDaoExecuteMsg {
    ExecuteProposalHook { msgs: Vec<CosmosMsg<Empty>> },
}

#[cw_serde]
pub enum MockDaoQueryMsg {
    Calls {},
}

fn mock_dao_instantiate(
    deps: cosmwasm_std::DepsMut,
    _env: cosmwasm_std::Env,
    _info: cosmwasm_std::MessageInfo,
    _msg: Empty,
) -> Result<cosmwasm_std::Response, StdError> {
    MOCK_DAO_CALLS.save(deps.storage, &0)?;
    Ok(cosmwasm_std::Response::default())
}

fn mock_dao_execute(
    deps: cosmwasm_std::DepsMut,
    _env: cosmwasm_std::Env,
    _info: cosmwasm_std::MessageInfo,
    msg: MockDaoExecuteMsg,
) -> Result<cosmwasm_std::Response, StdError> {
    match msg {
        MockDaoExecuteMsg::ExecuteProposalHook { msgs: _ } => {
            let n = MOCK_DAO_CALLS.load(deps.storage)?;
            MOCK_DAO_CALLS.save(deps.storage, &(n + 1))?;
            Ok(cosmwasm_std::Response::default())
        }
    }
}

fn mock_dao_query(
    deps: cosmwasm_std::Deps,
    _env: cosmwasm_std::Env,
    msg: MockDaoQueryMsg,
) -> Result<Binary, StdError> {
    match msg {
        MockDaoQueryMsg::Calls {} => to_json_binary(&MOCK_DAO_CALLS.load(deps.storage)?),
    }
}

fn mock_dao_contract() -> Box<dyn cw_multi_test::Contract<Empty>> {
    Box::new(ContractWrapper::new(
        mock_dao_execute,
        mock_dao_instantiate,
        mock_dao_query,
    ))
}

// ----------------------------------------------------------------------------
// Mock cw-filter — returns Pass / Fail / Fatal based on a flag in the filter JSON.
// Filter JSON shape: {"verdict": "pass" | "fail" | "fatal"}
// ----------------------------------------------------------------------------

fn mock_filter_instantiate(
    _deps: cosmwasm_std::DepsMut,
    _env: cosmwasm_std::Env,
    _info: cosmwasm_std::MessageInfo,
    _msg: Empty,
) -> Result<cosmwasm_std::Response, StdError> {
    Ok(cosmwasm_std::Response::default())
}

fn mock_filter_execute(
    _deps: cosmwasm_std::DepsMut,
    _env: cosmwasm_std::Env,
    _info: cosmwasm_std::MessageInfo,
    _msg: Empty,
) -> Result<cosmwasm_std::Response, StdError> {
    Ok(cosmwasm_std::Response::default())
}

fn mock_filter_query(
    _deps: cosmwasm_std::Deps,
    _env: cosmwasm_std::Env,
    msg: FilterQueryMsg,
) -> Result<Binary, StdError> {
    match msg {
        FilterQueryMsg::Filter { filter, msg: _ } => {
            let verdict = filter
                .get("verdict")
                .and_then(|v| v.as_str())
                .unwrap_or("pass");
            let resp = match verdict {
                "fail" => FilterResponse::Fail {
                    reason: "mock fail".to_string(),
                },
                "fatal" => FilterResponse::Fatal {
                    reason: "mock fatal".to_string(),
                },
                _ => FilterResponse::Pass {},
            };
            to_json_binary(&resp)
        }
    }
}

fn mock_filter_contract() -> Box<dyn cw_multi_test::Contract<Empty>> {
    Box::new(ContractWrapper::new(
        mock_filter_execute,
        mock_filter_instantiate,
        mock_filter_query,
    ))
}

// ----------------------------------------------------------------------------
// dao-proposal-wavs contract wrapper
// ----------------------------------------------------------------------------

fn dao_proposal_wavs_contract() -> Box<dyn cw_multi_test::Contract<Empty>> {
    Box::new(ContractWrapper::new(
        dao_proposal_wavs::contract::execute,
        dao_proposal_wavs::contract::instantiate,
        dao_proposal_wavs::contract::query,
    ))
}

// ----------------------------------------------------------------------------
// Helpers — build a canonical Solidity-ABI-encoded envelope:
//   Envelope { bytes20 eventId; bytes12 ordering; bytes payload }
// matches `wavs-types::WavsEnvelope(Binary)` byte-for-byte so envelopes built here
// would round-trip through cw-middleware v0.3.0's real verification path if signed.
// ----------------------------------------------------------------------------

fn abi_encode_envelope(event_id: [u8; 20], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(96 + 32 + ((payload.len() + 31) / 32) * 32);
    // Slot 0: eventId, right-padded.
    out.extend_from_slice(&event_id);
    out.extend_from_slice(&[0u8; 12]);
    // Slot 1: ordering, right-padded (zero by default — unused by our handler).
    out.extend_from_slice(&[0u8; 32]);
    // Slot 2: offset pointer to payload tail.
    let mut slot = [0u8; 32];
    slot[24..32].copy_from_slice(&96u64.to_be_bytes());
    out.extend_from_slice(&slot);
    // Tail: payload length.
    let mut slot = [0u8; 32];
    slot[24..32].copy_from_slice(&(payload.len() as u64).to_be_bytes());
    out.extend_from_slice(&slot);
    // Tail: payload data, padded to 32-byte multiple.
    out.extend_from_slice(payload);
    let pad = (32 - (payload.len() % 32)) % 32;
    if pad > 0 {
        out.extend(std::iter::repeat(0u8).take(pad));
    }
    out
}

fn build_envelope(event_id: [u8; 20], payload: &ProposalPayload) -> WavsEnvelope {
    let payload_bytes = serde_json::to_vec(payload).unwrap();
    WavsEnvelope(Binary::from(abi_encode_envelope(event_id, &payload_bytes)))
}

fn empty_sigs() -> WavsSignatureData {
    WavsSignatureData {
        signers: vec![OPERATOR.to_string()],
        signatures: vec![HexBinary::from(b"sig".to_vec())],
        reference_block: 0,
    }
}

fn setup_app() -> (App, Addr, Addr) {
    let (ctx,) = setup_app_full(None, None, false);
    (ctx.app, ctx.sm, ctx.prop)
}

/// Setup variant that lets a test pick the mandate filter, veto, and auto_execute config.
/// Returns (app, sm_addr, prop_addr) in a tuple plus optional dao_addr for tests that need it.
#[allow(dead_code)]
struct AppCtx {
    app: App,
    sm: Addr,
    prop: Addr,
    dao: Addr,
    filter: Option<Addr>,
}

fn setup_app_full(
    mandate_filter_verdict: Option<&str>,
    veto: Option<VetoConfig>,
    auto_execute: bool,
) -> (AppCtx,) {
    let mut app = App::default();

    // Mock service-manager
    let sm_id = app.store_code(mock_sm_contract());
    let sm_addr = app
        .instantiate_contract(
            sm_id,
            Addr::unchecked(DAO),
            &Empty {},
            &[],
            "mock-service-manager",
            None,
        )
        .unwrap();

    // Mock DAO core
    let dao_id = app.store_code(mock_dao_contract());
    let dao_addr = app
        .instantiate_contract(
            dao_id,
            Addr::unchecked(DAO),
            &Empty {},
            &[],
            "mock-dao",
            None,
        )
        .unwrap();

    // Optional mock cw-filter
    let (filter_addr, mandate_filter_cfg) = match mandate_filter_verdict {
        Some(verdict) => {
            let f_id = app.store_code(mock_filter_contract());
            let f_addr = app
                .instantiate_contract(
                    f_id,
                    Addr::unchecked(DAO),
                    &Empty {},
                    &[],
                    "mock-filter",
                    None,
                )
                .unwrap();
            (
                Some(f_addr.clone()),
                Some(MandateFilterConfig {
                    filter_contract: f_addr,
                    filter: serde_json::json!({ "verdict": verdict }),
                }),
            )
        }
        None => (None, None),
    };

    // dao-proposal-wavs — instantiated *as the DAO core* so cfg.dao = dao_addr.
    let prop_id = app.store_code(dao_proposal_wavs_contract());
    let prop_addr = app
        .instantiate_contract(
            prop_id,
            dao_addr.clone(),
            &InstantiateMsg {
                service_manager: sm_addr.to_string(),
                authorized_service: AuthorizedService::SingleOperator {
                    addr: Addr::unchecked(OPERATOR),
                },
                mandate_filter: mandate_filter_cfg,
                veto,
                auto_execute,
                close_proposal_on_execution_failure: true,
            },
            &[],
            "dao-proposal-wavs",
            None,
        )
        .unwrap();

    (AppCtx {
        app,
        sm: sm_addr,
        prop: prop_addr,
        dao: dao_addr,
        filter: filter_addr,
    },)
}

// ----------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------

#[test]
fn instantiate_works() {
    let (app, sm_addr, prop_addr) = setup_app();

    let cfg: Config = app
        .wrap()
        .query_wasm_smart(prop_addr.clone(), &QueryMsg::Config {})
        .unwrap();

    assert_eq!(cfg.service_manager, sm_addr);
    assert!(matches!(
        cfg.authorized_service,
        AuthorizedService::SingleOperator { .. }
    ));
    assert!(!cfg.auto_execute);
    assert!(cfg.close_proposal_on_execution_failure);

    let count: u64 = app
        .wrap()
        .query_wasm_smart(prop_addr, &QueryMsg::ProposalCount {})
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn happy_path_proposal_created() {
    let (mut app, _, prop_addr) = setup_app();

    let payload = ProposalPayload {
        title: "test proposal".to_string(),
        description: "first".to_string(),
        msgs: vec![],
    };
    let event_id = [42u8; 20];
    let envelope = build_envelope(event_id, &payload);

    app.execute_contract(
        Addr::unchecked(OPERATOR),
        prop_addr.clone(),
        &ExecuteMsg::ServiceHandler(ServiceHandlerExecuteMessages::WavsHandleSignedEnvelope {
            envelope,
            signature_data: empty_sigs(),
        }),
        &[],
    )
    .unwrap();

    let count: u64 = app
        .wrap()
        .query_wasm_smart(prop_addr.clone(), &QueryMsg::ProposalCount {})
        .unwrap();
    assert_eq!(count, 1);

    let proposal: WavsProposal = app
        .wrap()
        .query_wasm_smart(prop_addr.clone(), &QueryMsg::Proposal { proposal_id: 1 })
        .unwrap();
    assert_eq!(proposal.title, "test proposal");
    assert_eq!(proposal.description, "first");
    assert_eq!(
        proposal.event_id_hex,
        "2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a"
    );

    let seen: bool = app
        .wrap()
        .query_wasm_smart(
            prop_addr,
            &QueryMsg::EventIdSeen {
                event_id_hex: "2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a".to_string(),
            },
        )
        .unwrap();
    assert!(seen);
}

#[test]
fn replay_rejected() {
    let (mut app, _, prop_addr) = setup_app();

    let payload = ProposalPayload {
        title: "first".to_string(),
        description: "".to_string(),
        msgs: vec![],
    };
    let envelope = build_envelope([7u8; 20], &payload);

    app.execute_contract(
        Addr::unchecked(OPERATOR),
        prop_addr.clone(),
        &ExecuteMsg::ServiceHandler(ServiceHandlerExecuteMessages::WavsHandleSignedEnvelope {
            envelope: envelope.clone(),
            signature_data: empty_sigs(),
        }),
        &[],
    )
    .unwrap();

    // Same eventId — should be rejected.
    let result = app.execute_contract(
        Addr::unchecked(OPERATOR),
        prop_addr,
        &ExecuteMsg::ServiceHandler(ServiceHandlerExecuteMessages::WavsHandleSignedEnvelope {
            envelope,
            signature_data: empty_sigs(),
        }),
        &[],
    );
    assert!(result.is_err());
    let err = format!("{:?}", result.err().unwrap());
    assert!(err.contains("Replay"), "expected Replay error, got: {err}");
}

#[test]
fn unauthorized_operator_rejected() {
    let (mut app, _, prop_addr) = setup_app();

    let payload = ProposalPayload {
        title: "x".to_string(),
        description: "".to_string(),
        msgs: vec![],
    };
    let envelope = build_envelope([1u8; 20], &payload);

    let result = app.execute_contract(
        Addr::unchecked(NOT_OPERATOR),
        prop_addr,
        &ExecuteMsg::ServiceHandler(ServiceHandlerExecuteMessages::WavsHandleSignedEnvelope {
            envelope,
            signature_data: empty_sigs(),
        }),
        &[],
    );
    assert!(result.is_err());
    let err = format!("{:?}", result.err().unwrap());
    assert!(
        err.contains("Unauthorized"),
        "expected Unauthorized error, got: {err}"
    );
}

#[test]
fn malformed_payload_rejected() {
    let (mut app, _, prop_addr) = setup_app();

    // Canonical ABI envelope, but the payload bytes are not valid JSON.
    let envelope = WavsEnvelope(Binary::from(abi_encode_envelope(
        [9u8; 20],
        b"not valid json {{",
    )));

    let result = app.execute_contract(
        Addr::unchecked(OPERATOR),
        prop_addr,
        &ExecuteMsg::ServiceHandler(ServiceHandlerExecuteMessages::WavsHandleSignedEnvelope {
            envelope,
            signature_data: empty_sigs(),
        }),
        &[],
    );
    assert!(result.is_err());
    let err = format!("{:?}", result.err().unwrap());
    assert!(
        err.contains("Invalid envelope payload"),
        "expected Invalid envelope payload error, got: {err}"
    );
}

#[test]
fn short_envelope_rejected() {
    let (mut app, _, prop_addr) = setup_app();

    // Envelope shorter than the 96-byte ABI head — rejected at eventId extraction.
    let envelope = WavsEnvelope(Binary::from(vec![0u8; 50]));

    let result = app.execute_contract(
        Addr::unchecked(OPERATOR),
        prop_addr,
        &ExecuteMsg::ServiceHandler(ServiceHandlerExecuteMessages::WavsHandleSignedEnvelope {
            envelope,
            signature_data: empty_sigs(),
        }),
        &[],
    );
    assert!(result.is_err());
}

// ----------------------------------------------------------------------------
// v0.3 tests: mandate filter, auto-execute, veto, close, timelock
// ----------------------------------------------------------------------------

fn submit_envelope(
    ctx: &mut AppCtx,
    event_id: [u8; 20],
    payload: ProposalPayload,
) -> Result<(), String> {
    let envelope = build_envelope(event_id, &payload);
    ctx.app
        .execute_contract(
            Addr::unchecked(OPERATOR),
            ctx.prop.clone(),
            &ExecuteMsg::ServiceHandler(ServiceHandlerExecuteMessages::WavsHandleSignedEnvelope {
                envelope,
                signature_data: empty_sigs(),
            }),
            &[],
        )
        .map(|_| ())
        .map_err(|e| format!("{e:?}"))
}

fn simple_payload() -> ProposalPayload {
    ProposalPayload {
        title: "t".into(),
        description: "d".into(),
        msgs: vec![],
    }
}

#[test]
fn mandate_filter_pass_allows_proposal() {
    let (mut ctx,) = setup_app_full(Some("pass"), None, false);
    submit_envelope(&mut ctx, [1u8; 20], simple_payload()).unwrap();
    let count: u64 = ctx
        .app
        .wrap()
        .query_wasm_smart(ctx.prop, &QueryMsg::ProposalCount {})
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn mandate_filter_fail_rejects() {
    let (mut ctx,) = setup_app_full(Some("fail"), None, false);
    // Need a payload with at least one msg so the filter is invoked.
    let payload = ProposalPayload {
        title: "t".into(),
        description: "d".into(),
        msgs: vec![CosmosMsg::Custom(Empty {})],
    };
    let err = submit_envelope(&mut ctx, [1u8; 20], payload).unwrap_err();
    assert!(
        err.contains("Mandate filter rejected") && err.contains("mock fail"),
        "expected Mandate filter rejected with mock fail, got: {err}"
    );
}

#[test]
fn mandate_filter_fatal_rejects() {
    let (mut ctx,) = setup_app_full(Some("fatal"), None, false);
    let payload = ProposalPayload {
        title: "t".into(),
        description: "d".into(),
        msgs: vec![CosmosMsg::Custom(Empty {})],
    };
    let err = submit_envelope(&mut ctx, [1u8; 20], payload).unwrap_err();
    assert!(
        err.contains("Mandate filter encountered a fatal error") && err.contains("mock fatal"),
        "expected fatal error with mock fatal, got: {err}"
    );
}

#[test]
fn auto_execute_no_veto_dispatches_to_dao() {
    let (mut ctx,) = setup_app_full(None, None, true);

    // Payload includes one message that will get sent through to the mock DAO.
    let payload = ProposalPayload {
        title: "auto".into(),
        description: "".into(),
        msgs: vec![CosmosMsg::Custom(Empty {})],
    };
    submit_envelope(&mut ctx, [1u8; 20], payload).unwrap();

    // Mock DAO should have received one ExecuteProposalHook call.
    let calls: u32 = ctx
        .app
        .wrap()
        .query_wasm_smart(ctx.dao, &MockDaoQueryMsg::Calls {})
        .unwrap();
    assert_eq!(calls, 1);

    // Proposal status = Executed.
    let p: WavsProposal = ctx
        .app
        .wrap()
        .query_wasm_smart(ctx.prop, &QueryMsg::Proposal { proposal_id: 1 })
        .unwrap();
    assert!(matches!(p.status, Status::Executed));
}

#[test]
fn auto_execute_with_veto_waits_for_timelock() {
    let veto = VetoConfig {
        timelock_duration: Duration::Height(100),
        vetoer: VETOER.to_string(),
        early_execute: false,
        veto_before_passed: false,
    };
    let (mut ctx,) = setup_app_full(None, Some(veto), true);

    submit_envelope(&mut ctx, [1u8; 20], simple_payload()).unwrap();

    // Status should be Passed (not Executed) — auto_execute + veto means timelock starts.
    let p: WavsProposal = ctx
        .app
        .wrap()
        .query_wasm_smart(ctx.prop.clone(), &QueryMsg::Proposal { proposal_id: 1 })
        .unwrap();
    assert!(matches!(p.status, Status::Passed));

    // DAO should NOT have been called yet.
    let calls: u32 = ctx
        .app
        .wrap()
        .query_wasm_smart(ctx.dao.clone(), &MockDaoQueryMsg::Calls {})
        .unwrap();
    assert_eq!(calls, 0);

    // Try to Execute right away — should fail with TimelockNotExpired.
    let err = ctx
        .app
        .execute_contract(
            Addr::unchecked(OPERATOR),
            ctx.prop.clone(),
            &ExecuteMsg::Execute { proposal_id: 1 },
            &[],
        )
        .unwrap_err();
    assert!(format!("{err:?}").contains("Timelock"));

    // Advance height past timelock.
    ctx.app.update_block(|b| b.height += 101);

    // Now Execute should succeed.
    ctx.app
        .execute_contract(
            Addr::unchecked(OPERATOR),
            ctx.prop.clone(),
            &ExecuteMsg::Execute { proposal_id: 1 },
            &[],
        )
        .unwrap();

    let p: WavsProposal = ctx
        .app
        .wrap()
        .query_wasm_smart(ctx.prop, &QueryMsg::Proposal { proposal_id: 1 })
        .unwrap();
    assert!(matches!(p.status, Status::Executed));
    let calls: u32 = ctx
        .app
        .wrap()
        .query_wasm_smart(ctx.dao, &MockDaoQueryMsg::Calls {})
        .unwrap();
    assert_eq!(calls, 1);
}

#[test]
fn veto_succeeds_within_timelock() {
    let veto = VetoConfig {
        timelock_duration: Duration::Height(100),
        vetoer: VETOER.to_string(),
        early_execute: false,
        veto_before_passed: false,
    };
    let (mut ctx,) = setup_app_full(None, Some(veto), true);

    submit_envelope(&mut ctx, [1u8; 20], simple_payload()).unwrap();

    // Vetoer kills it during the timelock.
    ctx.app
        .execute_contract(
            Addr::unchecked(VETOER),
            ctx.prop.clone(),
            &ExecuteMsg::Veto { proposal_id: 1 },
            &[],
        )
        .unwrap();

    let p: WavsProposal = ctx
        .app
        .wrap()
        .query_wasm_smart(ctx.prop, &QueryMsg::Proposal { proposal_id: 1 })
        .unwrap();
    assert!(matches!(p.status, Status::Vetoed));
}

#[test]
fn veto_by_non_vetoer_rejected() {
    let veto = VetoConfig {
        timelock_duration: Duration::Height(100),
        vetoer: VETOER.to_string(),
        early_execute: false,
        veto_before_passed: false,
    };
    let (mut ctx,) = setup_app_full(None, Some(veto), true);

    submit_envelope(&mut ctx, [1u8; 20], simple_payload()).unwrap();

    // Wrong sender — should be Unauthorized.
    let err = ctx
        .app
        .execute_contract(
            Addr::unchecked(NOT_OPERATOR),
            ctx.prop,
            &ExecuteMsg::Veto { proposal_id: 1 },
            &[],
        )
        .unwrap_err();
    assert!(format!("{err:?}").contains("Unauthorized"));
}

#[test]
fn close_after_veto_succeeds() {
    let veto = VetoConfig {
        timelock_duration: Duration::Height(100),
        vetoer: VETOER.to_string(),
        early_execute: false,
        veto_before_passed: false,
    };
    let (mut ctx,) = setup_app_full(None, Some(veto), true);

    submit_envelope(&mut ctx, [1u8; 20], simple_payload()).unwrap();

    // Veto first.
    ctx.app
        .execute_contract(
            Addr::unchecked(VETOER),
            ctx.prop.clone(),
            &ExecuteMsg::Veto { proposal_id: 1 },
            &[],
        )
        .unwrap();

    // Now close.
    ctx.app
        .execute_contract(
            Addr::unchecked(OPERATOR),
            ctx.prop.clone(),
            &ExecuteMsg::Close { proposal_id: 1 },
            &[],
        )
        .unwrap();

    let p: WavsProposal = ctx
        .app
        .wrap()
        .query_wasm_smart(ctx.prop, &QueryMsg::Proposal { proposal_id: 1 })
        .unwrap();
    assert!(matches!(p.status, Status::Closed));
}

#[test]
fn close_open_proposal_rejected() {
    let (mut ctx,) = setup_app_full(None, None, false);
    submit_envelope(&mut ctx, [1u8; 20], simple_payload()).unwrap();

    // Status is Open (no auto_execute, no veto). Closing should fail.
    let err = ctx
        .app
        .execute_contract(
            Addr::unchecked(OPERATOR),
            ctx.prop,
            &ExecuteMsg::Close { proposal_id: 1 },
            &[],
        )
        .unwrap_err();
    assert!(
        format!("{err:?}").contains("InvalidProposalState")
            || format!("{err:?}").contains("not in a state")
    );
}
