mod execute;
mod instantiate;
mod queries;
mod tests;

use cosmwasm_std::Addr;
use cw_multi_test::{App, Executor};
use cw_utils::Duration;

use dao_interface::Admin;
use dao_testing::contracts::dao_voting_cw721_roles_contract;

use crate::msg::{InstantiateMsg, NftContract, NftMintMsg};

use self::instantiate::instantiate_cw721_roles;

/// Address used as the owner, instantiator, and minter.
pub(crate) const CREATOR_ADDR: &str = "creator";

pub(crate) struct CommonTest {
    app: App,
    module_addr: Addr,
    cw721_addr: Addr,
    module_id: u64,
    cw721_id: u64,
}

pub(crate) fn setup_test(initial_nfts: Vec<NftMintMsg>) -> CommonTest {
    let mut app = App::default();
    let module_id = app.store_code(dao_voting_cw721_roles_contract());

    let (cw721_addr, cw721_id) = instantiate_cw721_roles(&mut app, CREATOR_ADDR, CREATOR_ADDR);
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

    CommonTest {
        app,
        module_addr,
        module_id,
        cw721_addr,
        cw721_id,
    }
}

// Advantage to using a macro for this is that the error trace links
// to the exact line that the error occured, instead of inside of a
// function where the assertion would otherwise happen.
macro_rules! is_error {
    ($x:expr => $e:tt) => {
        assert!(format!("{:#}", $x.unwrap_err()).contains($e))
    };
}

pub(crate) use is_error;
