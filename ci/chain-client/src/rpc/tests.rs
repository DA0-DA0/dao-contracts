use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use base64::{engine::general_purpose::STANDARD, Engine};
use cosmrs::{
    crypto::secp256k1::{Signature, VerifyingKey},
    proto::{
        cosmos::{
            base::abci::v1beta1::GasInfo,
            crypto::secp256k1::PubKey,
            tx::v1beta1::{AuthInfo, SignDoc, TxBody, TxRaw},
        },
        cosmwasm::wasm::v1::{MsgInstantiateContractResponse, MsgStoreCode, MsgStoreCodeResponse},
    },
};
use serde_json::{json, Value};
use signature::Verifier;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
    task::{JoinHandle, JoinSet},
};

use super::*;
use crate::{
    config::{Config, DeployInfo},
    error::CosmwasmError,
    Key,
};

const WASM: &[u8] = b"\0asm\x01\0\0\0";

fn config() -> Config {
    serde_yaml::from_str(include_str!("../../../configs/cosm-orc/ci.yaml")).unwrap()
}

fn fixtures() -> Vec<Value> {
    serde_json::from_str(include_str!("../../../configs/test_accounts.json")).unwrap()
}

fn key() -> SigningKey {
    fixture_key(&fixtures()[0])
}

fn fixture_key(account: &Value) -> SigningKey {
    SigningKey {
        name: account["name"].as_str().unwrap().into(),
        key: Key::Mnemonic(account["mnemonic"].as_str().unwrap().into()),
        derivation_path: config().chain_cfg.derivation_path,
    }
}

fn message(sender: &AccountId) -> Result<Any> {
    Ok(Any {
        type_url: "/cosmwasm.wasm.v1.MsgStoreCode".into(),
        value: MsgStoreCode {
            sender: sender.to_string(),
            wasm_byte_code: WASM.into(),
            instantiate_permission: None,
        }
        .encode_to_vec(),
    })
}

fn tx_response<T: Message>(type_url: &str, response: T) -> Vec<u8> {
    TxMsgData {
        msg_responses: vec![Any {
            type_url: type_url.into(),
            value: response.encode_to_vec(),
        }],
        ..Default::default()
    }
    .encode_to_vec()
}

fn status() -> Value {
    json!({
        "node_info": { "protocol_version": {"p2p":"8","block":"11","app":"0"},
            "id": "0000000000000000000000000000000000000000", "listen_addr": "tcp://127.0.0.1:26656",
            "network":"testing", "version":"0.37.2", "channels":"40202122233038606100", "moniker":"mock",
            "other":{"tx_index":"on", "rpc_address":"tcp://127.0.0.1:26657"}},
        "sync_info": {"latest_block_hash":"00".repeat(32), "latest_app_hash":"00".repeat(32),
            "latest_block_height":"12", "latest_block_time":"2024-01-01T00:00:00Z",
            "earliest_block_hash":"00".repeat(32), "earliest_app_hash":"00".repeat(32),
            "earliest_block_height":"1", "earliest_block_time":"2024-01-01T00:00:00Z", "catching_up":false},
        "validator_info": {"address":"00".repeat(20),
            "pub_key":{"type":"tendermint/PubKeyEd25519", "value":STANDARD.encode([0;32])}, "voting_power":"10"}
    })
}

fn query_result(data: Vec<u8>) -> Value {
    json!({"response":{"code":0,"log":"","info":"","index":"0","key":"",
        "value":STANDARD.encode(data),"height":"12","codespace":""}})
}

