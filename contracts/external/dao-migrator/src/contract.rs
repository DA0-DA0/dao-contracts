use std::env;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, SubMsg,
    WasmMsg,
};
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{MIGRATION_PARAMS, MODULES_ADDRS, TEST_STATE},
    types::{CodeIdPair, MigrationMsgs, ModulesAddrs, TestState},
    utils::state_queries::{
        query_core_dump_state_v1, query_core_dump_state_v2, query_core_items_v1,
        query_core_items_v2, query_proposal_count_v1, query_proposal_count_v2, query_proposal_v1,
        query_proposal_v2, query_single_voting_power_v1, query_single_voting_power_v2,
        query_total_voting_power_v1, query_total_voting_power_v2,
    },
};

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-migrator";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) const CONJUCTION_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    MIGRATION_PARAMS.save(deps.storage, &msg.migration_params)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::MigrateV1ToV2 {} => execute_migration_v1_v2(deps, env, info),
        ExecuteMsg::Conjunction { operands } => Ok(Response::default().add_messages(operands)),
    }
}

fn execute_migration_v1_v2(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let migration_params = MIGRATION_PARAMS.load(deps.storage)?;
    // List of matching code ids (TESTNET) and the migration msg of each one of them.
    let code_ids: Vec<CodeIdPair> = vec![
        CodeIdPair::new(
            453,
            3463,
            MigrationMsgs::DaoProposalSingle(dao_proposal_single::msg::MigrateMsg::FromV1 {
                close_proposal_on_execution_failure: migration_params
                    .close_proposal_on_execution_failure,
                pre_propose_info: migration_params.pre_propose_info,
            }),
        ), // cw-proposal-single -> dao_proposal_single
        CodeIdPair::new(
            452,
            3457,
            MigrationMsgs::DaoCore(dao_core::msg::MigrateMsg::FromV1 {
                dao_uri: migration_params.dao_uri,
            }),
        ), // cw-core -> dao_core
        CodeIdPair::new(
            450,
            3465,
            MigrationMsgs::DaoVotingCw4(dao_voting_cw4::msg::MigrateMsg {}),
        ), // cw4-voting -> dao_voting_cw4
        CodeIdPair::new(
            449,
            3454,
            MigrationMsgs::Cw20Stake(cw20_stake::msg::MigrateMsg::FromV1 {}),
        ), // cw20-stake -> cw20_stake
        CodeIdPair::new(
            451,
            3464,
            MigrationMsgs::DaoVotingCw20Staked(dao_voting_cw20_staked::msg::MigrateMsg {}),
        ), // cw20-staked-balances-voting -> dao-voting-cw20-staked
    ];
    let mut msgs: Vec<WasmMsg> = vec![];
    let mut error: Option<ContractError> = None;
    let mut modules_addrs = ModulesAddrs::new();

    // We take all the modules of the DAO.
    let modules: Vec<Addr> = deps.querier.query_wasm_smart(
        info.sender,
        &cw_core_v1::msg::QueryMsg::ProposalModules {
            start_at: None,
            limit: None,
        },
    )?;

    let success = modules.into_iter().all(|module| {
        // Get the code id of the module
        let code_id =
            if let Ok(contract_info) = deps.querier.query_wasm_contract_info(module.clone()) {
                contract_info.code_id
            } else {
                // Return false if we don't get contract info, means something went wrong.
                error = Some(ContractError::NoContractInfo {
                    address: module.into(),
                });
                return false;
            };

        //TODO: pretty sure theres a better way of doing the below checks and msg creation
        // Make sure module code id is one of DAO DAOs code ids
        if let Some(code_pair) = code_ids.iter().find(|x| x.v1_code_id == code_id) {
            // Code id is valid DAO DAO code id, lets create a migration msg

            msgs.push(WasmMsg::Migrate {
                contract_addr: module.to_string(),
                new_code_id: code_pair.v2_code_id,
                msg: to_binary(&code_pair.migrate_msg).unwrap(),
            });

            // Add rules per module type based on what migration msg we got.
            match code_pair.migrate_msg {
                MigrationMsgs::DaoCore(_) => modules_addrs.core = Some(module),
                MigrationMsgs::DaoProposalSingle(_) => modules_addrs.proposals.push(module),
                MigrationMsgs::Cw20Stake(_) => {
                    // Confirm they want to migrate cw20_stake
                    if !migration_params
                        .migrate_stake_cw20_manager
                        .unwrap_or_default()
                    {
                        error = Some(ContractError::DontMigrateCw20);
                        return false;
                    }
                }
                _ => (),
            }
        } else {
            // Return false because we couldn't find the code id on our list.
            error = Some(ContractError::CantMigrateModule { code_id });
            return false;
        }

        true
    });

    if !success {
        return Err(error.unwrap());
    } else {
        // We successfully verified all modules of the DAO, we can send migration msgs.

        // Verify we got core address, and at least 1 proposal single address
        modules_addrs.verify()?;
        MODULES_ADDRS.save(deps.storage, &modules_addrs)?;
        // Do the state query, and save it in storage
        let state = query_state_v1(deps.as_ref(), modules_addrs)?;
        TEST_STATE.save(deps.storage, &state)?;

        // Create the conjuction msg.
        let conjuction_msg = SubMsg::reply_on_success(
            WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::Conjunction { operands: msgs })?,
                funds: vec![],
            },
            CONJUCTION_REPLY_ID,
        );

        return Ok(Response::default().add_submessage(conjuction_msg));
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {}
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        CONJUCTION_REPLY_ID => {
            let old_state = TEST_STATE.load(deps.storage)?;
            let modules_addrs = MODULES_ADDRS.load(deps.storage)?;
            test_state(deps.as_ref(), old_state, modules_addrs)?;

            Ok(Response::default()
                .add_attribute("action", "migrate")
                .add_attribute("status", "success"))
        }
        _ => Err(ContractError::UnrecognisedReplyId),
    }
}

