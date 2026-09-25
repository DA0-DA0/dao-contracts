//! Opt-in compatibility pilot. Never runs in ordinary unit tests or on a public
//! endpoint. Existing full integration tests remain the delivery requirement.
use std::path::PathBuf;

use cosmrs::proto::{
    cosmos::{
        bank::v1beta1::{QueryBalanceRequest, QueryBalanceResponse},
        staking::v1beta1::{QueryValidatorsRequest, QueryValidatorsResponse},
    },
    cosmwasm::wasm::v1::{
        MsgExecuteContract, MsgInstantiateContract, MsgInstantiateContractResponse, MsgStoreCode,
        MsgStoreCodeResponse, QuerySmartContractStateRequest, QuerySmartContractStateResponse,
    },
};
use serde_json::{json, Value};

use super::*;
use crate::{error::CosmwasmError, Config, Key};

fn report(stage: &str, receipt: &Receipt) {
    println!(
        "{}",
        json!({"stage":stage,"hash":receipt.hash,"height":receipt.height,
        "code":receipt.code,"gas_wanted":receipt.gas_wanted,"gas_used":receipt.gas_used,
        "events":receipt.events.len()})
    );
}

async fn members(rpc: &Rpc, address: &str) -> Value {
    let result: QuerySmartContractStateResponse = rpc
        .query(
            "/cosmwasm.wasm.v1.Query/SmartContractState",
            QuerySmartContractStateRequest {
                address: address.into(),
                query_data: br#"{"list_members":{}}"#.to_vec(),
            },
        )
        .await
        .unwrap();
    serde_json::from_slice(&result.data).unwrap()
}

/// Opt-in full-storage reproduction of the CI setup path. Like the pilot, it
/// never runs in ordinary unit tests and only targets an explicit throwaway
/// loopback endpoint.
// Synchronous on purpose: this mirrors the integration harness code path.
#[test]
#[ignore = "requires explicit throwaway pinned Juno endpoint and retained CI artifacts"]
fn pinned_juno_store_all() {
    let endpoint =
        std::env::var("PILOT_RPC").expect("PILOT_RPC must name the disposable loopback node");
    assert!(
        endpoint.starts_with("http://127.0.0.1:"),
        "pilot is restricted to loopback"
    );
    let artifacts =
        PathBuf::from(std::env::var("PILOT_ARTIFACT_DIR").expect("retained CI artifacts required"));
    let mut cfg: Config =
        serde_yaml::from_str(include_str!("../../../configs/cosm-orc/ci.yaml")).unwrap();
    cfg.chain_cfg.rpc_endpoint = Some(endpoint);
    let accounts: Vec<Value> =
        serde_json::from_str(include_str!("../../../configs/test_accounts.json")).unwrap();
    let key = SigningKey {
        name: accounts[0]["name"].as_str().unwrap().into(),
        key: Key::Mnemonic(accounts[0]["mnemonic"].as_str().unwrap().into()),
        derivation_path: cfg.chain_cfg.derivation_path.clone(),
    };
    let mut client = crate::ChainClient::new(cfg, true).unwrap();
    client
        .poll_for_n_blocks(1, Duration::from_secs(20), true)
        .unwrap();
    let mut names = std::fs::read_dir(&artifacts)
        .unwrap()
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()
        .unwrap();
    names.sort();
    names.retain(|path| {
        path.extension()
            .map(|extension| extension == "wasm")
            .unwrap_or(false)
    });
    let started = std::time::Instant::now();
    let result = client.store_contracts(artifacts.to_str().unwrap(), &key, None);
    match &result {
        Ok(stored) => println!(
            "{}",
            json!({"stage":"store-all-complete","stored":stored.len(),
            "expected":names.len(),"seconds":started.elapsed().as_secs(),
            "registry":client.contract_map.deploy_info().len()})
        ),
        Err(error) => println!(
            "{}",
            json!({"stage":"store-all-failed","expected":names.len(),
            "registry":client.contract_map.deploy_info().len(),"seconds":started.elapsed().as_secs(),
            "error":error.to_string()})
        ),
    }
    result.unwrap();
    assert_eq!(client.contract_map.deploy_info().len(), names.len());
}

