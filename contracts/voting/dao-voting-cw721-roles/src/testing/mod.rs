mod execute;
mod instantiate;
mod queries;
mod tests;

use cosmwasm_std::Addr;
use cw_multi_test::{App, Executor};
use dao_testing::contracts::dao_voting_cw721_roles_contract;

use crate::msg::{InstantiateMsg, NftContract, NftMintMsg};

use self::instantiate::instantiate_cw721_roles;

/// Address used as the owner, instantiator, and minter.
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
                },
            },
            &[],
            "cw721_voting",
            None,
        )
        .unwrap();

    CommonTest { app, module_addr }
}
