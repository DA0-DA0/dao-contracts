use crate::contract::{CONTRACT_NAME, CONTRACT_VERSION};
use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, GovTokenInstantiateMsg, GovTokenMsg, InstantiateMsg, ProposeMsg, QueryMsg,
    Threshold, VoteMsg,
};
use crate::query::{
    ConfigResponse, Cw20BalancesResponse, ProposalListResponse, ProposalResponse,
    ThresholdResponse, TokenListResponse, VoteInfo, VoteListResponse, VoteResponse,
    VoteTallyResponse,
};
use crate::state::{Config, Votes};
use cosmwasm_std::{
    coin, coins, to_binary, Addr, BankMsg, BlockInfo, Coin, CosmosMsg, Decimal, Empty, Timestamp,
    Uint128, WasmMsg,
};
use cw2::{query_contract_info, ContractVersion};
use cw20::{
    BalanceResponse, Cw20Coin, Cw20CoinVerified, Cw20Contract, Cw20ExecuteMsg, Cw20QueryMsg,
};
use cw3::{Status, Vote};
use cw_multi_test::{next_block, App, BankSudo, Contract, ContractWrapper, Executor, SudoMsg};
use cw_utils::{Duration, Expiration};
use stake_cw20::msg::ReceiveMsg;
use std::borrow::BorrowMut;

const OWNER: &str = "admin0001";
const VOTER1: &str = "voter0001";
const VOTER2: &str = "voter0002";
const VOTER3: &str = "voter0003";
const SOMEBODY: &str = "somebody";
const POWER_VOTER: &str = "power-voter";

const NATIVE_TOKEN_DENOM: &str = "ustars";
const INITIAL_BALANCE: u128 = 4000000;

pub fn contract_dao() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    )
    .with_reply(crate::contract::reply);
    Box::new(contract)
}

pub fn contract_cw20_gov() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

pub fn contract_staking() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        stake_cw20::contract::execute,
        stake_cw20::contract::instantiate,
        stake_cw20::contract::query,
    );
    Box::new(contract)
}

fn mock_app() -> App {
    App::default()
}

fn stake_balances(
    app: &mut App,
    initial_balances: Vec<Cw20Coin>,
    cw20_addr: &Addr,
    staking_addr: &Addr,
) {
    for balance in initial_balances.into_iter() {
        let msg = cw20::Cw20ExecuteMsg::Send {
            contract: staking_addr.to_string(),
            amount: balance.amount.checked_div(Uint128::new(2)).unwrap(),
            msg: to_binary(&ReceiveMsg::Stake {}).unwrap(),
        };
        app.execute_contract(
            Addr::unchecked(balance.address),
            cw20_addr.clone(),
            &msg,
            &[],
        )
        .unwrap();
    }
    app.update_block(next_block);
}

fn instantiate_dao(
    app: &mut App,
    cw20: Addr,
    threshold: Threshold,
    max_voting_period: Duration,
    proposal_deposit_amount: Option<Uint128>,
    refund_failed_proposals: Option<bool>,
) -> Addr {
    let dao_code_id = app.store_code(contract_dao());
    let staking_code_id = app.store_code(contract_staking());
    let msg = crate::msg::InstantiateMsg {
        name: "dao-dao".to_string(),
        description: "a great DAO!".to_string(),
        gov_token: GovTokenMsg::UseExistingCw20 {
            addr: cw20.to_string(),
            stake_contract_code_id: staking_code_id,
            label: "dao-dao".to_string(),
            unstaking_duration: None,
        },
        threshold,
        max_voting_period,
        proposal_deposit_amount: proposal_deposit_amount.unwrap_or_else(Uint128::zero),
        refund_failed_proposals,
    };
    app.instantiate_contract(
        dao_code_id,
        Addr::unchecked(OWNER),
        &msg,
        &[],
        "cw3-dao",
        None,
    )
    .unwrap()
}

fn setup_test_case(
    app: &mut App,
    threshold: Threshold,
    max_voting_period: Duration,
    init_funds: Vec<Coin>,
    proposal_deposit_amount: Option<Uint128>,
    refund_failed_proposals: Option<bool>,
) -> (Addr, Addr, Addr) {
    // Instantiate Gov Token Contract
    let cw20_id = app.store_code(contract_cw20_gov());
    let initial_balances = vec![
        Cw20Coin {
            address: OWNER.to_string(),
            amount: Uint128::new(INITIAL_BALANCE),
        },
        Cw20Coin {
            address: VOTER1.to_string(),
            amount: Uint128::new(INITIAL_BALANCE),
        },
        Cw20Coin {
            address: VOTER2.to_string(),
            amount: Uint128::new(INITIAL_BALANCE),
        },
        Cw20Coin {
            address: VOTER3.to_string(),
            amount: Uint128::new(INITIAL_BALANCE * 2),
        },
        Cw20Coin {
            address: POWER_VOTER.to_string(),
            amount: Uint128::new(INITIAL_BALANCE * 5),
        },
    ];
    let msg = cw20_base::msg::InstantiateMsg {
        name: String::from("Test"),
        symbol: String::from("TEST"),
        decimals: 6,
        initial_balances: initial_balances.clone(),
        mint: None,
        marketing: None,
    };

    let cw20_addr = app
        .instantiate_contract(cw20_id, Addr::unchecked(OWNER), &msg, &[], "cw20", None)
        .unwrap();

    app.update_block(next_block);

    // Instantiate DAO Contract
    let dao_addr = instantiate_dao(
        app,
        cw20_addr.clone(),
        threshold,
        max_voting_period,
        proposal_deposit_amount,
        refund_failed_proposals,
    );
    app.update_block(next_block);

    // Set some funds on the DAO contract for future proposals
    if !init_funds.is_empty() {
        app.sudo(SudoMsg::Bank({
            BankSudo::Mint {
                to_address: dao_addr.to_string(),
                amount: init_funds,
            }
        }))
        .ok();
    }

    // Get staking contract address
    let res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&dao_addr, &QueryMsg::GetConfig {})
        .unwrap();
    let staking_addr = res.staking_contract;

    // Stake balances
    stake_balances(
        app.borrow_mut(),
        initial_balances,
        &cw20_addr,
        &staking_addr,
    );

    (dao_addr, cw20_addr, staking_addr)
}

fn proposal_info() -> (Vec<CosmosMsg<Empty>>, String, String) {
    let bank_msg = BankMsg::Send {
        to_address: SOMEBODY.into(),
        amount: coins(1, NATIVE_TOKEN_DENOM),
    };
    let msgs = vec![bank_msg.into()];
    let title = "Pay somebody".to_string();
    let description = "Do I pay her?".to_string();
    (msgs, title, description)
}

fn pay_somebody_proposal() -> ExecuteMsg {
    let (msgs, title, description) = proposal_info();
    ExecuteMsg::Propose(ProposeMsg {
        title,
        description,
        msgs,
        latest: None,
    })
}