#[tokio::test]
#[ignore = "requires explicit throwaway pinned Juno endpoint and retained CI artifacts"]
async fn pinned_juno() {
    let endpoint =
        std::env::var("PILOT_RPC").expect("PILOT_RPC must name the disposable loopback node");
    assert!(
        endpoint.starts_with("http://127.0.0.1:"),
        "pilot is restricted to loopback"
    );
    let artifacts =
        PathBuf::from(std::env::var("PILOT_ARTIFACT_DIR").expect("retained CI artifacts required"));
    let mut cfg: Config =
        serde_yaml::from_str(include_str!("../../../configs/cosm-orc/ci.yaml")).unwrap();
    cfg.chain_cfg.rpc_endpoint = Some(endpoint);
    let accounts: Vec<Value> =
        serde_json::from_str(include_str!("../../../configs/test_accounts.json")).unwrap();
    let key = |i: usize| SigningKey {
        name: accounts[i]["name"].as_str().unwrap().into(),
        key: Key::Mnemonic(accounts[i]["mnemonic"].as_str().unwrap().into()),
        derivation_path: cfg.chain_cfg.derivation_path.clone(),
    };
    let owner = key(0);
    let other = key(1);
    let owner_addr = owner.address(&cfg.chain_cfg.prefix).unwrap();
    let other_addr = other.address(&cfg.chain_cfg.prefix).unwrap();
    let rpc = Rpc::new(cfg.chain_cfg.clone()).unwrap();
    let node = rpc.status().await.unwrap();
    println!(
        "{}",
        json!({"stage":"node", "version":node.node_info.version.to_string(),
        "chain_id":node.node_info.network.to_string(), "height":node.sync_info.latest_block_height.value()})
    );
    rpc.poll_for_n_blocks(1, Duration::from_secs(20), true)
        .await
        .unwrap();
    rpc.poll_for_n_secs(1, Duration::from_secs(20))
        .await
        .unwrap();
    for (i, account) in accounts.iter().enumerate() {
        let addr = key(i).address(&cfg.chain_cfg.prefix).unwrap();
        assert_eq!(addr.as_ref(), account["address"].as_str().unwrap());
        rpc.account(&addr).await.unwrap();
    }
    let balance: QueryBalanceResponse = rpc
        .query(
            "/cosmos.bank.v1beta1.Query/Balance",
            QueryBalanceRequest {
                address: owner_addr.to_string(),
                denom: cfg.chain_cfg.denom.clone(),
            },
        )
        .await
        .unwrap();
    assert!(balance.balance.unwrap().amount.parse::<u128>().unwrap() > 0);
    let validators: QueryValidatorsResponse = rpc
        .query(
            "/cosmos.staking.v1beta1.Query/Validators",
            QueryValidatorsRequest {
                status: "BOND_STATUS_BONDED".into(),
                pagination: None,
            },
        )
        .await
        .unwrap();
    assert!(!validators.validators.is_empty());
    println!(
        "{}",
        json!({"stage":"queries-and-polling", "fixture_accounts":accounts.len(),"bonded_validators":validators.validators.len()})
    );

    let wasm = std::fs::read(artifacts.join("cw4_group.wasm")).unwrap();
    let checksum = transaction_hash(&wasm).to_string().to_lowercase();
    let manifest = std::fs::read_to_string(artifacts.join("checksums.txt")).unwrap();
    assert!(manifest.lines().any(|line| {
        let mut fields = line.split_whitespace();
        fields.next() == Some(checksum.as_str()) && fields.next() == Some("cw4_group.wasm")
    }));
    let stored = rpc
        .transact(&owner, |sender| {
            Ok(Any {
                type_url: "/cosmwasm.wasm.v1.MsgStoreCode".into(),
                value: MsgStoreCode {
                    sender: sender.to_string(),
                    wasm_byte_code: wasm,
                    instantiate_permission: None,
                }
                .encode_to_vec(),
            })
        })
        .await
        .unwrap();
    report("store", &stored);
    let code: MsgStoreCodeResponse = stored
        .message_response("/cosmwasm.wasm.v1.MsgStoreCodeResponse")
        .unwrap();
    assert!(code.code_id > 0);
    assert_eq!(
        code.checksum,
        Sha256::digest(std::fs::read(artifacts.join("cw4_group.wasm")).unwrap()).to_vec()
    );
    let instantiated = rpc
        .transact(&owner, |sender| {
            Ok(Any {
                type_url: "/cosmwasm.wasm.v1.MsgInstantiateContract".into(),
                value: MsgInstantiateContract {
                    sender: sender.to_string(),
                    admin: String::new(),
                    code_id: code.code_id,
                    label: "cosm-orc".into(),
                    msg: serde_json::to_vec(&json!({"admin":owner_addr.to_string(),"members":[]}))?,
                    funds: vec![],
                }
                .encode_to_vec(),
            })
        })
        .await
        .unwrap();
    report("instantiate", &instantiated);
    let contract: MsgInstantiateContractResponse = instantiated
        .message_response("/cosmwasm.wasm.v1.MsgInstantiateContractResponse")
        .unwrap();
    assert!(contract.address.starts_with("juno1"));
    assert_eq!(members(&rpc, &contract.address).await["members"], json!([]));
    let update =
        json!({"update_members":{"remove":[],"add":[{"addr":other_addr.to_string(),"weight":7}]}});
    let execute_message = |sender: &AccountId| -> Result<Any> {
        Ok(Any {
            type_url: "/cosmwasm.wasm.v1.MsgExecuteContract".into(),
            value: MsgExecuteContract {
                sender: sender.to_string(),
                contract: contract.address.clone(),
                msg: serde_json::to_vec(&update)?,
                funds: vec![],
            }
            .encode_to_vec(),
        })
    };
    let executed = rpc.transact(&owner, execute_message).await.unwrap();
    report("execute", &executed);
    let expected = json!([{"addr":other_addr.to_string(),"weight":7}]);
    assert_eq!(members(&rpc, &contract.address).await["members"], expected);
    let before = rpc.account(&other_addr).await.unwrap();
    let rejected = rpc.transact(&other, execute_message).await.unwrap_err();
    assert!(
        matches!(
            rejected,
            ProcessError::CosmwasmError(CosmwasmError::TxError(TxError::Simulation { .. }))
        ),
        "{rejected}"
    );
    assert_eq!(
        rpc.account(&other_addr).await.unwrap().sequence,
        before.sequence
    );
    println!(
        "{}",
        json!({"stage":"simulation-rejection","error":rejected.to_string()})
    );

    // Fault injection ONLY in this test: bypass preflight so an unauthorized
    // execute reaches DeliverTx. Production transact always simulates. Derive
    // its fee using the unchanged formula and the preceding successful execute's
    // actual gas; do not raise fixture/block limits or retry the submission.
    let failed = timeout(OPERATION_TIMEOUT, async {
        let (signer, address) = other.derive(&cfg.chain_cfg.prefix).unwrap();
        let account = rpc.account(&address).await.unwrap();
        let bytes = signing::sign(
            &signer,
            vec![execute_message(&address).unwrap()],
            account.account_number,
            account.sequence,
            signing::fee(executed.gas_used, &cfg.chain_cfg).unwrap(),
            &cfg.chain_cfg.chain_id,
        )
        .unwrap();
        let hash = transaction_hash(&bytes);
        let result = rpc.client.broadcast_tx_commit(bytes).await.unwrap();
        assert!(result.check_tx.code.is_ok());
        assert!(result.tx_result.code.is_err());
        committed(result, hash).unwrap_err()
    })
    .await
    .unwrap();
    match failed {
        ProcessError::CosmwasmError(CosmwasmError::TxError(TxError::Execution {
            response,
            ..
        })) => report("execution-rejection", &response),
        other => panic!("not a committed execution rejection: {other}"),
    }
    assert_eq!(
        rpc.account(&other_addr).await.unwrap().sequence,
        before.sequence + 1
    );
    assert_eq!(members(&rpc, &contract.address).await["members"], expected);
    println!(
        "{}",
        json!({"stage":"pilot-complete","code_id":code.code_id,"contract":contract.address,
        "artifact_sha256":checksum})
    );
}
