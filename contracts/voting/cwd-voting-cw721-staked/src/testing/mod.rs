mod adversarial;
mod execute;
mod instantiate;
mod queries;
mod tests;

use cosmwasm_std::Addr;
use cw_multi_test::{App, Executor};
use cw_utils::Duration;

use cwd_interface::Admin;
use cwd_testing::contracts::voting_cw721_staked_contract;

use crate::msg::InstantiateMsg;

use self::instantiate::instantiate_cw721_base;

/// Address used as the owner, instantiator, and minter.
pub(crate) const CREATOR_ADDR: &str = "creator";

pub(crate) struct CommonTest {
    app: App,
    module: Addr,
    nft: Addr,
}

pub(crate) fn setup_test(owner: Option<Admin>, unstaking_duration: Option<Duration>) -> CommonTest {
    let mut app = App::default();
    let module_id = app.store_code(voting_cw721_staked_contract());

    let nft = instantiate_cw721_base(&mut app, CREATOR_ADDR, CREATOR_ADDR);
    let module = app
        .instantiate_contract(
            module_id,
            Addr::unchecked(CREATOR_ADDR),
            &InstantiateMsg {
                owner,
                nft_address: nft.to_string(),
                unstaking_duration,
            },
            &[],
            "cw721_voting",
            None,
        )
        .unwrap();
    CommonTest { app, module, nft }
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