fn normal(req: &Value) -> Value {
    match req["method"].as_str().unwrap() {
        "status" => status(),
        "abci_query" => match req["params"]["path"].as_str().unwrap() {
            "/cosmos.auth.v1beta1.Query/Account" => query_result(
                QueryAccountResponse {
                    account: Some(Any {
                        type_url: "/cosmos.auth.v1beta1.BaseAccount".into(),
                        value: BaseAccount {
                            address: fixtures()[0]["address"].as_str().unwrap().into(),
                            pub_key: None,
                            account_number: 7,
                            sequence: 3,
                        }
                        .encode_to_vec(),
                    }),
                }
                .encode_to_vec(),
            ),
            "/cosmos.tx.v1beta1.Service/Simulate" => query_result(
                SimulateResponse {
                    gas_info: Some(GasInfo {
                        gas_wanted: 20001,
                        gas_used: 20001,
                    }),
                    result: None,
                }
                .encode_to_vec(),
            ),
            other => panic!("unexpected ABCI path {other}"),
        },
        "broadcast_tx_commit" => {
            let bytes = STANDARD
                .decode(req["params"]["tx"].as_str().unwrap())
                .unwrap();
            json!({"check_tx":{"code":0,"data":"","log":"","info":"","gas_wanted":"30002","gas_used":"0","events":[],"codespace":""},
                "deliver_tx":{"code":0,"data":STANDARD.encode(tx_response("/cosmwasm.wasm.v1.MsgStoreCodeResponse",
                    MsgStoreCodeResponse { code_id:42, checksum: Sha256::digest(WASM).to_vec() })),
                    "log":"execution log", "info":"", "gas_wanted":"30002", "gas_used":"20001", "codespace":"",
                    "events":[{"type":"store_code","attributes":[{"key":"code_id","value":"42","index":true}]}]},
                "hash":transaction_hash(&bytes).to_string(),"height":"12"})
        }
        other => panic!("unexpected RPC method {other}"),
    }
}

enum Reply {
    Json(Value),
    Delay(Duration, Value),
    Hang,
    Disconnect,
    Redirect,
}

struct Server {
    endpoint: String,
    requests: Arc<Mutex<Vec<Value>>>,
    stop: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}

impl Server {
    async fn new(handler: impl Fn(&Value) -> Reply + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let (stop, mut stopped) = oneshot::channel();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&requests);
        let handler = Arc::new(handler);
        let redirect_to = endpoint.clone();
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            loop {
                tokio::select! {
                    _ = &mut stopped => break,
                    completed = connections.join_next(), if !connections.is_empty() => { completed.unwrap().unwrap(); },
                    accepted = listener.accept() => {
                        let (stream, _) = accepted.unwrap();
                        let (seen, handler, redirect_to) = (Arc::clone(&seen), Arc::clone(&handler), redirect_to.clone());
                        connections.spawn(async move { serve(stream, seen, handler.as_ref(), &redirect_to).await; });
                    }
                }
            }
            // Explicit shutdown also cancels intentionally stalled connections.
            connections.shutdown().await;
        });
        Self {
            endpoint,
            requests,
            stop: Some(stop),
            task: Some(task),
        }
    }

    fn client(&self) -> Rpc {
        let mut cfg = config().chain_cfg;
        cfg.rpc_endpoint = Some(self.endpoint.clone());
        Rpc::new(cfg).unwrap()
    }

    fn broadcasts(&self) -> usize {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r["method"] == "broadcast_tx_commit")
            .count()
    }

    async fn stop(mut self) {
        self.stop.take().unwrap().send(()).unwrap();
        self.task.take().unwrap().await.unwrap();
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}

