use cosmwasm_std::{coin, to_json_binary, Coin, CosmosMsg, Decimal, Uint128, WasmMsg};
use cw20::Cw20ExecuteMsg;
use cw_denom::UncheckedDenom;

use crate::{
    msg::{
        AdapterQueryMsg, AllSubmissionsResponse, AssetUnchecked, ExecuteMsg, ReceiveMsg,
        SampleGaugeMsgsResponse, SubmissionResponse,
    },
    multitest::suite::{addr, submit_cw20_create, Suite},
    ContractError,
};

#[test]
fn create_default_submission() {
    let suite = Suite::new_native(None);
    let community_pool = suite.community_pool.clone();

    // Created during instantiation.
    let res: SubmissionResponse = suite
        .query(&AdapterQueryMsg::Submission {
            address: community_pool.to_string(),
        })
        .unwrap();
    assert_eq!(
        res,
        SubmissionResponse {
            sender: suite.adapter.clone(),
            name: "Unimpressed".to_owned(),
            url: "Those funds go back to the community pool".to_owned(),
            address: community_pool,
        },
    );
}

#[test]
fn create_submission_no_required_deposit() {
    let mut suite = Suite::new_native(None);
    let owner = suite.owner.clone();
    let recipient = addr("recipient");
    suite.mint_native(&owner, coin(1_000, "juno"));

    // Sending funds when no deposit required is an error.
    let err = suite
        .create_submission(&owner, &recipient, Some(coin(1_000, "juno")))
        .unwrap_err();
    assert_eq!(
        ContractError::InvalidDepositAmount {
            correct_amount: Uint128::zero(),
        },
        err.downcast().unwrap()
    );

    // Without funds it succeeds.
    suite.create_submission(&owner, &recipient, None).unwrap();

    let res: SubmissionResponse = suite
        .query(&AdapterQueryMsg::Submission {
            address: recipient.to_string(),
        })
        .unwrap();
    assert_eq!(
        res,
        SubmissionResponse {
            sender: owner,
            name: "DAOers".to_owned(),
            url: "https://daodao.zone".to_owned(),
            address: recipient,
        },
    );
}

#[test]
fn overwrite_existing_submission() {
    let mut suite = Suite::new_native(None);
    let owner = suite.owner.clone();
    let recipient = addr("recipient");

    suite.create_submission(&owner, &recipient, None).unwrap();

    let res: SubmissionResponse = suite
        .query(&AdapterQueryMsg::Submission {
            address: recipient.to_string(),
        })
        .unwrap();
    assert_eq!(res.sender, owner);
    assert_eq!(res.url, "https://daodao.zone");

    // Submitting to the same recipient as a different sender is not allowed.
    let intruder = addr("intruder");
    let err = suite
        .create_submission(&intruder, &recipient, None)
        .unwrap_err();
    assert_eq!(
        ContractError::UnauthorizedSubmission {},
        err.downcast().unwrap()
    );

    // Overwriting as the original author works.
    suite.create_submission(&owner, &recipient, None).unwrap();
}

#[test]
fn create_submission_required_deposit() {
    let mut suite = Suite::new_native(Some(AssetUnchecked {
        denom: UncheckedDenom::Native("juno".into()),
        amount: Uint128::new(1_000),
    }));
    let owner = suite.owner.clone();
    let recipient = addr("recipient");
    suite.mint_native(&owner, coin(1_000, "wynd"));
    suite.mint_native(&owner, coin(1_000, "juno"));

    // No funds → PaymentError.
    let err = suite
        .create_submission(&owner, &recipient, None)
        .unwrap_err();
    assert_eq!(
        ContractError::PaymentError(cw_utils::PaymentError::NoFunds {}),
        err.downcast().unwrap()
    );

    // Right denom, wrong amount.
    let err = suite
        .create_submission(
            &owner,
            &recipient,
            Some(Coin {
                denom: "juno".into(),
                amount: Uint128::new(999),
            }),
        )
        .unwrap_err();
    assert_eq!(
        ContractError::InvalidDepositAmount {
            correct_amount: Uint128::new(1_000),
        },
        err.downcast().unwrap()
    );

    // Wrong denom, right amount.
    let err = suite
        .create_submission(
            &owner,
            &recipient,
            Some(Coin {
                denom: "wynd".into(),
                amount: Uint128::new(1_000),
            }),
        )
        .unwrap_err();
    assert_eq!(
        ContractError::InvalidDepositType {},
        err.downcast().unwrap()
    );

    // Valid submission.
    suite
        .create_submission(&owner, &recipient, Some(coin(1_000, "juno")))
        .unwrap();

    let res: SubmissionResponse = suite
        .query(&AdapterQueryMsg::Submission {
            address: recipient.to_string(),
        })
        .unwrap();
    assert_eq!(res.sender, owner);
    assert_eq!(res.address, recipient);
}

