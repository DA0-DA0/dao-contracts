use cosmwasm_std::{coins, Addr, Coin, Uint128};
use cw_ownable::Ownership;
use dao_interface::voting::{DenomResponse, IsActiveResponse, VotingPowerAtHeightResponse};
use dao_voting_token_staked::msg::{
    ExecuteMsg as VotingTokenExecuteMsg, QueryMsg as VotingTokenQueryMsg,
};
use osmosis_test_tube::{Account, OsmosisTestApp};

use super::test_env::{TestEnv, TestEnvBuilder, RESERVE};

#[test]
fn test_full_integration_correct_setup() {
    let app = OsmosisTestApp::new();
    let env = TestEnvBuilder::new();
    let TestEnv {
        dao,
        tf_issuer,
        cw_abc,
        vp_contract,
        ..
    } = env.full_dao_setup(&app);

    // Issuer owner should be set to the abc contract
    let issuer_admin = tf_issuer
        .query::<Ownership<Addr>>(&cw_tokenfactory_issuer::msg::QueryMsg::Ownership {})
        .unwrap()
        .owner;
    assert_eq!(
        issuer_admin,
        Some(Addr::unchecked(cw_abc.contract_addr.clone()))
    );

    // Abc contract should have DAO as owner
    let abc_admin = cw_abc
        .query::<Ownership<Addr>>(&cw_abc::msg::QueryMsg::Ownership {})
        .unwrap()
        .owner;
    assert_eq!(
        abc_admin,
        Some(Addr::unchecked(dao.unwrap().contract_addr.clone()))
    );

    let issuer_denom = tf_issuer
        .query::<cw_tokenfactory_issuer::msg::DenomResponse>(
            &cw_tokenfactory_issuer::msg::QueryMsg::Denom {},
        )
        .unwrap()
        .denom;

    let abc_denom = cw_abc
        .query::<cw_abc::msg::DenomResponse>(&cw_abc::msg::QueryMsg::Denom {})
        .unwrap()
        .denom;

    let vp_denom = vp_contract
        .query::<DenomResponse>(&VotingTokenQueryMsg::Denom {})
        .unwrap()
        .denom;

    // Denoms for contracts should be the same
    assert_eq!(issuer_denom, abc_denom);
    assert_eq!(issuer_denom, vp_denom);
}

#[test]
fn test_stake_unstake_new_denom() {
    let app = OsmosisTestApp::new();
    let env = TestEnvBuilder::new();
    let TestEnv {
        vp_contract,
        accounts,
        cw_abc,
        ..
    } = env.full_dao_setup(&app);

    let denom = vp_contract
        .query::<DenomResponse>(&VotingTokenQueryMsg::Denom {})
        .unwrap()
        .denom;

    // Buy tokens off of bonding curve
    cw_abc
        .execute(
            &cw_abc::msg::ExecuteMsg::Buy {},
            &coins(100000, RESERVE),
            &accounts[0],
        )
        .unwrap();

    // Stake 100 tokens
    let stake_msg = VotingTokenExecuteMsg::Stake {};
    vp_contract
        .execute(&stake_msg, &[Coin::new(100, denom)], &accounts[0])
        .unwrap();

    app.increase_time(1);

    // Query voting power
    let voting_power: VotingPowerAtHeightResponse = vp_contract
        .query(&VotingTokenQueryMsg::VotingPowerAtHeight {
            address: accounts[0].address(),
            height: None,
        })
        .unwrap();
    assert_eq!(voting_power.power, Uint128::new(100));

    // DAO is active (default threshold is absolute count of 75)
    let active = vp_contract
        .query::<IsActiveResponse>(&VotingTokenQueryMsg::IsActive {})
        .unwrap()
        .active;
    assert!(active);

    // Unstake 50 tokens
    let unstake_msg = VotingTokenExecuteMsg::Unstake {
        amount: Uint128::new(50),
    };
    vp_contract
        .execute(&unstake_msg, &[], &accounts[0])
        .unwrap();
    app.increase_time(1);
    let voting_power: VotingPowerAtHeightResponse = vp_contract
        .query(&VotingTokenQueryMsg::VotingPowerAtHeight {
            address: accounts[0].address(),
            height: None,
        })
        .unwrap();
    assert_eq!(voting_power.power, Uint128::new(50));

    // DAO is not active
    let active = vp_contract
        .query::<IsActiveResponse>(&VotingTokenQueryMsg::IsActive {})
        .unwrap()
        .active;
    assert!(!active);

    // Can't claim before unstaking period (2 seconds)
    vp_contract
        .execute(&VotingTokenExecuteMsg::Claim {}, &[], &accounts[0])
        .unwrap_err();

    // Pass time, unstaking duration is set to 2 seconds
    app.increase_time(5);
    vp_contract
        .execute(&VotingTokenExecuteMsg::Claim {}, &[], &accounts[0])
        .unwrap();
}