#[test]
fn test_instantiate_works() {
    let mut app = mock_app();

    let dao_code_id = app.store_code(contract_dao());
    let stake_contract_code_id = app.store_code(contract_staking());

    let max_voting_period = Duration::Time(1234567);
    let threshold = Threshold::ThresholdQuorum {
        threshold: Decimal::percent(51),
        quorum: Decimal::percent(10),
    };

    // Setup test case instantiates all contracts
    let (dao_addr, cw20_addr, _staking_addr) = setup_test_case(
        &mut app,
        threshold,
        max_voting_period,
        coins(100, NATIVE_TOKEN_DENOM),
        None,
        None,
    );

    // Total weight less than required weight not allowed
    let instantiate_msg = InstantiateMsg {
        name: "dao-dao".to_string(),
        description: "a great DAO!".to_string(),
        gov_token: GovTokenMsg::UseExistingCw20 {
            addr: cw20_addr.to_string(),
            stake_contract_code_id,
            label: "dao-dao".to_string(),
            unstaking_duration: None,
        },
        threshold: Threshold::AbsolutePercentage {
            percentage: Decimal::percent(101),
        },
        max_voting_period,
        proposal_deposit_amount: Uint128::zero(),
        refund_failed_proposals: None,
    };
    let err = app
        .instantiate_contract(
            dao_code_id,
            Addr::unchecked(OWNER),
            &instantiate_msg,
            &[],
            "high required weight",
            None,
        )
        .unwrap_err();
    assert_eq!(
        ContractError::UnreachableThreshold {},
        err.downcast().unwrap()
    );

    // Verify contract version set properly
    let version = query_contract_info(&app, dao_addr).unwrap();
    assert_eq!(
        ContractVersion {
            contract: CONTRACT_NAME.to_string(),
            version: CONTRACT_VERSION.to_string(),
        },
        version,
    );
}

#[test]
fn instantiate_new_gov_token() {
    let mut app = mock_app();
    let cw20_code_id = app.store_code(contract_cw20_gov());
    let dao_code_id = app.store_code(contract_dao());
    let stake_contract_code_id = app.store_code(contract_staking());

    // Fails with empty initial balances
    let instantiate_msg = InstantiateMsg {
        name: "dao-dao".to_string(),
        description: "a great DAO!".to_string(),
        gov_token: GovTokenMsg::InstantiateNewCw20 {
            cw20_code_id,
            stake_contract_code_id,
            label: String::from("DAO DAO"),
            initial_dao_balance: Some(Uint128::new(1000000)),
            msg: GovTokenInstantiateMsg {
                name: String::from("DAO DAO"),
                symbol: String::from("DAO"),
                decimals: 6,
                initial_balances: vec![],
                marketing: None,
            },
            unstaking_duration: None,
        },
        threshold: Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(10),
        },
        max_voting_period: Duration::Time(1234567),
        proposal_deposit_amount: Uint128::zero(),
        refund_failed_proposals: None,
    };
    let res = app.instantiate_contract(
        dao_code_id,
        Addr::unchecked(OWNER),
        &instantiate_msg,
        &[],
        "all good",
        None,
    );
    assert!(res.is_err());
    let initial_balances = vec![
        Cw20Coin {
            address: OWNER.to_string(),
            amount: Uint128::new(INITIAL_BALANCE),
        },
        Cw20Coin {
            address: VOTER1.to_string(),
            amount: Uint128::new(INITIAL_BALANCE),
        },
        Cw20Coin {
            address: VOTER2.to_string(),
            amount: Uint128::new(INITIAL_BALANCE),
        },
        Cw20Coin {
            address: VOTER3.to_string(),
            amount: Uint128::new(INITIAL_BALANCE * 2),
        },
        Cw20Coin {
            address: POWER_VOTER.to_string(),
            amount: Uint128::new(INITIAL_BALANCE * 5),
        },
    ];

    // Instantiate new gov token
    let instantiate_msg = InstantiateMsg {
        name: "dao-dao".to_string(),
        description: "a great DAO!".to_string(),
        gov_token: GovTokenMsg::InstantiateNewCw20 {
            cw20_code_id,
            stake_contract_code_id,
            label: String::from("DAO DAO"),
            initial_dao_balance: None,
            msg: GovTokenInstantiateMsg {
                name: String::from("DAO DAO"),
                symbol: String::from("DAO"),
                decimals: 6,
                initial_balances: initial_balances.clone(),
                marketing: None,
            },
            unstaking_duration: None,
        },
        threshold: Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(10),
        },
        max_voting_period: Duration::Time(1234567),
        proposal_deposit_amount: Uint128::zero(),
        refund_failed_proposals: None,
    };
    let res = app.instantiate_contract(
        dao_code_id,
        Addr::unchecked(OWNER),
        &instantiate_msg,
        &[],
        "all good",
        None,
    );
    assert!(res.is_ok());

    let dao_addr = res.unwrap();
    let res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&dao_addr, &QueryMsg::GetConfig {})
        .unwrap();
    let cw20_addr = res.gov_token;
    let staking_addr = res.staking_contract;

    // Stake balances
    stake_balances(
        app.borrow_mut(),
        initial_balances.clone(),
        &cw20_addr,
        &staking_addr,
    );

    // Make proposal to mint some gov tokens for the DAO
    let wasm_exec_msg = WasmMsg::Execute {
        contract_addr: cw20_addr.clone().into(),
        msg: to_binary(&Cw20ExecuteMsg::Mint {
            recipient: dao_addr.clone().into(),
            amount: Uint128::new(1000),
        })
        .unwrap(),
        funds: vec![],
    };
    let (_msgs, title, description) = proposal_info();
    let proposal_msg = ExecuteMsg::Propose(ProposeMsg {
        title,
        description,
        msgs: vec![wasm_exec_msg.into()],
        latest: None,
    });

    let res = app.execute_contract(
        Addr::unchecked(VOTER3),
        dao_addr.clone(),
        &proposal_msg,
        &[],
    );
    assert!(res.is_ok());

    let proposal_id: u64 = res.unwrap().custom_attrs(1)[2].value.parse().unwrap();

    // Proposal passes
    let yes_vote = ExecuteMsg::Vote(VoteMsg {
        proposal_id,
        vote: Vote::Yes,
    });
    let res = app.execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &yes_vote, &[]);
    assert!(res.is_ok());
    let res = app
        .execute_contract(
            Addr::unchecked(POWER_VOTER),
            dao_addr.clone(),
            &yes_vote,
            &[],
        )
        .unwrap();

    assert_eq!(
        res.custom_attrs(1),
        [
            ("action", "vote"),
            ("sender", POWER_VOTER),
            ("proposal_id", proposal_id.to_string().as_str()),
            ("status", "Passed"),
        ],
    );

    // Proposal is executed
    let execution = ExecuteMsg::Execute { proposal_id };
    // Execute works. Anybody can execute Passed proposals
    let res = app
        .execute_contract(Addr::unchecked(SOMEBODY), dao_addr.clone(), &execution, &[])
        .unwrap();
    assert_eq!(
        res.custom_attrs(1),
        [
            ("action", "execute"),
            ("sender", SOMEBODY),
            ("proposal_id", proposal_id.to_string().as_str()),
        ],
    );

    // Query gov token contract to get DAO balance
    let res: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            cw20_addr,
            &Cw20QueryMsg::Balance {
                address: dao_addr.into(),
            },
        )
        .unwrap();
    assert_eq!(
        res,
        BalanceResponse {
            balance: Uint128::new(1000)
        }
    );

    // Test giving DAO an initial gov token balance
    let instantiate_msg = InstantiateMsg {
        name: "dao-dao".to_string(),
        description: "a great DAO!".to_string(),
        gov_token: GovTokenMsg::InstantiateNewCw20 {
            cw20_code_id,
            stake_contract_code_id,
            label: String::from("DAO DAO"),
            initial_dao_balance: Some(Uint128::new(1000)),
            msg: GovTokenInstantiateMsg {
                name: String::from("DAO DAO"),
                symbol: String::from("DAO"),
                decimals: 6,
                initial_balances,
                marketing: None,
            },
            unstaking_duration: None,
        },
        threshold: Threshold::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(10),
        },
        max_voting_period: Duration::Time(1234567),
        proposal_deposit_amount: Uint128::zero(),
        refund_failed_proposals: None,
    };
    let res = app.instantiate_contract(
        dao_code_id,
        Addr::unchecked(OWNER),
        &instantiate_msg,
        &[],
        "all good",
        None,
    );
    let dao_addr = res.unwrap();
    let res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&dao_addr, &QueryMsg::GetConfig {})
        .unwrap();
    let cw20_addr = res.gov_token;
    let res: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            cw20_addr,
            &Cw20QueryMsg::Balance {
                address: dao_addr.into(),
            },
        )
        .unwrap();
    assert_eq!(
        res,
        BalanceResponse {
            balance: Uint128::new(1000)
        }
    );
}

