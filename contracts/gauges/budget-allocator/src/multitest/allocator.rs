use cosmwasm_std::{coin, BankMsg, CosmosMsg, Decimal};

use crate::{
    msg::{AllOptionsResponse, CheckOptionResponse, ExecuteMsg, QueryMsg, SampleGaugeMsgsResponse},
    multitest::suite::{addr, raw, ujuno, Suite},
    state::Config,
    ContractError,
};

#[test]
fn instantiate_requires_at_least_one_option() {
    use cw_multi_test::{App, Executor};

    let mut app = App::default();
    let admin = addr("admin");
    let code_id = app.store_code(crate::multitest::suite::contract());

    let err = app
        .instantiate_contract(
            code_id,
            admin.clone(),
            &crate::msg::InstantiateMsg {
                admin: admin.to_string(),
                options: vec![],
                epoch_budget: ujuno(1_000),
            },
            &[],
            "no-options",
            Some(admin.to_string()),
        )
        .unwrap_err();
    assert_eq!(ContractError::NoOptions {}, err.downcast().unwrap());
}

#[test]
fn happy_path_options_and_budget() {
    let suite = Suite::new(&["alice", "bob"], ujuno(1_000));

    let opts: AllOptionsResponse = suite.query(&QueryMsg::AllOptions {}).unwrap();
    assert_eq!(opts.options.len(), 2);
    assert!(opts.options.contains(&"alice".to_string()));

    let valid: CheckOptionResponse = suite
        .query(&QueryMsg::CheckOption {
            option: "alice".to_string(),
        })
        .unwrap();
    assert!(valid.valid);

    let invalid: CheckOptionResponse = suite
        .query(&QueryMsg::CheckOption {
            option: "stranger".to_string(),
        })
        .unwrap();
    assert!(!invalid.valid);

    let cfg: Config = suite.query(&QueryMsg::Config {}).unwrap();
    assert_eq!(cfg.epoch_budget, ujuno(1_000));
}

#[test]
fn admin_can_add_and_remove_options() {
    let mut suite = Suite::new(&["alice"], ujuno(1_000));

    suite
        .execute_admin(&ExecuteMsg::AddOption {
            option: "bob".to_string(),
        })
        .unwrap();
    let opts: AllOptionsResponse = suite.query(&QueryMsg::AllOptions {}).unwrap();
    assert_eq!(opts.options.len(), 2);

    suite
        .execute_admin(&ExecuteMsg::RemoveOption {
            option: "alice".to_string(),
        })
        .unwrap();
    let opts: AllOptionsResponse = suite.query(&QueryMsg::AllOptions {}).unwrap();
    assert_eq!(opts.options, vec!["bob"]);
}

#[test]
fn add_option_rejects_duplicates() {
    let mut suite = Suite::new(&["alice"], ujuno(1_000));
    let err = suite
        .execute_admin(&ExecuteMsg::AddOption {
            option: "alice".to_string(),
        })
        .unwrap_err();
    assert_eq!(
        ContractError::OptionAlreadyExists("alice".to_string()),
        err.downcast().unwrap(),
    );
}

#[test]
fn remove_option_rejects_missing() {
    let mut suite = Suite::new(&["alice"], ujuno(1_000));
    let err = suite
        .execute_admin(&ExecuteMsg::RemoveOption {
            option: "ghost".to_string(),
        })
        .unwrap_err();
    assert_eq!(
        ContractError::OptionDoesNotExist("ghost".to_string()),
        err.downcast().unwrap(),
    );
}

#[test]
fn non_admin_cannot_mutate() {
    let mut suite = Suite::new(&["alice"], ujuno(1_000));
    let intruder = addr("intruder");

    let err = suite
        .execute_as(
            &intruder,
            &ExecuteMsg::AddOption {
                option: "bob".to_string(),
            },
        )
        .unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());

    let err = suite
        .execute_as(
            &intruder,
            &ExecuteMsg::UpdateBudget {
                epoch_budget: ujuno(999),
            },
        )
        .unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());
}

#[test]
fn update_budget_works() {
    let mut suite = Suite::new(&["alice"], ujuno(1_000));
    suite
        .execute_admin(&ExecuteMsg::UpdateBudget {
            epoch_budget: ujuno(2_500),
        })
        .unwrap();
    let cfg: Config = suite.query(&QueryMsg::Config {}).unwrap();
    assert_eq!(cfg.epoch_budget, ujuno(2_500));
}

#[test]
fn sample_gauge_msgs_distributes_proportionally() {
    let suite = Suite::new(&["alice", "bob", "carol"], ujuno(10_000));

    let selected = vec![
        ("alice".to_string(), Decimal::percent(50)),
        ("bob".to_string(), Decimal::percent(30)),
        ("carol".to_string(), Decimal::percent(20)),
    ];

    let res: SampleGaugeMsgsResponse = suite
        .query(&QueryMsg::SampleGaugeMsgs { selected })
        .unwrap();
    assert_eq!(res.execute.len(), 3);
    assert_eq!(
        res.execute,
        [
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "alice".to_string(),
                amount: vec![coin(5_000, "ujuno")],
            }),
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "bob".to_string(),
                amount: vec![coin(3_000, "ujuno")],
            }),
            CosmosMsg::Bank(BankMsg::Send {
                to_address: "carol".to_string(),
                amount: vec![coin(2_000, "ujuno")],
            }),
        ]
    );
}

#[test]
fn sample_gauge_msgs_floors_when_weight_does_not_divide_evenly() {
    // 1_000 / 3 → 333 each via flooring; total spend = 999.
    let suite = Suite::new(&["alice", "bob", "carol"], ujuno(1_000));
    let third = Decimal::from_ratio(1u128, 3u128);
    let selected = vec![
        ("alice".to_string(), third),
        ("bob".to_string(), third),
        ("carol".to_string(), third),
    ];
    let res: SampleGaugeMsgsResponse = suite
        .query(&QueryMsg::SampleGaugeMsgs { selected })
        .unwrap();
    for msg in &res.execute {
        match msg {
            CosmosMsg::Bank(BankMsg::Send { amount, .. }) => {
                assert_eq!(amount[0].amount, raw(333));
            }
            _ => panic!("unexpected msg variant"),
        }
    }
}
