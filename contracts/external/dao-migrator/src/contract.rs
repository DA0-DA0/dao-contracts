#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, WasmMsg,
};
use cw2::set_contract_version;

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::{CodeIdPair, MigrationMsgs, MIGRATION_PARAMS},
};

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-migrator";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::MigrateV1ToV2 {
            migrate_stake_cw20_manager,
        } => execute_migration_v1_v2(deps, info, migrate_stake_cw20_manager.unwrap_or(false)),
        ExecuteMsg::Conjunction { operands: _ } => unimplemented!(),
    }
}

fn execute_migration_v1_v2(
    deps: DepsMut,
    info: MessageInfo,
    _migrate_stake_cw20_manager: bool,
) -> Result<Response, ContractError> {
    let migration_params = MIGRATION_PARAMS.load(deps.storage)?;
    // TODO: Add params checks for the migrateMsg, to make sure we got the msg correctly.
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
        let code_id = if let Ok(contract_info) = deps
            .querier
            .query_wasm_contract_info(module.address.clone())
        {
            contract_info.code_id
        } else {
            // Return false if we don't get contract info, means something went wrong.
            error = Some(ContractError::NoContractInfo {
                prefix: module.prefix,
                address: module.address.into(),
            });
            return false;
        };

        //TODO: pretty sure theres a better way of doing the below checks and msg creation
        // Make sure module code id is one of DAO DAOs code ids
        if let Some(code_pair) = code_ids.iter().find(|x| x.v1_code_id == code_id) {
            // Code id is valid DAO DAO code id, lets create a migration msg

            msgs.push(WasmMsg::Migrate {
                contract_addr: module.address.to_string(),
                new_code_id: code_pair.v2_code_id,
                msg: to_binary(&code_pair.migrate_msg).unwrap(),
            })
        } else {
            // Return false because we couldn't find the code id on our list.
            error = Some(ContractError::CantMigrateModule {
                prefix: module.prefix,
                code_id,
            });
            return false;
        }

        true
    });

    if !success {
        return Err(error.unwrap())
    } else {
        // We successfully verified all modules of the DAO, we can send migration msgs.
    }

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
pub fn reply(_deps: DepsMut, _env: Env, _msg: Reply) -> Result<Response, ContractError> {
    // let repl = TaggedReplyId::new(msg.id)?;
    // match repl {}
    Ok(Response::default())
}
