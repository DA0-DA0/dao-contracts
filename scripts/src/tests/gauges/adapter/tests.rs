use abstract_cw20::{msg::Cw20ExecuteMsgFns, Cw20ExecuteMsg as AbsCw20ExecuteMsg};
use abstract_cw20_base::msg::QueryMsgFns;
use cosmwasm_std::{coin, to_json_binary, Addr, CosmosMsg, Decimal, Uint128, WasmMsg};

use crate::tests::gauges::adapter::{setup_gauge_adapter,setup_cw20_reward_gauge_adapter,cw20_helper};
use cw_denom::UncheckedDenom;
use cw_orch::{contract::interface_traits::CwOrchExecute, mock::MockBech32, prelude::*};
use gauge_adapter::{
    msg::{
        AdapterQueryMsg, AdapterQueryMsgFns, AllSubmissionsResponse, AssetUnchecked, ExecuteMsg,
        ExecuteMsgFns, ReceiveMsg, SubmissionResponse,
    },
    ContractError,
};
mod intention_proposal {

    use super::*;
    #[test]
    fn create_default_submission() {
        let mock = MockBech32::new("mock");
        let treasury = &mock.addr_make("community_pool");
        let adapter = setup_gauge_adapter(mock.clone(), None);

        // this one is created by default during instantiation
        assert_eq!(
            SubmissionResponse {
                sender: adapter.address().unwrap(),
                name: "Unimpressed".to_owned(),
                url: "Those funds go back to the community pool".to_owned(),
                address: treasury.clone(),
            },
            adapter
                .query(&gauge_adapter::msg::AdapterQueryMsg::Submission {
                    address: treasury.to_string()
                })
                .unwrap()
        )
    }

    #[test]
    fn create_submission_no_required_deposit() {
        let mock = MockBech32::new("mock");
        let adapter = setup_gauge_adapter(mock.clone(), None);

        let recipient = mock.addr_make("recipient");
        mock.add_balance(&mock.sender, vec![coin(1_000, "juno")])
            .unwrap();

        // Fails send funds along with the tx.
        let err = adapter.native_submission_helper(
            mock.sender.clone(),
            recipient.clone(),
            Some(coin(1_000, "juno")),
        )
        .unwrap_err();

        assert_eq!(
            ContractError::InvalidDepositAmount {
                correct_amount: Uint128::zero()
            },
            err.downcast().unwrap(),
        );

        // Valid submission.
        let result = adapter.native_submission_helper(
            mock.sender.clone(),
            recipient.clone(),
            None,
        );
        assert!(result.is_ok());

        assert_eq!(
            SubmissionResponse {
                sender: mock.sender,
                name: "DAOers".to_owned(),
                url: "https://daodao.zone".to_owned(),
                address: recipient.clone(),
            },
            adapter
                .query(&gauge_adapter::msg::AdapterQueryMsg::Submission {
                    address: recipient.to_string()
                })
                .unwrap(),
        )
    }

    #[test]
    fn overwrite_existing_submission() {
        let mock = MockBech32::new("mock");
        let adapter = setup_gauge_adapter(mock.clone(), None);
        let recipient = mock.addr_make("recipient");
        adapter.native_submission_helper(
            mock.sender.clone(),
            recipient.clone(),
            None,
        )
        .unwrap();
        assert_eq!(
            SubmissionResponse {
                sender: mock.sender.clone(),
                name: "DAOers".to_owned(),
                url: "https://daodao.zone".to_string(),
                address: recipient.clone(),
            },
            adapter.submission(recipient.to_string()).unwrap()
        );

        // Try to submit to the same address with different user
        let err = adapter.native_submission_helper(
            Addr::unchecked("anotheruser"),
            recipient.clone(),
            None,
        )
        .unwrap_err();

        assert_eq!(
            ContractError::UnauthorizedSubmission {},
            err.downcast().unwrap()
        );

        // Overwriting submission as same author works
        adapter.native_submission_helper( mock.sender, recipient.clone(), None).unwrap();

        let response = adapter.submission(recipient.to_string()).unwrap();
        assert_eq!(response.url, "https://daodao.zone".to_owned());
    }