#[test]
fn test_propose_works() {
    let mut app = mock_app();

    let voting_period = Duration::Time(2000000);
    let threshold = Threshold::ThresholdQuorum {
        threshold: Decimal::percent(51),
        quorum: Decimal::percent(10),
    };
    let (dao_addr, _, _) = setup_test_case(
        &mut app,
        threshold,
        voting_period,
        coins(100, NATIVE_TOKEN_DENOM),
        None,
        None,
    );

    let proposal = pay_somebody_proposal();
    // Only voters with a gov token balance can propose
    let err = app
        .execute_contract(Addr::unchecked(SOMEBODY), dao_addr.clone(), &proposal, &[])
        .unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());

    // Wrong expiration option fails
    let msgs = match proposal.clone() {
        ExecuteMsg::Propose(ProposeMsg { msgs, .. }) => msgs,
        _ => panic!("Wrong variant"),
    };
    let proposal_wrong_exp = ExecuteMsg::Propose(ProposeMsg {
        title: "Rewarding somebody".to_string(),
        description: "Do we reward her?".to_string(),
        msgs,
        latest: Some(Expiration::AtHeight(123456)),
    });
    let err = app
        .execute_contract(
            Addr::unchecked(OWNER),
            dao_addr.clone(),
            &proposal_wrong_exp,
            &[],
        )
        .unwrap_err();
    assert_eq!(ContractError::WrongExpiration {}, err.downcast().unwrap());

    // Proposal from voter works
    let res = app
        .execute_contract(Addr::unchecked(VOTER3), dao_addr, &proposal, &[])
        .unwrap();
    assert_eq!(
        res.custom_attrs(1),
        [
            ("action", "propose"),
            ("sender", VOTER3),
            ("proposal_id", "1"),
            ("status", "Open"),
        ],
    );
}

fn get_tally(app: &App, dao_addr: &str, proposal_id: u64) -> Uint128 {
    // Get all the voters on the proposal
    let voters = QueryMsg::ListVotes {
        proposal_id,
        start_after: None,
        limit: None,
    };
    let votes: VoteListResponse = app.wrap().query_wasm_smart(dao_addr, &voters).unwrap();
    // Sum the weights of the Yes votes to get the tally
    votes
        .votes
        .iter()
        .filter(|&v| v.vote == Vote::Yes)
        .map(|v| v.weight)
        .sum()
}

fn expire(voting_period: Duration) -> impl Fn(&mut BlockInfo) {
    move |block: &mut BlockInfo| {
        match voting_period {
            Duration::Time(duration) => block.time = block.time.plus_seconds(duration + 1),
            Duration::Height(duration) => block.height += duration + 1,
        };
    }
}

fn unexpire(voting_period: Duration) -> impl Fn(&mut BlockInfo) {
    move |block: &mut BlockInfo| {
        match voting_period {
            Duration::Time(duration) => {
                block.time = Timestamp::from_nanos(block.time.nanos() - (duration * 1_000_000_000));
            }
            Duration::Height(duration) => block.height -= duration,
        };
    }
}

#[test]
fn test_proposal_queries() {
    let mut app = mock_app();

    let voting_period = Duration::Time(2000000);
    let threshold = Threshold::ThresholdQuorum {
        threshold: Decimal::percent(51),
        quorum: Decimal::percent(10),
    };
    let (dao_addr, _, _) = setup_test_case(
        &mut app,
        threshold,
        voting_period,
        coins(100, NATIVE_TOKEN_DENOM),
        None,
        None,
    );

    let prop_count: u64 = app
        .wrap()
        .query_wasm_smart(&dao_addr, &QueryMsg::ProposalCount {})
        .unwrap();
    assert_eq!(prop_count, 0);

    // create proposal with 1 vote power
    let proposal = pay_somebody_proposal();
    let res = app
        .execute_contract(Addr::unchecked(VOTER1), dao_addr.clone(), &proposal, &[])
        .unwrap();
    let proposal_id1: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

    let prop_count: u64 = app
        .wrap()
        .query_wasm_smart(&dao_addr, &QueryMsg::ProposalCount {})
        .unwrap();
    assert_eq!(prop_count, 1);

    // another proposal
    app.update_block(next_block);
    let proposal = pay_somebody_proposal();
    let res = app
        .execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &proposal, &[])
        .unwrap();
    let proposal_id2: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

    // Imediately passes on yes vote
    let yes_vote = ExecuteMsg::Vote(VoteMsg {
        proposal_id: proposal_id2,
        vote: Vote::Yes,
    });
    let res = app.execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &yes_vote, &[]);
    assert!(res.is_ok());

    // expire them both
    app.update_block(expire(voting_period));

    // add one more open proposal, 2 votes
    let proposal = pay_somebody_proposal();
    let res = app
        .execute_contract(Addr::unchecked(VOTER2), dao_addr.clone(), &proposal, &[])
        .unwrap();
    let proposal_id3: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
    let proposed_at = app.block_info();

    let prop_count: u64 = app
        .wrap()
        .query_wasm_smart(&dao_addr, &QueryMsg::ProposalCount {})
        .unwrap();
    assert_eq!(prop_count, 3);

    // next block, let's query them all... make sure status is properly updated (1 should be rejected in query)
    app.update_block(next_block);
    let list_query = QueryMsg::ListProposals {
        start_after: None,
        limit: None,
    };
    let res: ProposalListResponse = app.wrap().query_wasm_smart(&dao_addr, &list_query).unwrap();
    assert_eq!(3, res.proposals.len());

    // check the id and status are properly set
    let info: Vec<_> = res.proposals.iter().map(|p| (p.id, p.status)).collect();
    let expected_info = vec![
        (proposal_id1, Status::Rejected),
        (proposal_id2, Status::Passed),
        (proposal_id3, Status::Open),
    ];
    assert_eq!(expected_info, info);

    // ensure the common features are set
    let (expected_msgs, expected_title, expected_description) = proposal_info();
    for prop in res.proposals {
        assert_eq!(prop.title, expected_title);
        assert_eq!(prop.description, expected_description);
        assert_eq!(prop.msgs, expected_msgs);
    }

    // reverse query can get just proposal_id3
    let list_query = QueryMsg::ReverseProposals {
        start_before: None,
        limit: Some(1),
    };
    let res: ProposalListResponse = app.wrap().query_wasm_smart(&dao_addr, &list_query).unwrap();
    assert_eq!(1, res.proposals.len());

    let (msgs, title, description) = proposal_info();
    let expected = ProposalResponse {
        id: proposal_id3,
        title,
        description,
        proposer: Addr::unchecked(VOTER2),
        msgs,
        expires: voting_period.after(&proposed_at),
        status: Status::Open,
        threshold: ThresholdResponse::ThresholdQuorum {
            threshold: Decimal::percent(51),
            quorum: Decimal::percent(10),
            total_weight: Uint128::new(20000000),
        },
        deposit_amount: Uint128::zero(),
    };
    assert_eq!(&expected, &res.proposals[0]);
}