async fn serve(
    mut stream: TcpStream,
    seen: Arc<Mutex<Vec<Value>>>,
    handler: &(impl Fn(&Value) -> Reply + ?Sized),
    endpoint: &str,
) {
    let mut bytes = Vec::new();
    let header_end = loop {
        if stream.read_buf(&mut bytes).await.unwrap() == 0 {
            return;
        }
        if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break end + 4;
        }
        assert!(bytes.len() < 16384, "excessive HTTP headers");
    };
    let headers = std::str::from_utf8(&bytes[..header_end]).unwrap();
    let len: usize = headers
        .lines()
        .find_map(|l| {
            l.split_once(':')
                .filter(|(key, _)| key.eq_ignore_ascii_case("content-length"))
                .map(|(_, value)| value.trim().parse().unwrap())
        })
        .unwrap();
    while bytes.len() < header_end + len {
        if stream.read_buf(&mut bytes).await.unwrap() == 0 {
            return;
        }
    }
    let req: Value = serde_json::from_slice(&bytes[header_end..header_end + len]).unwrap();
    seen.lock().unwrap().push(req.clone());
    let reply = handler(&req);
    let value = match reply {
        Reply::Json(value) => value,
        Reply::Delay(delay, value) => {
            sleep(delay).await;
            value
        }
        Reply::Hang => {
            std::future::pending::<()>().await;
            unreachable!()
        }
        Reply::Disconnect => return,
        Reply::Redirect => {
            let response = format!("HTTP/1.1 307 Temporary Redirect\r\nLocation: {endpoint}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            let _ = stream.write_all(response.as_bytes()).await;
            return;
        }
    };
    let body = json!({"jsonrpc":"2.0","id":req["id"],"result":value}).to_string();
    let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
    let _ = stream.write_all(response.as_bytes()).await;
}

#[test]
fn all_fixture_keys_paths_prefixes_and_redaction() {
    for account in fixtures() {
        let key = fixture_key(&account);
        assert_eq!(
            key.address("juno").unwrap().as_ref(),
            account["address"].as_str().unwrap()
        );
        assert!(!format!("{key:?}").contains(account["mnemonic"].as_str().unwrap()));
        let mut different = key.clone();
        different.derivation_path = "m/44'/118'/0'/0/1".into();
        assert_ne!(
            different.address("juno").unwrap(),
            key.address("juno").unwrap()
        );
    }
    let mut bad = key();
    bad.derivation_path = "invalid".into();
    assert!(matches!(bad.address("juno"), Err(ProcessError::Signing(_))));
    bad.key = Key::Mnemonic("not a mnemonic".into());
    assert!(matches!(bad.address("juno"), Err(ProcessError::Signing(_))));
}

