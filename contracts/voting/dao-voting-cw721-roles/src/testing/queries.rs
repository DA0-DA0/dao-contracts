use cosmwasm_std::{Addr, StdResult};
use cw_multi_test::App;
use dao_cw721_extensions::roles::QueryExt;
use dao_interface::voting::{
    InfoResponse, TotalPowerAtHeightResponse, VotingPowerAtHeightResponse,
};

use crate::{msg::QueryMsg, state::Config};

pub fn query_config(app: &App, module: &Addr) -> StdResult<Config> {
    let config = app.wrap().query_wasm_smart(module, &QueryMsg::Config {})?;
    Ok(config)
}

pub fn query_voting_power(
    app: &App,
    module: &Addr,
    addr: &str,
    height: Option<u64>,
) -> StdResult<VotingPowerAtHeightResponse> {
    let power = app.wrap().query_wasm_smart(
        module,
        &QueryMsg::VotingPowerAtHeight {
            address: addr.to_string(),
            height,
        },
    )?;
    Ok(power)
}

pub fn query_total_power(
    app: &App,
    module: &Addr,
    height: Option<u64>,
) -> StdResult<TotalPowerAtHeightResponse> {
    let power = app
        .wrap()
        .query_wasm_smart(module, &QueryMsg::TotalPowerAtHeight { height })?;
    Ok(power)
}

pub fn query_info(app: &App, module: &Addr) -> StdResult<InfoResponse> {
    let info = app.wrap().query_wasm_smart(module, &QueryMsg::Info {})?;
    Ok(info)
}

pub fn query_minter(app: &App, nft: &Addr) -> StdResult<cw721_base::MinterResponse> {
    let minter = app
        .wrap()
        .query_wasm_smart(nft, &cw721_base::QueryMsg::<QueryExt>::Minter {})?;
    Ok(minter)
}