#[test]
fn test_query_limited() {
    let mut app = mock_app();

    let voting_period = Duration::Time(2000000);
    let threshold = Threshold::ThresholdQuorum {
        threshold: Decimal::percent(51),
        quorum: Decimal::percent(10),
    };
    let (dao_addr, _, _) = setup_test_case(
        &mut app,
        threshold,
        voting_period,
        coins(100, NATIVE_TOKEN_DENOM),
        None,
        None,
    );

    // Make some queries that should be limited
    let res: Result<ProposalListResponse, _> = app.wrap().query_wasm_smart(
        &dao_addr,
        &QueryMsg::ReverseProposals {
            start_before: None,
            limit: Some(40),
        },
    );
    assert!(res.is_err());
    let res: Result<ProposalListResponse, _> = app.wrap().query_wasm_smart(
        &dao_addr,
        &QueryMsg::ListProposals {
            start_after: None,
            limit: Some(31),
        },
    );
    assert!(res.is_err());
    let res: Result<Cw20BalancesResponse, _> = app.wrap().query_wasm_smart(
        &dao_addr,
        &QueryMsg::Cw20Balances {
            start_after: None,
            limit: Some(3000),
        },
    );
    assert!(res.is_err());

    // Make some queries that should not be limited
    let res: ProposalListResponse = app
        .wrap()
        .query_wasm_smart(
            &dao_addr,
            &QueryMsg::ReverseProposals {
                start_before: None,
                limit: Some(30),
            },
        )
        .unwrap();
    assert_eq!(res.proposals, vec![]);
    let res: ProposalListResponse = app
        .wrap()
        .query_wasm_smart(
            &dao_addr,
            &QueryMsg::ListProposals {
                start_after: None,
                limit: Some(0),
            },
        )
        .unwrap();
    assert_eq!(res.proposals, vec![]);
    let res: Cw20BalancesResponse = app
        .wrap()
        .query_wasm_smart(
            &dao_addr,
            &QueryMsg::Cw20Balances {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(
        res.cw20_balances,
        vec![Cw20CoinVerified {
            address: Addr::unchecked("Contract #0"),
            amount: Uint128::zero()
        }]
    );
}

#[test]
fn test_token_add_limited() {
    let mut app = mock_app();

    let voting_period = Duration::Time(2000000);
    let threshold = Threshold::AbsolutePercentage {
        percentage: Decimal::percent(20),
    };
    let (dao_addr, _cw20_addr, _) = setup_test_case(
        &mut app,
        threshold,
        voting_period,
        coins(10, NATIVE_TOKEN_DENOM),
        None,
        None,
    );

    // Attempt to add a bunch of nonesense tokens
    let update_token_list_msg = ExecuteMsg::UpdateCw20TokenList {
        to_add: (0..20)
            .into_iter()
            .map(|i| Addr::unchecked(i.to_string()))
            .collect(),
        to_remove: (20..31)
            .into_iter()
            .map(|i| Addr::unchecked(i.to_string()))
            .collect(),
    };
    let wasm_msg = WasmMsg::Execute {
        contract_addr: dao_addr.clone().into(),
        msg: to_binary(&update_token_list_msg).unwrap(),
        funds: vec![],
    };
    let proposal_msg = ExecuteMsg::Propose(ProposeMsg {
        title: String::from("Change params"),
        description: String::from("Add some _totally legit_ tokens"),
        msgs: vec![wasm_msg.into()],
        latest: None,
    });
    let res = app
        .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &proposal_msg, &[])
        .unwrap();
    let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

    // Imediately passes on yes vote
    let yes_vote = ExecuteMsg::Vote(VoteMsg {
        proposal_id,
        vote: Vote::Yes,
    });
    let res = app.execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &yes_vote, &[]);
    assert!(res.is_ok());

    // Execute - this ought to fail as we are attempting to add
    // far too many voting tokens.
    let execution = ExecuteMsg::Execute { proposal_id };
    let err = app
        .execute_contract(Addr::unchecked(OWNER), dao_addr, &execution, &[])
        .unwrap_err();
    assert!(matches!(
        err.downcast().unwrap(),
        ContractError::OversizedRequest {
            size: 31u64,
            max: 30u64
        }
    ));
}

#[test]
fn test_vote_works() {
    let mut app = mock_app();

    let voting_period = Duration::Time(2000000);
    let threshold = Threshold::ThresholdQuorum {
        threshold: Decimal::percent(51),
        quorum: Decimal::percent(10),
    };
    let (dao_addr, _cw20_addr, _) = setup_test_case(
        &mut app,
        threshold,
        voting_period,
        coins(100, NATIVE_TOKEN_DENOM),
        None,
        None,
    );

    // create proposal with 0 vote power
    let proposal = pay_somebody_proposal();
    let res = app
        .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &proposal, &[])
        .unwrap();

    // Get the proposal id from the logs
    let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

    // Owner votes
    let yes_vote = ExecuteMsg::Vote(VoteMsg {
        proposal_id,
        vote: Vote::Yes,
    });
    let res = app.execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &yes_vote, &[]);
    assert!(res.is_ok());

    // Owner cannot vote (again)
    let yes_vote = ExecuteMsg::Vote(VoteMsg {
        proposal_id,
        vote: Vote::Yes,
    });
    let err = app
        .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &yes_vote, &[])
        .unwrap_err();
    assert_eq!(ContractError::AlreadyVoted {}, err.downcast().unwrap());

    // Only voters can vote
    let err = app
        .execute_contract(Addr::unchecked(SOMEBODY), dao_addr.clone(), &yes_vote, &[])
        .unwrap_err();
    assert_eq!(ContractError::Unauthorized {}, err.downcast().unwrap());

    // But voter1 can
    let res = app
        .execute_contract(Addr::unchecked(VOTER1), dao_addr.clone(), &yes_vote, &[])
        .unwrap();
    assert_eq!(
        res.custom_attrs(1),
        [
            ("action", "vote"),
            ("sender", VOTER1),
            ("proposal_id", proposal_id.to_string().as_str()),
            ("status", "Open"),
        ],
    );

    // No/Veto votes have no effect on the tally
    // Compute the current tally
    let tally = get_tally(&app, dao_addr.as_ref(), proposal_id);
    assert_eq!(tally, Uint128::new(4000000));

    // Cast a No vote
    let no_vote = ExecuteMsg::Vote(VoteMsg {
        proposal_id,
        vote: Vote::No,
    });
    let _ = app
        .execute_contract(Addr::unchecked(VOTER2), dao_addr.clone(), &no_vote, &[])
        .unwrap();

    // Cast a Veto vote
    let veto_vote = ExecuteMsg::Vote(VoteMsg {
        proposal_id,
        vote: Vote::Veto,
    });
    let _ = app
        .execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &veto_vote, &[])
        .unwrap();

    // Tally unchanged
    assert_eq!(tally, get_tally(&app, dao_addr.as_ref(), proposal_id));

    let err = app
        .execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &yes_vote, &[])
        .unwrap_err();
    assert_eq!(ContractError::AlreadyVoted {}, err.downcast().unwrap());

    // Expired proposals cannot be voted
    app.update_block(expire(voting_period));
    let err = app
        .execute_contract(Addr::unchecked(VOTER1), dao_addr.clone(), &yes_vote, &[])
        .unwrap_err();
    assert_eq!(ContractError::Expired {}, err.downcast().unwrap());
    app.update_block(unexpire(voting_period));

    // Power voter supports it, so it passes
    let res = app
        .execute_contract(
            Addr::unchecked(POWER_VOTER),
            dao_addr.clone(),
            &yes_vote,
            &[],
        )
        .unwrap();

    assert_eq!(
        res.custom_attrs(1),
        [
            ("action", "vote"),
            ("sender", POWER_VOTER),
            ("proposal_id", proposal_id.to_string().as_str()),
            ("status", "Passed"),
        ],
    );

    // non-Open proposals cannot be voted
    let err = app
        .execute_contract(Addr::unchecked(VOTER1), dao_addr.clone(), &yes_vote, &[])
        .unwrap_err();
    assert_eq!(ContractError::NotOpen {}, err.downcast().unwrap());

    // query individual votes
    let voter = OWNER.into();
    let vote: VoteResponse = app
        .wrap()
        .query_wasm_smart(&dao_addr, &QueryMsg::Vote { proposal_id, voter })
        .unwrap();
    assert_eq!(
        vote.vote.unwrap(),
        VoteInfo {
            voter: OWNER.into(),
            vote: Vote::Yes,
            weight: Uint128::new(2000000)
        }
    );

    // nay sayer
    let voter = VOTER2.into();
    let vote: VoteResponse = app
        .wrap()
        .query_wasm_smart(&dao_addr, &QueryMsg::Vote { proposal_id, voter })
        .unwrap();
    assert_eq!(
        vote.vote.unwrap(),
        VoteInfo {
            voter: VOTER2.into(),
            vote: Vote::No,
            weight: Uint128::new(2000000),
        }
    );

    // non-voter
    let voter = SOMEBODY.into();
    let vote: VoteResponse = app
        .wrap()
        .query_wasm_smart(&dao_addr, &QueryMsg::Vote { proposal_id, voter })
        .unwrap();
    assert!(vote.vote.is_none());
}