#[test]
fn configuration_round_trip_and_fee_bounds() {
    let mut cfg = config();
    cfg.chain_cfg.rpc_endpoint = None;
    assert!(
        matches!(cfg.chain_cfg.validate(), Err(ProcessError::Config(s)) if s.contains("rpc_endpoint"))
    );
    cfg.chain_cfg.rpc_endpoint = Some("http://127.0.0.1:26657".into());
    cfg.contract_deploy_info.insert(
        "test-default".into(),
        DeployInfo {
            code_id: Some(42),
            address: Some("stored-address".into()),
        },
    );
    let restored: Config = serde_yaml::from_str(&serde_yaml::to_string(&cfg).unwrap()).unwrap();
    assert_eq!(restored.contract_deploy_info, cfg.contract_deploy_info);
    assert_eq!(
        restored.chain_cfg.grpc_endpoint,
        cfg.chain_cfg.grpc_endpoint
    );
    assert_eq!(restored.chain_cfg.rpc_endpoint, cfg.chain_cfg.rpc_endpoint);
    let fee = signing::fee(20001, &cfg.chain_cfg).unwrap();
    assert_eq!(fee.gas_limit, 30002);
    assert_eq!(fee.amount[0].amount, 3001);
    assert!(signing::fee(u64::MAX, &cfg.chain_cfg).is_err());
    cfg.chain_cfg.gas_price = f64::NAN;
    assert!(cfg.chain_cfg.validate().is_err());
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

#[tokio::test]
async fn real_client_signs_once_preserves_envelopes_and_committed_events() {
    let server = Server::new(|req| Reply::Json(normal(req))).await;
    let receipt = server.client().transact(&key(), message).await.unwrap();
    assert_eq!(
        (receipt.height, receipt.gas_wanted, receipt.gas_used),
        (12, 30002, 20001)
    );
    assert_eq!(receipt.log, "execution log");
    assert_eq!(
        receipt.events[0].attributes[0].key_str().unwrap(),
        "code_id"
    );
    assert_eq!(receipt.events[0].attributes[0].value_str().unwrap(), "42");
    let response: MsgStoreCodeResponse = receipt
        .message_response("/cosmwasm.wasm.v1.MsgStoreCodeResponse")
        .unwrap();
    assert_eq!(response.code_id, 42);
    let requests = server.requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 4);
    assert_eq!(server.broadcasts(), 1);
    let sim =
        SimulateRequest::decode(unhex(requests[2]["params"]["data"].as_str().unwrap()).as_slice())
            .unwrap();
    let simulation = TxRaw::decode(sim.tx_bytes.as_slice()).unwrap();
    let body = TxBody::decode(simulation.body_bytes.as_slice()).unwrap();
    let auth = AuthInfo::decode(simulation.auth_info_bytes.as_slice()).unwrap();
    assert_eq!(body.memo, signing::SIMULATION_MEMO);
    assert_eq!(body.timeout_height, 0);
    assert_eq!(simulation.signatures, vec![Vec::<u8>::new()]);
    assert!(auth.signer_infos[0].public_key.is_none());
    assert_eq!(auth.signer_infos[0].sequence, 3);
    assert_eq!(auth.fee.unwrap().amount[0].amount, "0");
    let tx = STANDARD
        .decode(requests[3]["params"]["tx"].as_str().unwrap())
        .unwrap();
    let signed = TxRaw::decode(tx.as_slice()).unwrap();
    let final_body = TxBody::decode(signed.body_bytes.as_slice()).unwrap();
    let final_auth = AuthInfo::decode(signed.auth_info_bytes.as_slice()).unwrap();
    assert_eq!(final_body.messages, body.messages);
    assert_eq!(final_body.memo, signing::TRANSACTION_MEMO);
    assert_eq!(final_body.timeout_height, 0);
    let store = MsgStoreCode::decode(final_body.messages[0].value.as_slice()).unwrap();
    assert_eq!(store.wasm_byte_code, WASM); // no gzip, aliasing, or Wasm execution claim
    assert_eq!(store.sender, fixtures()[0]["address"]);
    let fee = final_auth.fee.unwrap();
    assert_eq!(fee.gas_limit, 30002);
    assert_eq!(fee.amount[0].amount, "3001");
    assert_eq!(fee.amount[0].denom, "ujuno");
    let public = PubKey::decode(
        final_auth.signer_infos[0]
            .public_key
            .as_ref()
            .unwrap()
            .value
            .as_slice(),
    )
    .unwrap();
    assert_eq!(signed.signatures.len(), 1);
    let doc = SignDoc {
        body_bytes: signed.body_bytes,
        auth_info_bytes: signed.auth_info_bytes,
        chain_id: "testing".into(),
        account_number: 7,
    };
    VerifyingKey::from_sec1_bytes(&public.key)
        .unwrap()
        .verify(
            &doc.encode_to_vec(),
            &Signature::from_slice(&signed.signatures[0]).unwrap(),
        )
        .unwrap();
    assert_eq!(receipt.hash, transaction_hash(&tx).to_string());
    server.stop().await;
}

