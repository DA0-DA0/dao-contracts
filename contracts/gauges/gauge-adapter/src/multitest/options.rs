use cosmwasm_std::{coin, Uint128};
use cw_denom::UncheckedDenom;

use crate::{
    msg::{
        AdapterQueryMsg, AllOptionsResponse, AllSubmissionsResponse, AssetUnchecked,
        CheckOptionResponse,
    },
    multitest::suite::{addr, Suite},
};

#[test]
fn option_queries() {
    let mut suite = Suite::new_native(Some(AssetUnchecked {
        denom: UncheckedDenom::Native("juno".into()),
        amount: Uint128::new(1_000),
    }));

    let owner = suite.owner.clone();
    let recipient = addr("recipient");
    let newton = addr("newton");
    let einstein = addr("einstein");
    suite.mint_native(&einstein, coin(1_000, "juno"));
    suite.mint_native(&owner, coin(1_000, "juno"));

    let options: AllSubmissionsResponse = suite.query(&AdapterQueryMsg::AllSubmissions {}).unwrap();
    // account for the default option (community pool refund target).
    assert_eq!(options.submissions.len(), 1);

    // Valid submission from owner.
    suite
        .create_submission(&owner, &recipient, Some(coin(1_000, "juno")))
        .unwrap();

    // Valid submission from einstein (recipient = self).
    suite
        .create_submission(&einstein, &einstein, Some(coin(1_000, "juno")))
        .unwrap();

    let options: AllOptionsResponse = suite.query(&AdapterQueryMsg::AllOptions {}).unwrap();
    let community_pool = suite.community_pool.clone();
    let mut expected = vec![
        einstein.to_string(),
        community_pool.to_string(),
        recipient.to_string(),
    ];
    expected.sort();
    let mut got = options.options.clone();
    got.sort();
    assert_eq!(got, expected);

    let option: CheckOptionResponse = suite
        .query(&AdapterQueryMsg::CheckOption {
            option: einstein.to_string(),
        })
        .unwrap();
    assert!(option.valid);

    let option: CheckOptionResponse = suite
        .query(&AdapterQueryMsg::CheckOption {
            option: newton.to_string(),
        })
        .unwrap();
    assert!(!option.valid);

    // SubmissionsBySender returns only submissions whose `sender` matches.
    let by_owner: AllSubmissionsResponse = suite
        .query(&AdapterQueryMsg::SubmissionsBySender {
            sender: owner.to_string(),
        })
        .unwrap();
    assert_eq!(by_owner.submissions.len(), 1);
    assert_eq!(by_owner.submissions[0].address, recipient);

    let by_einstein: AllSubmissionsResponse = suite
        .query(&AdapterQueryMsg::SubmissionsBySender {
            sender: einstein.to_string(),
        })
        .unwrap();
    assert_eq!(by_einstein.submissions.len(), 1);
    assert_eq!(by_einstein.submissions[0].address, einstein);

    // A sender with no submissions returns empty.
    let by_newton: AllSubmissionsResponse = suite
        .query(&AdapterQueryMsg::SubmissionsBySender {
            sender: newton.to_string(),
        })
        .unwrap();
    assert!(by_newton.submissions.is_empty());
}