#[test]
fn test_execute_works() {
    let mut app = mock_app();

    let voting_period = Duration::Time(2000000);
    let threshold = Threshold::ThresholdQuorum {
        threshold: Decimal::percent(10),
        quorum: Decimal::percent(10),
    };
    let (dao_addr, _cw20_addr, _) = setup_test_case(
        &mut app,
        threshold,
        voting_period,
        coins(10, NATIVE_TOKEN_DENOM),
        None,
        None,
    );

    // ensure we have cash to cover the proposal
    let contract_bal = app
        .wrap()
        .query_balance(&dao_addr, NATIVE_TOKEN_DENOM)
        .unwrap();
    assert_eq!(contract_bal, coin(10, NATIVE_TOKEN_DENOM));

    // create proposal with 0 vote power
    let proposal = pay_somebody_proposal();
    let res = app
        .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &proposal, &[])
        .unwrap();

    // Get the proposal id from the logs
    let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

    // Only Passed can be executed
    let execution = ExecuteMsg::Execute { proposal_id };
    let err = app
        .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &execution, &[])
        .unwrap_err();
    assert_eq!(
        ContractError::WrongExecuteStatus {},
        err.downcast().unwrap()
    );

    // Vote it, so it passes
    let vote = ExecuteMsg::Vote(VoteMsg {
        proposal_id,
        vote: Vote::Yes,
    });
    let res = app
        .execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &vote, &[])
        .unwrap();
    assert_eq!(
        res.custom_attrs(1),
        [
            ("action", "vote"),
            ("sender", VOTER3),
            ("proposal_id", proposal_id.to_string().as_str()),
            ("status", "Passed"),
        ],
    );

    // In passing: Try to close Passed fails
    let closing = ExecuteMsg::Close { proposal_id };
    let err = app
        .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &closing, &[])
        .unwrap_err();
    assert_eq!(ContractError::WrongCloseStatus {}, err.downcast().unwrap());

    // Execute works. Anybody can execute Passed proposals
    let res = app
        .execute_contract(Addr::unchecked(SOMEBODY), dao_addr.clone(), &execution, &[])
        .unwrap();
    assert_eq!(
        res.custom_attrs(1),
        [
            ("action", "execute"),
            ("sender", SOMEBODY),
            ("proposal_id", proposal_id.to_string().as_str()),
        ],
    );

    // verify money was transfered
    let some_bal = app
        .wrap()
        .query_balance(SOMEBODY, NATIVE_TOKEN_DENOM)
        .unwrap();
    assert_eq!(some_bal, coin(1, NATIVE_TOKEN_DENOM));
    let contract_bal = app
        .wrap()
        .query_balance(&dao_addr, NATIVE_TOKEN_DENOM)
        .unwrap();
    assert_eq!(contract_bal, coin(9, NATIVE_TOKEN_DENOM));

    // In passing: Try to close Executed fails
    let err = app
        .execute_contract(Addr::unchecked(OWNER), dao_addr, &closing, &[])
        .unwrap_err();
    assert_eq!(ContractError::WrongCloseStatus {}, err.downcast().unwrap());
}

#[test]
fn test_close_works() {
    let mut app = mock_app();

    let voting_period = Duration::Height(2000000);
    let threshold = Threshold::ThresholdQuorum {
        threshold: Decimal::percent(51),
        quorum: Decimal::percent(10),
    };
    let (dao_addr, _cw20_addr, _) = setup_test_case(
        &mut app,
        threshold,
        voting_period,
        coins(10, NATIVE_TOKEN_DENOM),
        None,
        None,
    );

    // create proposal with 0 vote power
    let proposal = pay_somebody_proposal();
    let res = app
        .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &proposal, &[])
        .unwrap();

    // Get the proposal id from the logs
    let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

    // Non-expired proposals cannot be closed
    let closing = ExecuteMsg::Close { proposal_id };
    let err = app
        .execute_contract(Addr::unchecked(SOMEBODY), dao_addr.clone(), &closing, &[])
        .unwrap_err();
    assert_eq!(ContractError::NotExpired {}, err.downcast().unwrap());

    // Expired proposals can be closed
    app.update_block(expire(voting_period));
    let res = app
        .execute_contract(Addr::unchecked(SOMEBODY), dao_addr.clone(), &closing, &[])
        .unwrap();
    assert_eq!(
        res.custom_attrs(1),
        [
            ("action", "close"),
            ("sender", SOMEBODY),
            ("proposal_id", proposal_id.to_string().as_str()),
        ],
    );

    // Trying to close it again fails
    let closing = ExecuteMsg::Close { proposal_id };
    let err = app
        .execute_contract(Addr::unchecked(SOMEBODY), dao_addr, &closing, &[])
        .unwrap_err();
    assert_eq!(ContractError::WrongCloseStatus {}, err.downcast().unwrap());
}

