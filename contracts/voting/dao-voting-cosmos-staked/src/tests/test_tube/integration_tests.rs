use crate::{
    msg::{ExecuteMsg, QueryMsg},
    tests::test_tube::{authz::Authz, staking::Staking},
};
use cosmwasm_std::{to_json_binary, Addr, Coin, CosmosMsg, Uint128};
use dao_voting::voting::{SingleChoiceAutoVote, Vote};
use osmosis_std::{
    shim::Any,
    types::{
        cosmos::{
            authz::v1beta1::{Grant, MsgExec, MsgGrant},
            staking::v1beta1::{MsgCreateValidator, MsgDelegate},
        },
        cosmwasm::wasm::v1::{
            AcceptedMessageKeysFilter, ContractExecutionAuthorization, ContractGrant,
            MaxCallsLimit, MsgExecuteContract,
        },
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

    let validator = &accounts[0];
    let staker = &accounts[1];
    let bot = &accounts[2];

    let prefix = validator.prefix();
    let valoper = validator
        .public_key()
        .account_id(format!("{prefix}valoper").as_str())
        .unwrap();

    let authz = Authz::new(&app);
    let staking = Staking::new(&app);

    staking
        .create_validator(
            MsgCreateValidator {
                description: None,
                commission: None,
                min_self_delegation: "1".to_string(),
                delegator_address: validator.address(),
                validator_address: valoper.to_string(),
                pubkey: Some(Any {
                    type_url: validator.public_key().type_url().to_string(),
                    value: validator.public_key().to_any().unwrap().value,
                }),
                value: Some(Coin::new(1, DENOM).into()),
            },
            &validator,
        )
        .unwrap();

    staking
        .delegate(
            MsgDelegate {
                delegator_address: staker.address(),
                validator_address: valoper.to_string(),
                amount: Some(Coin::new(100, DENOM).into()),
            },
            &staker,
        )
        .unwrap();

    app.increase_time(1);

    // Authz grant bot to execute
    proposal_single
        .execute(
            &dao_proposal_single::msg::ExecuteMsg::Propose(
                dao_voting::proposal::SingleChoiceProposeMsg {
                    title: "authz".to_string(),
                    description: "authz".to_string(),
                    msgs: vec![CosmosMsg::Stargate {
                        type_url: "/cosmos.authz.v1beta1.MsgExec".to_string(),
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
                                expiration: None,
                            }),
                        }
                        .into(),
                    }],
                    proposer: Some(staker.address()),
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
    proposal_single
        .execute(
            &dao_proposal_single::msg::ExecuteMsg::Execute { proposal_id: 1 },
            &[],
            staker,
        )
        .unwrap();

    // Query voting power
    let voting_power = vp_contract.query_vp(&accounts[0].address(), None).unwrap();
    assert_eq!(voting_power.power, Uint128::new(100));

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
