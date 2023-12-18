#![allow(unused_must_use)]

use localic_std::modules::cosmwasm::CosmWasm;
use reqwest::blocking::Client;
use serde_json::json;

use localic_std::polling::*;
use localic_std::transactions::*;

const API_URL: &str = "http://127.0.0.1:8080";

// ICTEST_HOME=./local-interchain local-ic start juno --api-port 8080
fn main() {
    let client = Client::new();
    poll_for_start(&client.clone(), &API_URL, 150);

    let rb: ChainRequestBuilder = match ChainRequestBuilder::new(API_URL.to_string(), "localjuno-1".to_string(), true) {
        Ok(rb) => rb,
        Err(err) => {
            println!("err: {}", err);
            return;
        }
    };

    test_cosmos_staking(&rb);
}

fn test_cosmos_staking(rb: &ChainRequestBuilder) {
    let mut cw = CosmWasm::new(&rb);

    let file_path = get_contract_path().join("dao-voting-cosmos-staking.wasm");
    let code_id = cw.store("acc0", &file_path);
    if code_id.is_err() {
        panic!("code_id error: {:?}", code_id);
    }

    let code_id = code_id.unwrap_or_default();
    if code_id == 0 {
        panic!("code_id is 0");
    }

    // print code_id
    println!("code_id: {}", code_id);

    // let msg = r#"{"count":0}"#;
    // let res = cw.instantiate(
    //     "acc0",
    //     msg,
    //     "my-label",
    //     Some("juno1hj5fveer5cjtn4wd6wstzugjfdxzl0xps73ftl"),
    //     "",
    // );
    // println!("res: {:?}", res);

    // let prev_res = cw.query( r#"{"get_count":{}}"#);
    // assert_eq!(prev_res, json!({"data":{"count":0}}));
    // println!("prev_res: {}", prev_res);

    // let data = cw.execute("acc0", r#"{"increment":{}}"#, "--gas=auto --gas-adjustment=2.0");
    // println!("unwrap: {}", data.unwrap());

    // let updated_res = cw.query(r#"{"get_count":{}}"#);
    // assert_eq!(updated_res, json!({"data":{"count":1}}));
    // println!("updated_res: {}", updated_res);
}

fn parent_dir() -> std::path::PathBuf {
    return std::env::current_dir().unwrap().parent().unwrap().to_path_buf();
}

// get_contract_path returns the artifacts dire from parent_dir
fn get_contract_path() -> std::path::PathBuf {
    parent_dir().join("artifacts")
}