#[test]
fn test_close_works_with_refund() {
    let mut app = mock_app();
    let voting_period = Duration::Height(2000000);
    let threshold = Threshold::ThresholdQuorum {
        threshold: Decimal::percent(51),
        quorum: Decimal::percent(10),
    };
    let proposal_deposit_amount = Uint128::new(10);
    let (dao_addr, cw20_addr, _) = setup_test_case(
        &mut app,
        threshold.clone(),
        voting_period,
        coins(10, NATIVE_TOKEN_DENOM),
        Some(proposal_deposit_amount),
        Some(true),
    );

    let cw20 = Cw20Contract(cw20_addr.clone());

    let allowance = Cw20ExecuteMsg::IncreaseAllowance {
        spender: dao_addr.clone().into(),
        amount: proposal_deposit_amount,
        expires: None,
    };

    let res = app.execute_contract(Addr::unchecked(OWNER), cw20_addr.clone(), &allowance, &[]);
    assert!(res.is_ok());

    let owner_initial_balance = cw20.balance(&app, Addr::unchecked(OWNER)).unwrap();
    let proposal = pay_somebody_proposal();
    let res = app
        .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &proposal, &[])
        .unwrap();

    // Get the proposal id from the logs
    let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

    let closing = ExecuteMsg::Close { proposal_id };

    // Manually expire proposal to be able to close.
    app.update_block(expire(voting_period));
    let res = app
        .execute_contract(Addr::unchecked(SOMEBODY), dao_addr.clone(), &closing, &[])
        .unwrap();
    assert_eq!(
        res.custom_attrs(1),
        [
            ("action", "close"),
            ("sender", SOMEBODY),
            ("proposal_id", proposal_id.to_string().as_str()),
        ],
    );

    let owner_balance_after_failed_proposal = cw20.balance(&app, Addr::unchecked(OWNER)).unwrap();
    assert_eq!(owner_balance_after_failed_proposal, owner_initial_balance);

    // Update Config to not refund proposals
    let update_config_msg = ExecuteMsg::UpdateConfig(Config {
        name: "dao-dao".to_string(),
        description: "a great DAO!".to_string(),
        threshold,
        max_voting_period: voting_period,
        proposal_deposit: proposal_deposit_amount,
        refund_failed_proposals: Some(false),
    });

    let res = app.execute_contract(dao_addr.clone(), dao_addr.clone(), &update_config_msg, &[]);
    assert!(res.is_ok());

    let allowance = Cw20ExecuteMsg::IncreaseAllowance {
        spender: dao_addr.clone().into(),
        amount: proposal_deposit_amount,
        expires: None,
    };

    let res = app.execute_contract(Addr::unchecked(OWNER), cw20_addr, &allowance, &[]);
    assert!(res.is_ok());

    let res = app
        .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &proposal, &[])
        .unwrap();

    let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
    let closing = ExecuteMsg::Close { proposal_id };

    app.update_block(expire(voting_period));
    let res = app
        .execute_contract(Addr::unchecked(SOMEBODY), dao_addr, &closing, &[])
        .unwrap();

    assert_eq!(
        res.custom_attrs(1),
        [
            ("action", "close"),
            ("sender", SOMEBODY),
            ("proposal_id", proposal_id.to_string().as_str()),
        ],
    );

    assert_eq!(
        owner_initial_balance - proposal_deposit_amount,
        cw20.balance(&app, Addr::unchecked(OWNER)).unwrap()
    );
}

#[test]
fn quorum_enforced_even_if_absolute_threshold_met() {
    let mut app = mock_app();

    // 33% required for quora, which is 5 of the initial 15
    // 50% yes required to pass early (8 of the initial 15)
    let voting_period = Duration::Time(20000);
    let (dao_addr, _cw20_addr, _) = setup_test_case(
        &mut app,
        // note that 60% yes is not enough to pass without 20% no as well
        Threshold::ThresholdQuorum {
            threshold: Decimal::percent(50),
            quorum: Decimal::percent(80),
        },
        voting_period,
        coins(10, NATIVE_TOKEN_DENOM),
        None,
        None,
    );

    // create proposal
    let proposal = pay_somebody_proposal();
    let res = app
        .execute_contract(Addr::unchecked(VOTER1), dao_addr.clone(), &proposal, &[])
        .unwrap();

    // Get the proposal id from the logs
    let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();
    let prop_status = |app: &App| -> Status {
        let query_prop = QueryMsg::Proposal { proposal_id };
        let prop: ProposalResponse = app.wrap().query_wasm_smart(&dao_addr, &query_prop).unwrap();
        prop.status
    };
    assert_eq!(prop_status(&app), Status::Open);
    app.update_block(|block| block.height += 3);

    // reach 60% of yes votes, not enough to pass early (or late)
    let yes_vote = ExecuteMsg::Vote(VoteMsg {
        proposal_id,
        vote: Vote::Yes,
    });
    app.execute_contract(Addr::unchecked(VOTER1), dao_addr.clone(), &yes_vote, &[])
        .unwrap();
    app.execute_contract(Addr::unchecked(VOTER2), dao_addr.clone(), &yes_vote, &[])
        .unwrap();
    app.execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &yes_vote, &[])
        .unwrap();
    app.execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &yes_vote, &[])
        .unwrap();

    // 9 of 15 is 60% absolute threshold, but less than 12 (80% quorum needed)
    assert_eq!(prop_status(&app), Status::Open);

    // add 3 weight no vote and we hit quorum and this passes
    let no_vote = ExecuteMsg::Vote(VoteMsg {
        proposal_id,
        vote: Vote::No,
    });
    app.execute_contract(
        Addr::unchecked(POWER_VOTER),
        dao_addr.clone(),
        &no_vote,
        &[],
    )
    .unwrap();
    assert_eq!(prop_status(&app), Status::Passed);
}

#[test]
fn test_burn_does_not_change_proposal_query_response_threshold() {
    let mut app = mock_app();

    let voting_period = Duration::Time(2000000);
    let threshold = Threshold::ThresholdQuorum {
        threshold: Decimal::percent(20),
        quorum: Decimal::percent(10),
    };
    let (dao_addr, cw20_addr, _) = setup_test_case(
        &mut app,
        threshold,
        voting_period,
        coins(100, NATIVE_TOKEN_DENOM),
        None,
        None,
    );

    let proposal = pay_somebody_proposal();
    let res = app
        .execute_contract(Addr::unchecked(VOTER1), dao_addr.clone(), &proposal, &[])
        .unwrap();

    let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

    let query_prop = QueryMsg::Proposal { proposal_id };
    let prop: ProposalResponse = app.wrap().query_wasm_smart(&dao_addr, &query_prop).unwrap();

    let total_weight = match prop.threshold {
        ThresholdResponse::AbsoluteCount {
            weight: _,
            total_weight,
        } => total_weight,
        ThresholdResponse::AbsolutePercentage {
            percentage: _,
            total_weight,
        } => total_weight,
        ThresholdResponse::ThresholdQuorum {
            threshold: _,
            quorum: _,
            total_weight,
        } => total_weight,
    };

    assert_eq!(total_weight, Uint128::from(20000000u128));

    // Burn some tokens
    app.execute_contract(
        Addr::unchecked(VOTER1),
        cw20_addr,
        &cw20::Cw20ExecuteMsg::Burn {
            amount: Uint128::from(1u128),
        },
        &[],
    )
    .unwrap();

    let query_prop = QueryMsg::Proposal { proposal_id };
    let prop: ProposalResponse = app.wrap().query_wasm_smart(&dao_addr, &query_prop).unwrap();

    let total_weight = match prop.threshold {
        ThresholdResponse::AbsoluteCount {
            weight: _,
            total_weight,
        } => total_weight,
        ThresholdResponse::AbsolutePercentage {
            percentage: _,
            total_weight,
        } => total_weight,
        ThresholdResponse::ThresholdQuorum {
            threshold: _,
            quorum: _,
            total_weight,
        } => total_weight,
    };

    assert_eq!(total_weight, Uint128::from(20000000u128));
}

