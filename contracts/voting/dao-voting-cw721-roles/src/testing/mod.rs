mod execute;
mod instantiate;
mod queries;
mod tests;

use cosmwasm_std::Addr;
use cw_multi_test::{App, Executor};
use cw_ownable::Action;
use dao_cw721_extensions::roles::{ExecuteExt, MetadataExt};
use dao_testing::contracts::dao_voting_cw721_roles_contract;

use crate::msg::{InstantiateMsg, NftContract, NftMintMsg};
use crate::testing::queries::query_config;

use self::instantiate::instantiate_cw721_roles;

/// Address used as the owner, instantiator, and minter. In a real deployment
/// this address would be the dao-dao-core; here we play that role with a
/// normal account so cw_multi_test can drive the test.
pub(crate) const CREATOR_ADDR: &str = "creator";

pub(crate) struct CommonTest {
    app: App,
    module_addr: Addr,
}

pub(crate) fn setup_test(initial_nfts: Vec<NftMintMsg>) -> CommonTest {
    let mut app = App::default();
    let module_id = app.store_code(dao_voting_cw721_roles_contract());

    let (_, cw721_id) = instantiate_cw721_roles(&mut app, CREATOR_ADDR, CREATOR_ADDR);
    let module_addr = app
        .instantiate_contract(
            module_id,
            Addr::unchecked(CREATOR_ADDR),
            &InstantiateMsg {
                nft_contract: NftContract::New {
                    code_id: cw721_id,
                    label: "cw721-roles".to_string(),
                    name: "Job Titles".to_string(),
                    symbol: "TITLES".to_string(),
                    initial_nfts,
                    salt: None,
                },
            },
            &[],
            "cw721_voting",
            None,
        )
        .unwrap();

    // In production, the dao-dao-core's VOTE_MODULE_INSTANTIATE_REPLY_ID
    // handler decodes the voting module's reply data as a
    // ModuleInstantiateCallback and dispatches its msgs — one of which is
    // AcceptOwnership on the new cw721-roles, completing the two-phase
    // cw-ownable handover so the DAO becomes the cw721-roles owner.
    // cw_multi_test does not chain reply data into the parent the same way,
    // so we explicitly accept ownership here as CREATOR_ADDR (which played
    // the role of dao-core during the voting module's instantiate).
    let config = query_config(&app, &module_addr)
        .expect("voting module should expose config immediately after instantiate");
    let cw721_addr = config.nft_address;
    // .unwrap() — NOT `let _ =`. If AcceptOwnership fails here it means the
    // bootstrap handover from voting module → DAO is broken, and every
    // downstream test that mints, burns, or queries voting power is reasoning
    // about state that doesn't exist on chain. Make that loud.
    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        cw721_addr.clone(),
        &cw721_base::ExecuteMsg::<MetadataExt, ExecuteExt>::UpdateOwnership(
            Action::AcceptOwnership {},
        ),
        &[],
    )
    .expect("DAO (CREATOR_ADDR) AcceptOwnership on cw721-roles should succeed — if this fires, the cw_ownable two-phase handover is broken");

    CommonTest { app, module_addr }
}