    #[test]
    fn create_submission_required_deposit() {
        let mock = MockBech32::new("mock");
        let adapter = setup_gauge_adapter(
            mock.clone(),
            Some(AssetUnchecked {
                denom: UncheckedDenom::Native("juno".into()),
                amount: 1_000u128.into(),
            }),
        );

        let recipient = mock.addr_make("recipient");
        mock.add_balance(&mock.sender.clone(), vec![coin(1_000, "wynd")])
            .unwrap();
        mock.add_balance(&mock.sender.clone(), vec![coin(1_000, "juno")])
            .unwrap();
        // Fails if no funds sent.
        let err = adapter.native_submission_helper(
            mock.sender.clone(),
            recipient.clone(),
            None,
        )
        .unwrap_err();

        assert_eq!(
            ContractError::PaymentError(cw_utils::PaymentError::NoFunds {}),
            err.downcast().unwrap()
        );

        // Fails if correct denom but not enough amount.
        // Fails if no funds sent.
        let err = adapter.native_submission_helper(
            mock.sender.clone(),
            recipient.clone(),
            Some(Coin {
                denom: "juno".into(),
                amount: 999u128.into(),
            }),
        )
        .unwrap_err();

        assert_eq!(
            ContractError::InvalidDepositAmount {
                correct_amount: Uint128::new(1_000)
            },
            err.downcast().unwrap()
        );

        // Fails if enough amount but incorrect denom.
        let err = adapter.native_submission_helper(
            mock.sender.clone(),
            recipient.clone(),
            Some(Coin {
                denom: "wynd".into(),
                amount: 1_000u128.into(),
            }),
        )
        .unwrap_err();

        assert_eq!(
            ContractError::InvalidDepositType {},
            err.downcast().unwrap()
        );

        // Valid submission.
        adapter.native_submission_helper(
            mock.sender.clone(),
            recipient.clone(),
            Some(Coin {
                denom: "juno".into(),
                amount: 1_000u128.into(),
            }),
        )
        .unwrap();

        assert_eq!(
            SubmissionResponse {
                sender: mock.sender.clone(),
                name: "DAOers".to_owned(),
                url: "https://daodao.zone".to_owned(),
                address: recipient.clone(),
            },
            adapter
                .query(&AdapterQueryMsg::Submission {
                    address: recipient.to_string()
                })
                .unwrap()
        )
    }