#[test]
fn query_proposal_tally() {
    let mut app = mock_app();

    let voting_period = Duration::Time(20000);
    let (dao_addr, _, _) = setup_test_case(
        &mut app,
        Threshold::ThresholdQuorum {
            threshold: Decimal::percent(50),
            quorum: Decimal::percent(80),
        },
        voting_period,
        coins(10, NATIVE_TOKEN_DENOM),
        None,
        None,
    );

    let assert_tally_is = |app: &mut App, proposal_id, expected| {
        let query_prop = QueryMsg::Tally { proposal_id };
        let prop: VoteTallyResponse = app.wrap().query_wasm_smart(&dao_addr, &query_prop).unwrap();
        assert_eq!(prop, expected);
    };

    // create proposal
    let proposal = pay_somebody_proposal();
    let res = app
        .execute_contract(Addr::unchecked(VOTER1), dao_addr.clone(), &proposal, &[])
        .unwrap();

    // Get the proposal id from the logs
    let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

    assert_tally_is(
        &mut app,
        proposal_id,
        VoteTallyResponse {
            status: Status::Open,
            threshold: ThresholdResponse::ThresholdQuorum {
                threshold: Decimal::percent(50),
                quorum: Decimal::percent(80),
                total_weight: Uint128::from(20000000u128),
            },
            quorum: Decimal::percent(0),
            total_votes: Uint128::zero(),
            total_weight: Uint128::from(20000000u128),
            votes: Votes::new(Uint128::zero()),
        },
    );

    // Vote yes with 20% of voting power. This will not pass the
    // proposal.
    let yes_vote = ExecuteMsg::Vote(VoteMsg {
        proposal_id,
        vote: Vote::Yes,
    });
    let res = app.execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &yes_vote, &[]);
    assert!(res.is_ok());

    assert_tally_is(
        &mut app,
        proposal_id,
        VoteTallyResponse {
            status: Status::Open,
            threshold: ThresholdResponse::ThresholdQuorum {
                threshold: Decimal::percent(50),
                quorum: Decimal::percent(80),
                total_weight: Uint128::from(20000000u128),
            },
            quorum: Decimal::percent(20),
            total_votes: Uint128::from(4000000u128),
            total_weight: Uint128::from(20000000u128),
            votes: Votes::new(Uint128::from(4000000u128)),
        },
    );

    // Vote abstain with 10% of voting power. This will not pass
    // the proposal.
    let yes_vote = ExecuteMsg::Vote(VoteMsg {
        proposal_id,
        vote: Vote::Abstain,
    });
    let res = app.execute_contract(Addr::unchecked(VOTER1), dao_addr.clone(), &yes_vote, &[]);
    assert!(res.is_ok());

    assert_tally_is(
        &mut app,
        proposal_id,
        VoteTallyResponse {
            status: Status::Open,
            threshold: ThresholdResponse::ThresholdQuorum {
                threshold: Decimal::percent(50),
                quorum: Decimal::percent(80),
                total_weight: Uint128::from(20000000u128),
            },
            quorum: Decimal::percent(30),
            total_votes: Uint128::from(6000000u128),
            total_weight: Uint128::from(20000000u128),
            votes: Votes {
                yes: Uint128::from(4000000u128),
                abstain: Uint128::from(2000000u128),
                no: Uint128::zero(),
                veto: Uint128::zero(),
            },
        },
    );

    // Vote yes with and additional 50% of voting power. This will
    // pass the proposal at 80% quorum and 70% yes.
    let yes_vote = ExecuteMsg::Vote(VoteMsg {
        proposal_id,
        vote: Vote::Yes,
    });
    let res = app.execute_contract(
        Addr::unchecked(POWER_VOTER),
        dao_addr.clone(),
        &yes_vote,
        &[],
    );
    assert!(res.is_ok());

    assert_tally_is(
        &mut app,
        proposal_id,
        VoteTallyResponse {
            status: Status::Passed,
            threshold: ThresholdResponse::ThresholdQuorum {
                threshold: Decimal::percent(50),
                quorum: Decimal::percent(80),
                total_weight: Uint128::from(20000000u128),
            },
            quorum: Decimal::percent(80),
            total_votes: Uint128::from(16000000u128),
            total_weight: Uint128::from(20000000u128),
            votes: Votes {
                yes: Uint128::from(14000000u128),
                abstain: Uint128::from(2000000u128),
                no: Uint128::zero(),
                veto: Uint128::zero(),
            },
        },
    );
}

#[test]
fn test_update_config() {
    let mut app = mock_app();

    let voting_period = Duration::Time(2000000);
    let threshold = Threshold::ThresholdQuorum {
        threshold: Decimal::percent(20),
        quorum: Decimal::percent(10),
    };
    let (dao_addr, cw20_addr, staking_addr) = setup_test_case(
        &mut app,
        threshold,
        voting_period,
        coins(100, NATIVE_TOKEN_DENOM),
        None,
        None,
    );

    // nobody can call call update contract method
    let new_threshold = Threshold::ThresholdQuorum {
        threshold: Decimal::percent(51),
        quorum: Decimal::percent(10),
    };
    let new_voting_period = Duration::Time(5000000);
    let new_proposal_deposit_amount = Uint128::from(10u8);
    let update_config_msg = ExecuteMsg::UpdateConfig(Config {
        name: "dao-dao-dao-dao-dao-dao-dao-dao-dao-dao-dao-dao".to_string(),
        description: "a really great DAO with emojis 💫 and a name that is really long!"
            .to_string(),
        threshold: new_threshold.clone(),
        max_voting_period: new_voting_period,
        proposal_deposit: new_proposal_deposit_amount,
        refund_failed_proposals: None,
    });
    let res = app.execute_contract(
        Addr::unchecked(VOTER1),
        dao_addr.clone(),
        &update_config_msg,
        &[],
    );
    assert!(res.is_err());
    let res = app.execute_contract(
        Addr::unchecked(OWNER),
        dao_addr.clone(),
        &update_config_msg,
        &[],
    );
    assert!(res.is_err());

    let wasm_msg = WasmMsg::Execute {
        contract_addr: dao_addr.clone().into(),
        msg: to_binary(&update_config_msg).unwrap(),
        funds: vec![],
    };

    // Update config proposal must be made
    let proposal_msg = ExecuteMsg::Propose(ProposeMsg {
        title: String::from("Change params"),
        description: String::from("Updates threshold and max voting params"),
        msgs: vec![wasm_msg.into()],
        latest: None,
    });
    let res = app
        .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &proposal_msg, &[])
        .unwrap();
    let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

    // Imediately passes on yes vote
    let yes_vote = ExecuteMsg::Vote(VoteMsg {
        proposal_id,
        vote: Vote::Yes,
    });
    let res = app.execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &yes_vote, &[]);
    assert!(res.is_ok());

    // Execute
    let execution = ExecuteMsg::Execute { proposal_id };
    let res = app.execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &execution, &[]);
    assert!(res.is_ok());

    // Check that config was updated
    let res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&dao_addr, &QueryMsg::GetConfig {})
        .unwrap();

    assert_eq!(
        res,
        ConfigResponse {
            config: Config {
                name: "dao-dao-dao-dao-dao-dao-dao-dao-dao-dao-dao-dao".to_string(),
                description: "a really great DAO with emojis 💫 and a name that is really long!"
                    .to_string(),
                threshold: new_threshold,
                max_voting_period: new_voting_period,
                proposal_deposit: new_proposal_deposit_amount,
                refund_failed_proposals: None,
            },
            gov_token: cw20_addr,
            staking_contract: staking_addr
        }
    )
}

#[test]
fn test_config_query() {
    let mut app = mock_app();

    let voting_period = Duration::Time(2000000);
    let threshold = Threshold::ThresholdQuorum {
        threshold: Decimal::percent(51),
        quorum: Decimal::percent(10),
    };
    let (dao_addr, cw20_addr, staking_addr) = setup_test_case(
        &mut app,
        threshold.clone(),
        voting_period,
        coins(100, NATIVE_TOKEN_DENOM),
        None,
        None,
    );

    let config_query = QueryMsg::GetConfig {};
    let res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&dao_addr, &config_query)
        .unwrap();

    assert_eq!(
        res,
        ConfigResponse {
            config: Config {
                name: "dao-dao".to_string(),
                description: "a great DAO!".to_string(),
                threshold,
                max_voting_period: voting_period,
                proposal_deposit: Uint128::zero(),
                refund_failed_proposals: None,
            },
            gov_token: cw20_addr,
            staking_contract: staking_addr
        }
    )
}