#[tokio::test]
async fn rejection_and_malformed_response_matrix() {
    for case in [
        "check",
        "execute",
        "zero-height",
        "wrong-hash",
        "invalid-hash",
        "missing-hash",
        "missing-height",
        "missing-check",
        "missing-execute",
        "negative-gas",
        "lowercase-hash",
        "malformed-data",
    ] {
        let server = Server::new(move |req| {
            let mut value = normal(req);
            if req["method"] == "broadcast_tx_commit" {
                match case {
                    "check" => {
                        value["check_tx"]["code"] = json!(9);
                        value["check_tx"]["log"] = json!("bad sequence");
                        value["height"] = json!("0");
                    }
                    "execute" => {
                        value["deliver_tx"]["code"] = json!(5);
                        value["deliver_tx"]["log"] = json!("unauthorized");
                    }
                    "zero-height" => value["height"] = json!("0"),
                    "wrong-hash" => value["hash"] = json!("11".repeat(32)),
                    "invalid-hash" => value["hash"] = json!("not a hash"),
                    "missing-hash" => {
                        value.as_object_mut().unwrap().remove("hash");
                    }
                    "missing-height" => {
                        value.as_object_mut().unwrap().remove("height");
                    }
                    "missing-check" => {
                        value.as_object_mut().unwrap().remove("check_tx");
                    }
                    "missing-execute" => {
                        value.as_object_mut().unwrap().remove("deliver_tx");
                    }
                    "negative-gas" => value["deliver_tx"]["gas_used"] = json!("-1"),
                    // Comet 0.37 uses canonical uppercase hex. The maintained
                    // Hash decoder rejects noncanonical casing, rather than
                    // normalizing an unvalidated response.
                    "lowercase-hash" => {
                        value["hash"] = json!(value["hash"].as_str().unwrap().to_lowercase())
                    }
                    "malformed-data" => value["deliver_tx"]["data"] = json!("not base64"),
                    _ => unreachable!(),
                }
            }
            Reply::Json(value)
        })
        .await;
        let err = server.client().transact(&key(), message).await.unwrap_err();
        match case {
            "check" => assert!(matches!(
                err,
                ProcessError::CosmwasmError(CosmwasmError::TxError(TxError::CheckTx {
                    code: 9,
                    ..
                }))
            )),
            "execute" => match err {
                ProcessError::CosmwasmError(CosmwasmError::TxError(TxError::Execution {
                    code: 5,
                    response,
                    ..
                })) => {
                    assert_eq!(response.log, "unauthorized");
                    assert_eq!(response.gas_used, 20001);
                }
                other => panic!("unexpected execution error: {other}"),
            },
            _ => assert!(
                !matches!(err, ProcessError::CosmwasmError(_)),
                "{case}: {err}"
            ),
        }
        assert_eq!(server.broadcasts(), 1, "{case}");
        server.stop().await;
    }
}

#[tokio::test]
async fn delayed_commit_is_not_early_success() {
    let server = Server::new(|req| {
        let value = normal(req);
        if req["method"] == "broadcast_tx_commit" {
            Reply::Delay(Duration::from_millis(150), value)
        } else {
            Reply::Json(value)
        }
    })
    .await;
    let start = Instant::now();
    let res = server.client().transact(&key(), message).await.unwrap();
    assert!(start.elapsed() >= Duration::from_millis(150));
    assert_eq!(res.height, 12);
    assert_eq!(server.broadcasts(), 1);
    server.stop().await;
}

#[tokio::test]
async fn deadlines_cover_each_stalled_call_and_total_operation() {
    for stalled in [
        "status",
        "account",
        "simulate",
        "broadcast_tx_commit",
        "cumulative",
    ] {
        let server = Server::new(move |req| {
            let method = req["method"].as_str().unwrap();
            let path = req["params"]["path"].as_str().unwrap_or("");
            if method == stalled
                || (stalled == "account" && path.ends_with("/Account"))
                || (stalled == "simulate" && path.ends_with("/Simulate"))
            {
                Reply::Hang
            } else if stalled == "cumulative" {
                Reply::Delay(Duration::from_millis(200), normal(req))
            } else {
                Reply::Json(normal(req))
            }
        })
        .await;
        let mut client = server.client();
        client.operation_timeout = Duration::from_millis(500);
        let start = Instant::now();
        let err = client.transact(&key(), message).await.unwrap_err();
        assert!(start.elapsed() < Duration::from_secs(2), "{stalled}");
        assert!(
            matches!(err,ProcessError::Timeout{hash:Some(_)} if stalled=="broadcast_tx_commit")
                || matches!(err,ProcessError::Timeout{hash:None} if stalled!="broadcast_tx_commit"),
            "{stalled}: {err}"
        );
        assert_eq!(
            server.broadcasts(),
            usize::from(stalled == "broadcast_tx_commit")
        );
        server.stop().await;
    }
}

