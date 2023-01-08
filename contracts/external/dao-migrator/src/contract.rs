#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, WasmMsg,
};
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::CodeIdPair,
};

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-migrator";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
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
        ExecuteMsg::MigrateV1ToV2 {
            migrate_stake_cw20_manager: _,
        } => unimplemented!(),
        ExecuteMsg::Conjunction { operands: _ } => unimplemented!(),
    }
}

fn execute_migration_v1_v2(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    // TODO: Probably should be dynamic?
    // TODO: Add params checks for the migrateMsg, to make sure we got the msg correctly.
    // List of matching code ids (TESTNET) and the migration msg of each one of them.
    let code_ids: Vec<CodeIdPair> = vec![
        CodeIdPair::new(
            453,
            3463,
            dao_proposal_single::msg::MigrateMsg::FromV1 {
                close_proposal_on_execution_failure: false,
                pre_propose_info: dao_voting::pre_propose::PreProposeInfo::AnyoneMayPropose {},
            },
        ), // cw-proposal-single -> dao_proposal_single
           // CodeIdPair::new(452, 3457), // cw-core -> dao_core
           // CodeIdPair::new(450, 3465), // cw4-voting -> dao_voting_cw4
           // CodeIdPair::new(449, 3454), // cw20-stake -> cw20_stake
           // CodeIdPair::new(451, 3468), // cw20-staked-balances-voting -> dao_voting_staking_denom_staked (TODO: correct?)
           // CodeIdPair::new(77, 3471),  // cw20_base -> cw20_base
           // CodeIdPair::new(78, 3472),  // cw4_group -> cw4_group
    ];
    let mut msgs: Vec<WasmMsg> = vec![];
    let error: Option<ContractError> = None;

    // We take all the modules of the DAO.
    let modules: Vec<dao_core::state::ProposalModule> = deps.querier.query_wasm_smart(
        info.sender,
        &dao_core::msg::QueryMsg::ProposalModules {
            start_after: None,
            limit: None,
        },
    )?;

    let success = modules.into_iter().all(|module| {
        // Get the code id of the module
        // Return false if we don't get contract info, means something went wrong.
        let code_id =
            if let Ok(contract_info) = deps.querier.query_wasm_contract_info(module.address.clone()) {
                contract_info.code_id
            } else {
                // TODO: return meaningful error
                return false;
            };

        //TODO: pretty sure theres a better way of doing the below checks and msg creation
        // Make sure moduel code id is one of DAO DAOs code ids
        if let Some(codePair) = code_ids.iter().find(|x| x.v1_code_id == code_id) {
            // Code id is valid DAO DAO code id, lets create a migration msg

            msgs.push(WasmMsg::Migrate {
                contract_addr: module.address.to_string(),
                new_code_id: codePair.v2_code_id,
                msg: to_binary(&codePair.migrate_msg).unwrap(),
            })
        } else {
            return false;
        }

        true
    });
    // Get all DAO modules
    // 1. Loop over all modules of a DAO and get the code id
    // 2. Make sure all code ids, match to DAO DAOs code ids
    // 3. Fail if 1 module doesn't match code ids.
    // While you do the loop, create migration msgs for later.

    // After we successfully made sure we only have DAO DAO modules,
    // we query the state of the DAO for check after migration.
    // We save the old_state in this contract to use in reply.
    // We call ourselves with `Conjunction` with all migration msgs.

    // DONE, we continue in reply to the `Conjunction`

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {}
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    // let repl = TaggedReplyId::new(msg.id)?;
    // match repl {}
    Ok(Response::default())
}