// TODO: Refactor to match queries based on passed version? or leave it like that? 
// We can pass the version we want to query to a single function and let the function handle the right call to make.
fn query_state_v1(deps: Deps, module_addrs: ModulesAddrs) -> Result<TestState, ContractError> {
    // Queries needs to do
    // 1. `query_dump_state` - query dao-core (`DumpState`)to get the `proposal_modules`, `voting_module`, and `total_proposal_module_count`
    // 2. `query_items` - query dao-core `ListItems`.
    // 3. `query_proposal_count` - query all proposal modules with `ProposalCount`
    // 4. `query_proposal` - query all proposal modules with `ReverseProposals`, get 1st proposal, convert it from v1 to v2.
    // 5. `query_total_power` - query voting module for `TotalPowerAtHeight`
    // 6. `query_voting_power` - query proposer at start height with `VotingPowerAtHeight`
    let core_dump_state = query_core_dump_state_v1(deps, module_addrs.core.as_ref().unwrap())?;
    let core_items = query_core_items_v1(deps, module_addrs.core.as_ref().unwrap())?;
    let proposal_counts = query_proposal_count_v1(deps, module_addrs.proposals.clone())?;
    let (proposals, last_proposal_data) = query_proposal_v1(deps, module_addrs.proposals.clone())?;
    let total_voting_power = query_total_voting_power_v1(
        deps,
        core_dump_state.voting_module.clone(),
        last_proposal_data.start_height,
    )?;
    let single_voting_power = query_single_voting_power_v1(
        deps,
        core_dump_state.voting_module.clone(),
        last_proposal_data.proposer,
        last_proposal_data.start_height,
    )?;

    Ok(TestState {
        core_dump_state,
        core_items,
        proposal_counts,
        proposals,
        total_voting_power,
        single_voting_power,
    })
}

fn query_state_v2(deps: Deps, module_addrs: ModulesAddrs) -> Result<TestState, ContractError> {
    let core_dump_state = query_core_dump_state_v2(deps, module_addrs.core.as_ref().unwrap())?;
    let core_items = query_core_items_v2(deps, module_addrs.core.as_ref().unwrap())?;
    let proposal_counts = query_proposal_count_v2(deps, module_addrs.proposals.clone())?;
    let (proposals, last_proposal_data) = query_proposal_v2(deps, module_addrs.proposals.clone())?;
    let total_voting_power = query_total_voting_power_v2(
        deps,
        core_dump_state.voting_module.clone(),
        last_proposal_data.start_height,
    )?;
    let single_voting_power = query_single_voting_power_v2(
        deps,
        core_dump_state.voting_module.clone(),
        last_proposal_data.proposer,
        last_proposal_data.start_height,
    )?;

    Ok(TestState {
        core_dump_state,
        core_items,
        proposal_counts,
        proposals,
        total_voting_power,
        single_voting_power,
    })
}

fn test_state(
    deps: Deps,
    old_state: TestState,
    modules_addrs: ModulesAddrs,
) -> Result<(), ContractError> {
    let new_state = query_state_v2(deps, modules_addrs)?;

    if new_state == old_state {
        Ok(())
    } else {
        Err(ContractError::TestFailed)
    }
}