#[tokio::test]
async fn disconnects_and_redirects_never_resubmit_or_satisfy_rejection_assertions() {
    for redirect in [false, true] {
        let server = Server::new(move |req| {
            if req["method"] == "broadcast_tx_commit" {
                if redirect {
                    Reply::Redirect
                } else {
                    Reply::Disconnect
                }
            } else {
                Reply::Json(normal(req))
            }
        })
        .await;
        let err = server.client().transact(&key(), message).await.unwrap_err();
        assert!(matches!(err, ProcessError::Broadcast { .. }), "{err}");
        assert_eq!(server.broadcasts(), 1);
        server.stop().await;
    }
}

#[test]
fn top_level_response_not_nested_event_controls_address() {
    let address = fixtures()[0]["address"].as_str().unwrap().to_string();
    let receipt = Receipt {
        hash: String::new(),
        height: 12,
        code: 0,
        codespace: String::new(),
        log: String::new(),
        gas_used: 1,
        gas_wanted: 1,
        events: serde_json::from_value(json!([{"type":"instantiate", "attributes":[
            {"key":"_contract_address","value":"nested-not-the-top-level-address","index":true}
        ]}]))
        .unwrap(),
        data: tx_response(
            "/cosmwasm.wasm.v1.MsgInstantiateContractResponse",
            MsgInstantiateContractResponse {
                address: address.clone(),
                data: vec![],
            },
        ),
    };
    let res: MsgInstantiateContractResponse = receipt
        .message_response("/cosmwasm.wasm.v1.MsgInstantiateContractResponse")
        .unwrap();
    assert_eq!(res.address, address);
    assert!(receipt
        .message_response::<MsgStoreCodeResponse>("/cosmwasm.wasm.v1.MsgStoreCodeResponse")
        .is_err());
    let empty = Receipt {
        data: vec![],
        ..receipt
    };
    assert!(empty
        .message_response::<MsgInstantiateContractResponse>(
            "/cosmwasm.wasm.v1.MsgInstantiateContractResponse"
        )
        .is_err());
}

#[tokio::test]
async fn node_and_account_validation_and_simulation_failure_happen_before_submission() {
    for case in [
        "chain",
        "version",
        "account-type",
        "account-address",
        "account-missing",
        "gas-missing",
        "query-malformed",
        "simulation-rejection",
    ] {
        let server = Server::new(move |req| {
            let mut value = normal(req);
            if req["method"] == "status" {
                if case == "chain" {
                    value["node_info"]["network"] = json!("wrong-chain");
                }
                if case == "version" {
                    value["node_info"]["version"] = json!("0.39.0");
                }
            }
            let path = req["params"]["path"].as_str().unwrap_or("");
            if path.ends_with("/Account") {
                if case == "account-missing" {
                    value = query_result(QueryAccountResponse { account: None }.encode_to_vec());
                }
                if case == "query-malformed" {
                    value = query_result(vec![255]);
                }
                if case == "account-type" || case == "account-address" {
                    value = query_result(
                        QueryAccountResponse {
                            account: Some(Any {
                                type_url: if case == "account-type" {
                                    "/unsupported"
                                } else {
                                    "/cosmos.auth.v1beta1.BaseAccount"
                                }
                                .into(),
                                value: BaseAccount {
                                    address: "different".into(),
                                    ..Default::default()
                                }
                                .encode_to_vec(),
                            }),
                        }
                        .encode_to_vec(),
                    );
                }
            }
            if path.ends_with("/Simulate") {
                if case == "gas-missing" {
                    value = query_result(SimulateResponse::default().encode_to_vec());
                }
                if case == "simulation-rejection" {
                    value["response"]["code"] = json!(18);
                    value["response"]["log"] = json!("unauthorized");
                }
            }
            Reply::Json(value)
        })
        .await;
        let err = server.client().transact(&key(), message).await.unwrap_err();
        if case == "simulation-rejection" {
            assert!(matches!(
                err,
                ProcessError::CosmwasmError(CosmwasmError::TxError(TxError::Simulation {
                    code: 18,
                    ..
                }))
            ));
        } else {
            assert!(
                !matches!(err, ProcessError::CosmwasmError(_)),
                "{case}: {err}"
            );
        }
        assert_eq!(server.broadcasts(), 0, "{case}");
        server.stop().await;
    }
}