    #[test]
    fn create_receive_required_deposit() {
        let mock = MockBech32::new("mock");
        let cw20 = cw20_helper(mock.clone());
        let bad_cw20 = cw20_helper(mock.clone());
        let cw20_addr = cw20.address().unwrap();
        let bad_cw20_addr = bad_cw20.address().unwrap();
        println!("good cw20: {:#?}", cw20_addr);
        println!("bad cw20: {:#?}", bad_cw20_addr);
        let adapter = setup_gauge_adapter(
            mock.clone(),
            Some(AssetUnchecked {
                denom: UncheckedDenom::Cw20(cw20_addr.to_string()),
                amount: 1_000u128.into(),
            }),
        );

        let recipient = mock.sender_addr().to_string();

        let binary_msg = to_json_binary(&ReceiveMsg::CreateSubmission {
            name: "DAOers".into(),
            url: "https://daodao.zone".into(),
            address: recipient.clone(),
        })
        .unwrap();
        // Fails by sending wrong cw20.
        let err = adapter
            .call_as(&Addr::unchecked(
                "mock1mzdhwvvh22wrt07w59wxyd58822qavwkx5lcej7aqfkpqqlhaqfsetqc4t",
            ))
            .execute(
                &ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
                    sender: recipient.to_string(),
                    amount: Uint128::from(1_000u128),
                    msg: binary_msg.clone(),
                }),
                None,
            )
            .unwrap_err();

        assert_eq!(
            ContractError::InvalidDepositType {},
            err.downcast().unwrap(),
        );

        // Fails by sending less tokens than required.
        let err = adapter
            .call_as(&cw20.address().unwrap())
            .execute(
                &ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
                    sender: recipient.to_string(),
                    amount: Uint128::from(999u128),
                    msg: binary_msg.clone(),
                }),
                None,
            )
            .unwrap_err();

        assert_eq!(
            ContractError::InvalidDepositAmount {
                correct_amount: Uint128::new(1_000)
            },
            err.downcast().unwrap()
        );

        // Valid submission.
        adapter
            .call_as(&cw20.address().unwrap())
            .execute(
                &ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
                    sender: recipient.to_string(),
                    amount: Uint128::from(1_000u128),
                    msg: binary_msg,
                }),
                None,
            )
            .unwrap();

        assert_eq!(
            SubmissionResponse {
                sender: mock.sender.clone(),
                name: "DAOers".to_owned(),
                url: "https://daodao.zone".to_owned(),
                address: Addr::unchecked(recipient.clone()),
            },
            adapter
                .query(&AdapterQueryMsg::Submission {
                    address: recipient.to_string()
                })
                .unwrap()
        );
        assert_eq!(
            2,
            adapter
                .query::<AllSubmissionsResponse>(&AdapterQueryMsg::AllSubmissions {})
                .unwrap()
                .submissions
                .len()
        )
    }

    #[test]
    fn return_deposits_no_required_deposit() {
        let mock = MockBech32::new("mock");
        let adapter = setup_gauge_adapter(mock.clone(), None);

        let err = adapter
            .execute(&ExecuteMsg::ReturnDeposits {}, None)
            .unwrap_err();

        assert_eq!(ContractError::NoDepositToRefund {}, err.downcast().unwrap())
    }

    #[test]
    fn return_deposits_no_admin() {
        let mock = MockBech32::new("mock");
        let bad_addr = mock.addr_make("einstien");
        let adapter = setup_gauge_adapter(
            mock.clone(),
            Some(AssetUnchecked {
                denom: UncheckedDenom::Native("juno".into()),
                amount: 1_000u128.into(),
            }),
        );

        let err = adapter
            .call_as(&bad_addr)
            .execute(&ExecuteMsg::ReturnDeposits {}, None)
            .unwrap_err();

        assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap())
    }

    #[test]
    fn return_deposits_required_native_deposit() {
        let mock = MockBech32::new("mock");
        let adapter = setup_gauge_adapter(
            mock.clone(),
            Some(AssetUnchecked {
                denom: UncheckedDenom::Native("juno".into()),
                amount: 1_000u128.into(),
            }),
        );
        mock.add_balance(&mock.sender, vec![coin(1_000u128, "juno")])
            .unwrap();
        let recipient = mock.addr_make("recipient");
        // Valid submission.
        adapter.native_submission_helper(
            mock.sender.clone(),
            recipient.clone(),
            Some(coin(1_000u128, "juno")),
        )
        .unwrap();

        assert_eq!(
            mock.query_balance(&mock.sender.clone(), "juno").unwrap(),
            Uint128::zero()
        );
        assert_eq!(
            mock.query_balance(&recipient, "juno").unwrap(),
            Uint128::zero()
        );
        assert_eq!(
            mock.query_balance(&adapter.address().unwrap(), "juno")
                .unwrap(),
            Uint128::from(1000u128)
        );

        adapter
            .execute(&ExecuteMsg::ReturnDeposits {}, None)
            .unwrap();
        assert_eq!(
            mock.query_balance(&mock.sender.clone(), "juno").unwrap(),
            Uint128::from(1000u128)
        );
        assert_eq!(
            mock.query_balance(&recipient, "juno").unwrap(),
            Uint128::zero()
        );
        assert_eq!(
            mock.query_balance(&adapter.address().unwrap(), "juno")
                .unwrap(),
            Uint128::zero()
        );
    }

    #[test]
    fn return_deposits_required_native_deposit_multiple_deposits() {
        let mock = MockBech32::new("mock");
        let adapter = setup_gauge_adapter(
            mock.clone(),
            Some(AssetUnchecked {
                denom: UncheckedDenom::Native("juno".into()),
                amount: 1_000u128.into(),
            }),
        );

        let recipient = mock.addr_make("recipient");
        let einstien = mock
            .addr_make_with_balance("einstien", vec![coin(1_000u128, "juno")])
            .unwrap();
        mock.add_balance(&mock.sender, vec![coin(1_000u128, "juno")])
            .unwrap();
        // Valid submission.
        adapter.native_submission_helper(
            mock.sender.clone(),
            recipient.clone(),
            Some(coin(1_000u128, "juno")),
        )
        .unwrap();
        // Valid submission.
        adapter.native_submission_helper(
            einstien.clone(),
            einstien.clone(),
            Some(coin(1_000u128, "juno")),
        )
        .unwrap();
        adapter.return_deposits().unwrap();
        assert_eq!(
            mock.query_balance(&mock.sender.clone(), "juno").unwrap(),
            Uint128::from(1000u128)
        );
        assert_eq!(
            mock.query_balance(&einstien, "juno").unwrap(),
            Uint128::from(1000u128)
        );
        assert_eq!(
            mock.query_balance(&recipient, "juno").unwrap(),
            Uint128::zero()
        );
        assert_eq!(
            mock.query_balance(&adapter.address().unwrap(), "juno")
                .unwrap(),
            Uint128::zero()
        );
    }

    #[test]
    fn return_deposits_required_cw20_deposit() {
        let mock = MockBech32::new("mock");
        let cw20 = cw20_helper(mock.clone());
        let recipient = mock.addr_make("recipient");
        let adapter = setup_gauge_adapter(
            mock.clone(),
            Some(AssetUnchecked {
                denom: UncheckedDenom::Cw20(cw20.addr_str().unwrap()),
                amount: 1_000u128.into(),
            }),
        );
        let binary_msg = to_json_binary(&ReceiveMsg::CreateSubmission {
            name: "DAOers".into(),
            url: "https://daodao.zone".into(),
            address: recipient.to_string(),
        })
        .unwrap();

        // Valid submission.
        cw20.send(1_000u128.into(), adapter.addr_str().unwrap(), binary_msg)
            .unwrap();

        assert_eq!(
            cw20.balance(mock.sender.to_string()).unwrap().balance,
            Uint128::from(999_000u128)
        );
        assert_eq!(
            cw20.balance(recipient.to_string()).unwrap().balance,
            Uint128::zero()
        );
        assert_eq!(
            cw20.balance(adapter.address().unwrap().to_string())
                .unwrap()
                .balance,
            Uint128::from(1_000u128),
        );

        adapter.return_deposits().unwrap();

        assert_eq!(
            cw20.balance(mock.sender.to_string()).unwrap().balance,
            Uint128::from(1_000_000u128),
        );
        // Tokens are sent back to submission sender, not recipient.
        assert_eq!(
            cw20.balance(recipient.to_string()).unwrap().balance,
            Uint128::zero(),
        );
        assert_eq!(
            cw20.balance(adapter.address().unwrap().to_string())
                .unwrap()
                .balance,
            Uint128::zero(),
        );
    }

    #[test]
    fn sample_gauge_msgs_cw20() {
        let mock = MockBech32::new("mock");
        let addr_1 = mock.addr_make("addr1");
        let addr_2 = mock.addr_make("addr2");
        let addr_3 = mock.addr_make("addr3");
        let reward = Uint128::new(1_000_000);
        let (adapter, cw20) = setup_cw20_reward_gauge_adapter(mock.clone(), None);

        adapter
            .create_submission(addr_1.to_string(), "name", "https://test.url")
            .unwrap();
        adapter
            .create_submission(addr_2.to_string(), "name", "https://test.url")
            .unwrap();

        let selected = vec![
            (addr_1.to_string(), Decimal::percent(41)),
            (addr_2.to_string(), Decimal::percent(33)),
            (addr_3.to_string(), Decimal::percent(26)),
        ];

        let res: gauge_adapter::msg::SampleGaugeMsgsResponse =
            adapter.sample_gauge_msgs(selected.clone()).unwrap();
        assert_eq!(res.execute.len(), 3);
        assert_eq!(
            res.execute,
            [
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cw20.addr_str().unwrap(),
                    msg: to_json_binary(&AbsCw20ExecuteMsg::Transfer {
                        recipient: addr_1.to_string(),
                        amount: reward * Decimal::percent(41)
                    })
                    .unwrap(),
                    funds: vec![]
                }),
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cw20.addr_str().unwrap(),
                    msg: to_json_binary(&AbsCw20ExecuteMsg::Transfer {
                        recipient: addr_2.to_string(),
                        amount: reward * Decimal::percent(33)
                    })
                    .unwrap(),
                    funds: vec![]
                }),
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: cw20.addr_str().unwrap(),
                    msg: to_json_binary(&AbsCw20ExecuteMsg::Transfer {
                        recipient: addr_3.to_string(),
                        amount: reward * Decimal::percent(26)
                    })
                    .unwrap(),
                    funds: vec![]
                }),
            ]
        );
    }
}
