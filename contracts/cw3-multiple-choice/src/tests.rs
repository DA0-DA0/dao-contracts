use crate::{
    msg::{ExecuteMsg, ProposeMsg, QueryMsg},
    query::ConfigResponse,
};
use cosmwasm_std::{coins, from_slice, Addr, BankMsg, Coin, Decimal, Empty, Uint128};
use cw2::{query_contract_info, ContractVersion};
use cw20::Cw20Coin;
use cw_multi_test::{App, BankSudo, Contract, ContractWrapper, Executor, SudoMsg};
use cw_utils::Duration;

use crate::{
    constants::CONFIG_KEY,
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    msg::{InstantiateMsg, Threshold},
    state::Config,
};

const SOMEBODY: &str = "somebody";
const NATIVE_TOKEN_DENOM: &str = "ustars";

#[test]
fn test_instantiate_success() {
    let mut app = App::default();

    let max_voting_period = Duration::Time(1234567);

    let threshold = Threshold::ThresholdQuorum {
        threshold: Decimal::percent(51),
        quorum: Decimal::percent(10),
    };

    let instantiate_msg = InstantiateMsg {
        threshold: threshold.clone(),
        max_voting_period: max_voting_period,
        proposal_deposit_amount: Uint128::new(150),
        gov_token_address: "gov_token_address".to_string(),
        refund_failed_proposals: Some(true),
        parent_dao_contract_address: "parent_address".to_string(),
    };

    let code_id = app.store_code(contract_proposal());

    // validate instantiate succeeds
    let proposal_contract = app.instantiate_contract(
        code_id,
        Addr::unchecked("owner"),
        &instantiate_msg,
        &[],
        "label",
        None,
    );

    assert!(proposal_contract.is_ok());
    let contract_address = proposal_contract.unwrap();

    // Verify contract version set properly
    let version = query_contract_info(&app, contract_address.clone()).unwrap();
    assert_eq!(
        ContractVersion {
            contract: CONTRACT_NAME.to_string(),
            version: CONTRACT_VERSION.to_string(),
        },
        version,
    );

    // Verify config set properly
    let res = app
        .wrap()
        .query_wasm_raw(contract_address, CONFIG_KEY.as_bytes());

    assert!(res.is_ok());
    let opt = res.unwrap();
    assert!(opt.is_some());
    let bytes = opt.unwrap();
    let des = from_slice::<Config>(&bytes);
    assert!(des.is_ok());
    let cfg = des.unwrap();

    // Check fields in config
    assert_eq!(
        cfg.parent_dao_contract_address.to_string(),
        "parent_address"
    );

    assert_eq!(cfg.gov_token_address.to_string(), "gov_token_address");
    assert_eq!(cfg.max_voting_period, max_voting_period);
    assert_eq!(cfg.proposal_deposit, Uint128::new(150));
    assert_eq!(cfg.refund_failed_proposals, Some(true));
    assert_eq!(cfg.threshold, threshold);
}

#[test]
fn test_vote_success() {
    let mut app = App::default();

    let max_voting_period = Duration::Time(1234567);

    let threshold = Threshold::ThresholdQuorum {
        threshold: Decimal::percent(51),
        quorum: Decimal::percent(10),
    };

    let parent_dao_contract_address = "parent_address".to_string();

    let instantiate_msg = InstantiateMsg {
        threshold: threshold.clone(),
        max_voting_period: max_voting_period,
        proposal_deposit_amount: Uint128::new(150),
        gov_token_address: "gov_token_address".to_string(),
        refund_failed_proposals: Some(true),
        parent_dao_contract_address: parent_dao_contract_address.clone(),
    };

    let code_id = app.store_code(contract_proposal());

    let proposal_contract = app.instantiate_contract(
        code_id,
        Addr::unchecked("owner"),
        &instantiate_msg,
        &[],
        "label",
        None,
    );

    assert!(proposal_contract.is_ok());
    let contract_address = proposal_contract.unwrap();

    let bank_msg = BankMsg::Send {
        to_address: SOMEBODY.into(),
        amount: coins(1, NATIVE_TOKEN_DENOM),
    };
    let mut msgs = Vec::new();
    msgs.push(vec![bank_msg.into()]);
    let title = "title".to_string();
    let description = "description".to_string();
    let choices = vec!["pay somebody money".to_string()];

    let proposal = ExecuteMsg::Propose(ProposeMsg {
        title,
        description,
        choices,
        msgs,
        latest: None,
    });

    let res = app.execute_contract(Addr::unchecked("OWNER"), contract_address, &proposal, &[]);

    // assert!(res.is_ok());
}

fn contract_proposal() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}
