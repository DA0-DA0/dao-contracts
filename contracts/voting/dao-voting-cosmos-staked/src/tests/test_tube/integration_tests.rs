use crate::{
    msg::{ExecuteMsg, QueryMsg},
    tests::test_tube::{authz::Authz, staking::Staking},
};
use cosmwasm_std::{to_json_binary, Addr, Coin, CosmosMsg, Uint128};
use dao_voting::voting::{SingleChoiceAutoVote, Vote};
use osmosis_std::types::{
    cosmos::{
        authz::v1beta1::{Grant, MsgExec, MsgGrant},
        staking::v1beta1::MsgDelegate,
    },
    cosmwasm::wasm::v1::{
        AcceptedMessageKeysFilter, ContractExecutionAuthorization, ContractGrant, MaxCallsLimit,
        MsgExecuteContract,
    },
};
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
fn test_staked_voting_power_and_update() {
    let app = OsmosisTestApp::new();
    let env = TestEnvBuilder::new();
    let TestEnv {
        dao: _dao,
        proposal_single: _proposal_single,
        vp_contract,
        accounts,
        ..
    } = env.full_dao_setup(&app);

    let dao = _dao.unwrap();
    let proposal_single = _proposal_single.unwrap();

    let staker = &accounts[0];
    let bot = &accounts[1];

    let authz = Authz::new(&app);
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

    // Query voting power
    let voting_power = vp_contract.query_vp(&accounts[0].address(), None).unwrap();
    assert_eq!(voting_power.power, Uint128::new(100));

    // Authz grant bot to execute
    proposal_single
        .execute(
            &dao_proposal_single::msg::ExecuteMsg::Propose(
                dao_voting::proposal::SingleChoiceProposeMsg {
                    title: "authz".to_string(),
                    description: "authz".to_string(),
                    msgs: vec![CosmosMsg::Stargate {
                        type_url: "/cosmos.authz.v1beta1.MsgGrant".to_string(),
                        value: MsgGrant {
                            granter: dao.contract_addr.to_string(),
                            grantee: bot.address().to_string(),
                            grant: Some(Grant {
                                authorization: Some(
                                    ContractExecutionAuthorization {
                                        grants: vec![ContractGrant {
                                            contract: vp_contract.contract_addr.clone(),
                                            limit: Some(MaxCallsLimit { remaining: 10 }.to_any()),
                                            filter: Some(
                                                AcceptedMessageKeysFilter {
                                                    keys: vec!["update_total_staked".to_string()],
                                                }
                                                .to_any(),
                                            ),
                                        }],
                                    }
                                    .to_any(),
                                ),
                                expiration: Some(app.get_block_timestamp().plus_seconds(5)),
                            }),
                        }
                        .into(),
                    }],
                    proposer: None,
                    vote: Some(SingleChoiceAutoVote {
                        vote: Vote::Yes,
                        rationale: None,
                    }),
                },
            ),
            &[],
            staker,
        )
        .unwrap();

    app.increase_time(10);

    proposal_single
        .execute(
            &dao_proposal_single::msg::ExecuteMsg::Execute { proposal_id: 1 },
            &[],
            staker,
        )
        .unwrap();

    // Update total power from bot via authz exec on behalf of DAO
    authz
        .exec(
            MsgExec {
                grantee: bot.address(),
                msgs: vec![MsgExecuteContract {
                    sender: dao.contract_addr,
                    contract: vp_contract.contract_addr.clone(),
                    msg: to_json_binary(&ExecuteMsg::UpdateTotalStaked {
                        amount: Uint128::new(100),
                        height: None,
                    })
                    .unwrap()
                    .into(),
                    funds: vec![],
                }
                .to_any()],
            },
            bot,
        )
        .unwrap();

    // Query total power
    let total_power = vp_contract.query_tp(None).unwrap();
    assert_eq!(total_power.power, Uint128::new(100));
}