#[tokio::test]
async fn block_polling_uses_chain_height_time_and_caller_deadline() {
    use std::sync::atomic::{AtomicU64, Ordering};
    let count = Arc::new(AtomicU64::new(0));
    let seen = Arc::clone(&count);
    let server = Server::new(move |req| {
        assert_eq!(req["method"], "status");
        let mut value = status();
        value["sync_info"]["latest_block_height"] =
            json!(seen.fetch_add(1, Ordering::SeqCst).to_string());
        Reply::Json(value)
    })
    .await;
    server
        .client()
        .poll_for_n_blocks(2, Duration::from_secs(5), true)
        .await
        .unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 4); // zero -> first block 1 -> target 3
    assert_eq!(server.broadcasts(), 0);
    server.stop().await;

    let count = Arc::new(AtomicU64::new(0));
    let seen = Arc::clone(&count);
    let server = Server::new(move |req| {
        assert_eq!(req["method"], "status");
        let mut value = status();
        let seconds = if seen.fetch_add(1, Ordering::SeqCst) == 0 {
            "00"
        } else {
            "10"
        };
        value["sync_info"]["latest_block_time"] = json!(format!("2024-01-01T00:00:{seconds}Z"));
        Reply::Json(value)
    })
    .await;
    // Ten chain seconds advance without waiting ten wall-clock seconds.
    server
        .client()
        .poll_for_n_secs(10, Duration::from_secs(1))
        .await
        .unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 2);
    let result = server
        .client()
        .poll_for_n_blocks(1, Duration::from_millis(20), false)
        .await;
    assert!(matches!(result, Err(ProcessError::Timeout { hash: None })));
    server.stop().await;
}

#[tokio::test]
async fn synchronous_facade_registers_and_profiles_only_confirmed_matching_storage() {
    use crate::ChainClient;
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    for case in ["success", "check-rejected", "wrong-checksum"] {
        let server = Server::new(move |req| {
            let mut value = normal(req);
            if req["method"] == "broadcast_tx_commit" {
                if case == "check-rejected" {
                    value["check_tx"]["code"] = json!(9);
                }
                if case == "wrong-checksum" {
                    value["deliver_tx"]["data"] = json!(STANDARD.encode(tx_response(
                        "/cosmwasm.wasm.v1.MsgStoreCodeResponse",
                        MsgStoreCodeResponse {
                            code_id: 42,
                            checksum: vec![1; 32]
                        },
                    )));
                }
            }
            Reply::Json(value)
        })
        .await;
        let mut cfg = config();
        cfg.chain_cfg.rpc_endpoint = Some(server.endpoint.clone());
        tokio::task::spawn_blocking(move || {
            let directory = std::env::temp_dir().join(format!(
                "dao-chain-client-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::SeqCst)
            ));
            std::fs::create_dir(&directory).unwrap();
            let cleanup = Cleanup(directory);
            std::fs::write(cleanup.0.join("token-default.wasm"), WASM).unwrap();
            let mut client = ChainClient::new(cfg, true).unwrap();
            let result = client.store_contracts(cleanup.0.to_str().unwrap(), &key(), None);
            if case == "success" {
                assert_eq!(result.unwrap().len(), 1);
                assert_eq!(client.contract_map.code_id("token-default").unwrap(), 42);
                assert_eq!(
                    client.gas_profiler_report().unwrap()["token-default"]["Store__Store"].gas_used,
                    20001
                );
            } else {
                assert!(result.is_err());
                assert!(client.contract_map.deploy_info().is_empty());
                assert!(client.gas_profiler_report().unwrap().is_empty());
            }
        })
        .await
        .unwrap();
        assert_eq!(server.broadcasts(), 1);
        server.stop().await;
    }
}