#[test]
fn create_receive_required_deposit() {
    let (mut suite, deposit_cw20) = Suite::new_cw20_deposit();
    let owner = suite.owner.clone();
    let recipient_addr = owner.clone();

    // A second cw20 for the "wrong cw20" error path.
    let bad_cw20 = suite.instantiate_cw20();

    let binary_msg = to_json_binary(&ReceiveMsg::CreateSubmission {
        name: "DAOers".to_string(),
        url: "https://daodao.zone".to_string(),
        address: recipient_addr.to_string(),
    })
    .unwrap();

    // Sending from the wrong cw20 fails (we impersonate the cw20 contract).
    let err = suite
        .execute(
            &bad_cw20,
            &ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
                sender: recipient_addr.to_string(),
                amount: Uint128::new(1_000),
                msg: binary_msg.clone(),
            }),
            &[],
        )
        .unwrap_err();
    assert_eq!(
        ContractError::InvalidDepositType {},
        err.downcast().unwrap(),
    );

    // Right cw20 but less than required fails.
    let err = suite
        .execute(
            &deposit_cw20,
            &ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
                sender: recipient_addr.to_string(),
                amount: Uint128::new(999),
                msg: binary_msg.clone(),
            }),
            &[],
        )
        .unwrap_err();
    assert_eq!(
        ContractError::InvalidDepositAmount {
            correct_amount: Uint128::new(1_000),
        },
        err.downcast().unwrap()
    );

    // Valid submission via correct cw20.
    suite
        .execute(
            &deposit_cw20,
            &ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
                sender: recipient_addr.to_string(),
                amount: Uint128::new(1_000),
                msg: binary_msg,
            }),
            &[],
        )
        .unwrap();

    let all: AllSubmissionsResponse = suite.query(&AdapterQueryMsg::AllSubmissions {}).unwrap();
    // default (community-pool refund) + the one we just added.
    assert_eq!(all.submissions.len(), 2);
}

#[test]
fn return_deposits_no_required_deposit() {
    let mut suite = Suite::new_native(None);
    let err = suite
        .execute_owner(&ExecuteMsg::ReturnDeposits {})
        .unwrap_err();
    assert_eq!(ContractError::NoDepositToRefund {}, err.downcast().unwrap());
}

#[test]
fn return_deposits_no_admin() {
    let mut suite = Suite::new_native(Some(AssetUnchecked {
        denom: UncheckedDenom::Native("juno".into()),
        amount: Uint128::new(1_000),
    }));
    let intruder = addr("intruder");
    let err = suite
        .execute(&intruder, &ExecuteMsg::ReturnDeposits {}, &[])
        .unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());
}

#[test]
fn return_deposits_required_native_deposit() {
    let mut suite = Suite::new_native(Some(AssetUnchecked {
        denom: UncheckedDenom::Native("juno".into()),
        amount: Uint128::new(1_000),
    }));
    let owner = suite.owner.clone();
    let recipient = addr("recipient");

    suite.mint_native(&owner, coin(1_000, "juno"));
    suite
        .create_submission(&owner, &recipient, Some(coin(1_000, "juno")))
        .unwrap();

    assert_eq!(suite.native_balance(&owner, "juno"), Uint128::zero());
    assert_eq!(suite.native_balance(&recipient, "juno"), Uint128::zero());
    let adapter = suite.adapter.clone();
    assert_eq!(suite.native_balance(&adapter, "juno"), Uint128::new(1_000));

    suite.execute_owner(&ExecuteMsg::ReturnDeposits {}).unwrap();
    assert_eq!(suite.native_balance(&owner, "juno"), Uint128::new(1_000));
    assert_eq!(suite.native_balance(&recipient, "juno"), Uint128::zero());
    assert_eq!(suite.native_balance(&adapter, "juno"), Uint128::zero());
}

