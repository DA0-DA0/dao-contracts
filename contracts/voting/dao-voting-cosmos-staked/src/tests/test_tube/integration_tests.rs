use crate::{msg::QueryMsg, tests::test_tube::staking::Staking};
use cosmwasm_std::{Addr, Coin, Uint128};
use osmosis_std::types::cosmos::staking::v1beta1::MsgDelegate;
use osmosis_test_tube::{Account, Module, OsmosisTestApp};

use super::test_env::{TestEnv, TestEnvBuilder};

const DENOM: &str = "uosmo";

#[test]
fn test_full_integration_correct_setup() {
    let app = OsmosisTestApp::new();
    let env = TestEnvBuilder::new();
    let TestEnv {
        dao, vp_contract, ..
    } = env.full_dao_setup(&app);

    // VP DAO should be set to the DAO.
    let vp_dao = vp_contract.query::<Addr>(&QueryMsg::Dao {}).unwrap();
    assert_eq!(vp_dao, dao.unwrap().contract_addr);
}

#[test]
fn test_staked_voting_power() {
    let app = OsmosisTestApp::new();
    let env = TestEnvBuilder::new();
    let TestEnv {
        dao: _dao,
        proposal_single: _proposal_single,
        vp_contract,
        accounts,
        ..
    } = env.full_dao_setup(&app);

    let staker = &accounts[0];

    let staking = Staking::new(&app);

    staking
        .delegate(
            MsgDelegate {
                delegator_address: staker.address(),
                validator_address: app.get_first_validator_address().unwrap(),
                amount: Some(Coin::new(100, DENOM).into()),
            },
            staker,
        )
        .unwrap();

    // Query address voting power
    let voting_power = vp_contract.query_vp(&accounts[0].address(), None).unwrap();
    assert_eq!(voting_power.power, Uint128::new(100));

    // Query total power
    let total_power = vp_contract.query_tp(None).unwrap();
    assert_eq!(total_power.power, Uint128::new(100));
}