#[test]
fn test_proposal_deposit_works() {
    let mut app = mock_app();

    let voting_period = Duration::Time(2000000);
    let threshold = Threshold::AbsolutePercentage {
        percentage: Decimal::percent(20),
    };
    let (dao_addr, cw20_addr, _) = setup_test_case(
        &mut app,
        threshold.clone(),
        voting_period,
        coins(10, NATIVE_TOKEN_DENOM),
        None,
        None,
    );

    let cw20 = Cw20Contract(cw20_addr.clone());

    let initial_owner_cw20_balance = cw20.balance(&app, Addr::unchecked(OWNER)).unwrap();

    // ensure we have cash to cover the proposal
    let contract_bal = app
        .wrap()
        .query_balance(&dao_addr, NATIVE_TOKEN_DENOM)
        .unwrap();
    assert_eq!(contract_bal, coin(10, NATIVE_TOKEN_DENOM));

    let proposal_deposit_amount = Uint128::new(10);

    let update_config_msg = ExecuteMsg::UpdateConfig(Config {
        name: "dao-dao".to_string(),
        description: "a great DAO!".to_string(),
        threshold,
        max_voting_period: voting_period,
        proposal_deposit: proposal_deposit_amount,
        refund_failed_proposals: None,
    });
    let res = app.execute_contract(dao_addr.clone(), dao_addr.clone(), &update_config_msg, &[]);
    assert!(res.is_ok());

    // Give dao allowance for proposal
    let allowance = Cw20ExecuteMsg::IncreaseAllowance {
        spender: dao_addr.clone().into(),
        amount: proposal_deposit_amount,
        expires: None,
    };
    let res = app.execute_contract(Addr::unchecked(OWNER), cw20_addr, &allowance, &[]);
    assert!(res.is_ok());

    // create proposal with 0 vote power
    let proposal = pay_somebody_proposal();
    let res = app
        .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &proposal, &[])
        .unwrap();

    // Get the proposal id from the logs
    let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

    // Check proposal deposit was made
    let balance = cw20.balance(&app, Addr::unchecked(OWNER)).unwrap();
    let expected_balance = initial_owner_cw20_balance
        .checked_sub(proposal_deposit_amount)
        .unwrap();
    assert_eq!(balance, expected_balance);

    // Only Passed can be executed
    let execution = ExecuteMsg::Execute { proposal_id };
    let err = app
        .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &execution, &[])
        .unwrap_err();
    assert_eq!(
        ContractError::WrongExecuteStatus {},
        err.downcast().unwrap()
    );

    // Vote it, so it passes
    let vote = ExecuteMsg::Vote(VoteMsg {
        proposal_id,
        vote: Vote::Yes,
    });
    let res = app
        .execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &vote, &[])
        .unwrap();
    assert_eq!(
        res.custom_attrs(1),
        [
            ("action", "vote"),
            ("sender", VOTER3),
            ("proposal_id", proposal_id.to_string().as_str()),
            ("status", "Passed"),
        ],
    );

    // Execute works. Anybody can execute Passed proposals
    let res = app
        .execute_contract(Addr::unchecked(SOMEBODY), dao_addr, &execution, &[])
        .unwrap();
    assert_eq!(
        res.custom_attrs(1),
        [
            ("action", "execute"),
            ("sender", SOMEBODY),
            ("proposal_id", proposal_id.to_string().as_str()),
        ],
    );

    // Check deposit has been refunded
    let balance = cw20.balance(&app, Addr::unchecked(OWNER)).unwrap();
    assert_eq!(balance, initial_owner_cw20_balance);
}

#[test]
fn treasury_queries() {
    let mut app = mock_app();

    let voting_period = Duration::Time(2000000);
    let threshold = Threshold::AbsolutePercentage {
        percentage: Decimal::percent(20),
    };
    let (dao_addr, _, _) = setup_test_case(
        &mut app,
        threshold,
        voting_period,
        coins(10, NATIVE_TOKEN_DENOM),
        None,
        None,
    );

    // Query All Treasury Balances
    let cw20_token_balances_msg = QueryMsg::Cw20Balances {
        start_after: None,
        limit: None,
    };
    let all_balances: Cw20BalancesResponse = app
        .wrap()
        .query_wasm_smart(&dao_addr, &cw20_token_balances_msg)
        .unwrap();
    assert_eq!(
        all_balances.cw20_balances,
        vec![Cw20CoinVerified {
            address: Addr::unchecked("Contract #0"),
            amount: Uint128::zero(),
        }]
    );

    // Query token list
    let token_list: TokenListResponse = app
        .wrap()
        .query_wasm_smart(&dao_addr, &QueryMsg::Cw20TokenList {})
        .unwrap();
    assert_eq!(token_list.token_list.len(), 1);

    // Make a new token with initial balance
    let cw20_id = app.store_code(contract_cw20_gov());
    let msg = cw20_base::msg::InstantiateMsg {
        name: String::from("NewCoin"),
        symbol: String::from("COIN"),
        decimals: 6,
        initial_balances: vec![Cw20Coin {
            address: OWNER.to_string(),
            amount: Uint128::new(INITIAL_BALANCE * 5),
        }],
        mint: None,
        marketing: None,
    };
    let res = app.instantiate_contract(cw20_id, Addr::unchecked(OWNER), &msg, &[], "cw20", None);
    assert!(res.is_ok());
    let other_cw20_addr = res.unwrap();

    // Manually add token to list by voting
    let update_token_list_msg = ExecuteMsg::UpdateCw20TokenList {
        to_add: vec![Addr::unchecked("NEW"), Addr::unchecked("NEWNEW")],
        to_remove: vec![other_cw20_addr],
    };
    let wasm_msg = WasmMsg::Execute {
        contract_addr: dao_addr.clone().into(),
        msg: to_binary(&update_token_list_msg).unwrap(),
        funds: vec![],
    };
    let proposal_msg = ExecuteMsg::Propose(ProposeMsg {
        title: String::from("Change params"),
        description: String::from("Updates threshold and max voting params"),
        msgs: vec![wasm_msg.into()],
        latest: None,
    });
    let res = app
        .execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &proposal_msg, &[])
        .unwrap();
    let proposal_id: u64 = res.custom_attrs(1)[2].value.parse().unwrap();

    // Imediately passes on yes vote
    let yes_vote = ExecuteMsg::Vote(VoteMsg {
        proposal_id,
        vote: Vote::Yes,
    });
    let res = app.execute_contract(Addr::unchecked(VOTER3), dao_addr.clone(), &yes_vote, &[]);
    assert!(res.is_ok());

    // Execute
    let execution = ExecuteMsg::Execute { proposal_id };
    let res = app.execute_contract(Addr::unchecked(OWNER), dao_addr.clone(), &execution, &[]);
    assert!(res.is_ok());

    // Token list should be 3 now
    let token_list: TokenListResponse = app
        .wrap()
        .query_wasm_smart(&dao_addr, &QueryMsg::Cw20TokenList {})
        .unwrap();
    assert_eq!(token_list.token_list.len(), 3);

    // Contact #2 token has been removed
    assert!(!token_list
        .token_list
        .contains(&Addr::unchecked("Contract #2")));
}