#[test]
fn return_deposits_required_native_deposit_multiple_deposits() {
    let mut suite = Suite::new_native(Some(AssetUnchecked {
        denom: UncheckedDenom::Native("juno".into()),
        amount: Uint128::new(1_000),
    }));
    let owner = suite.owner.clone();
    let recipient = addr("recipient");
    let einstein = addr("einstein");

    suite.mint_native(&owner, coin(1_000, "juno"));
    suite.mint_native(&einstein, coin(1_000, "juno"));

    suite
        .create_submission(&owner, &recipient, Some(coin(1_000, "juno")))
        .unwrap();
    suite
        .create_submission(&einstein, &einstein, Some(coin(1_000, "juno")))
        .unwrap();

    suite.execute_owner(&ExecuteMsg::ReturnDeposits {}).unwrap();
    assert_eq!(suite.native_balance(&owner, "juno"), Uint128::new(1_000));
    assert_eq!(suite.native_balance(&einstein, "juno"), Uint128::new(1_000));
    assert_eq!(suite.native_balance(&recipient, "juno"), Uint128::zero());
    let adapter = suite.adapter.clone();
    assert_eq!(suite.native_balance(&adapter, "juno"), Uint128::zero());
}

#[test]
fn return_deposits_required_cw20_deposit() {
    let (mut suite, cw20) = Suite::new_cw20_deposit();
    let owner = suite.owner.clone();
    let adapter = suite.adapter.clone();
    let recipient = addr("recipient");

    let inner = to_json_binary(&ReceiveMsg::CreateSubmission {
        name: "DAOers".to_string(),
        url: "https://daodao.zone".to_string(),
        address: recipient.to_string(),
    })
    .unwrap();
    suite
        .cw20_send(&cw20, &owner, &adapter, 1_000, inner)
        .unwrap();

    assert_eq!(suite.cw20_balance(&cw20, &owner), Uint128::new(999_000));
    assert_eq!(suite.cw20_balance(&cw20, &recipient), Uint128::zero());
    assert_eq!(suite.cw20_balance(&cw20, &adapter), Uint128::new(1_000));

    suite.execute_owner(&ExecuteMsg::ReturnDeposits {}).unwrap();

    assert_eq!(suite.cw20_balance(&cw20, &owner), Uint128::new(1_000_000));
    // Refund target is the submission sender (owner), not the recipient.
    assert_eq!(suite.cw20_balance(&cw20, &recipient), Uint128::zero());
    assert_eq!(suite.cw20_balance(&cw20, &adapter), Uint128::zero());
}

#[test]
fn sample_gauge_msgs_cw20() {
    let (mut suite, cw20) = Suite::new_cw20_reward(None);
    let owner = suite.owner.clone();
    let addr_1 = addr("addr1");
    let addr_2 = addr("addr2");
    let addr_3 = addr("addr3");
    let reward = Uint128::new(1_000_000);

    suite
        .execute(
            &owner,
            &ExecuteMsg::CreateSubmission {
                name: "name".to_string(),
                url: "https://test.url".to_string(),
                address: addr_1.to_string(),
            },
            &[],
        )
        .unwrap();
    suite
        .execute(
            &owner,
            &ExecuteMsg::CreateSubmission {
                name: "name".to_string(),
                url: "https://test.url".to_string(),
                address: addr_2.to_string(),
            },
            &[],
        )
        .unwrap();

    let selected = vec![
        (addr_1.to_string(), Decimal::percent(41)),
        (addr_2.to_string(), Decimal::percent(33)),
        (addr_3.to_string(), Decimal::percent(26)),
    ];

    let res: SampleGaugeMsgsResponse = suite
        .query(&AdapterQueryMsg::SampleGaugeMsgs { selected })
        .unwrap();
    assert_eq!(res.execute.len(), 3);
    assert_eq!(
        res.execute,
        [
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cw20.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: addr_1.to_string(),
                    amount: reward * Decimal::percent(41),
                })
                .unwrap(),
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cw20.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: addr_2.to_string(),
                    amount: reward * Decimal::percent(33),
                })
                .unwrap(),
                funds: vec![],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: cw20.to_string(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: addr_3.to_string(),
                    amount: reward * Decimal::percent(26),
                })
                .unwrap(),
                funds: vec![],
            }),
        ]
    );

    // Suppress unused-import lint in case helper not used.
    let _ = submit_cw20_create;
}
