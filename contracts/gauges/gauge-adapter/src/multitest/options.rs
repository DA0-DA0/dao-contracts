use cosmwasm_std::{coin, coins};
use cw_denom::UncheckedDenom;
use cw_orch::{contract::interface_traits::CwOrchQuery, mock::MockBech32};

use crate::{
    msg::{
        AdapterQueryMsg, AllOptionsResponse, AllSubmissionsResponse, AssetUnchecked,
        CheckOptionResponse,
    },
    multitest::suite::{native_submission_helper, setup_gauge_adapter},
};

#[test]
fn option_queries() {
    let mock = MockBech32::new("mock");
    let adapter = setup_gauge_adapter(
        mock.clone(),
        Some(AssetUnchecked {
            denom: UncheckedDenom::Native("juno".into()),
            amount: 1_000u128.into(),
        }),
    );

    let recipient = mock.addr_make("recipient");
    let newton = mock.addr_make("newton");
    let einstein = mock
        .addr_make_with_balance("einstein", coins(1_000, "juno"))
        .unwrap();

    mock.add_balance(&mock.sender, coins(1_000, "juno"))
        .unwrap();
    let options: AllSubmissionsResponse =
        adapter.query(&AdapterQueryMsg::AllSubmissions {}).unwrap();
    // account for a default option
    assert_eq!(options.submissions.len(), 1);

    // Valid submission.
    native_submission_helper(
        adapter.clone(),
        mock.sender.clone(),
        recipient.clone(),
        Some(coin(1_000u128, "juno")),
    )
    .unwrap();

    // Valid submission.
    native_submission_helper(
        adapter.clone(),
        einstein.clone(),
        einstein.clone(),
        Some(coin(1_000u128, "juno")),
    )
    .unwrap();

    let options: AllOptionsResponse = adapter.query(&AdapterQueryMsg::AllOptions {}).unwrap();
    assert_eq!(
        options,
        AllOptionsResponse {
            options: vec![
                einstein.to_string(),
                mock.addr_make("community_pool").to_string(),
                recipient.to_string()
            ]
        },
    );

    let option: CheckOptionResponse = adapter
        .query(&AdapterQueryMsg::CheckOption {
            option: einstein.to_string(),
        })
        .unwrap();
    assert!(option.valid);

    let option: CheckOptionResponse = adapter
        .query(&AdapterQueryMsg::CheckOption {
            option: newton.to_string(),
        })
        .unwrap();
    assert!(!option.valid);
}
