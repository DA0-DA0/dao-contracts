use cosmwasm_std::Addr;
use cw_multi_test::{App, Executor};
use dao_testing::contracts::cw721_base_contract;

pub fn instantiate_cw721_base(app: &mut App, sender: &str, minter: &str) -> Addr {
    let cw721_id = app.store_code(cw721_base_contract());

    app.instantiate_contract(
        cw721_id,
        Addr::unchecked(sender),
        &cw721_base::InstantiateMsg {
            name: "bad kids".to_string(),
            symbol: "bad kids".to_string(),
            minter: minter.to_string(),
        },
        &[],
        "cw721_base".to_string(),
        None,
    )
    .unwrap()
}
