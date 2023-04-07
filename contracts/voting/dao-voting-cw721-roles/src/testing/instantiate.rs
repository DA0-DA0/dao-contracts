use cosmwasm_std::Addr;
use cw_multi_test::{App, Executor};
use dao_testing::contracts::cw721_roles_contract;

pub fn instantiate_cw721_roles(app: &mut App, sender: &str, minter: &str) -> (Addr, u64) {
    let cw721_id = app.store_code(cw721_roles_contract());

    let cw721_addr = app
        .instantiate_contract(
            cw721_id,
            Addr::unchecked(sender),
            &cw721_base::InstantiateMsg {
                name: "bad kids".to_string(),
                symbol: "bad kids".to_string(),
                minter: minter.to_string(),
            },
            &[],
            "cw721_roles".to_string(),
            None,
        )
        .unwrap();

    (cw721_addr, cw721_id)
}
