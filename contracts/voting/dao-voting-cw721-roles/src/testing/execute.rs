use cosmwasm_std::{Addr, Binary, Empty};
use cw721::Cw721ExecuteMsg;
use cw721_roles::msg::{ExecuteExt, MetadataExt};
use cw_multi_test::{App, AppResponse, Executor};

use anyhow::Result as AnyResult;
use cw_utils::Duration;

use crate::msg::ExecuteMsg;

// Shorthand for an unchecked address.
macro_rules! addr {
    ($x:expr ) => {
        Addr::unchecked($x)
    };
}

pub fn send_nft(
    app: &mut App,
    cw721: &Addr,
    sender: &str,
    receiver: &Addr,
    token_id: &str,
    msg: Binary,
) -> AnyResult<AppResponse> {
    app.execute_contract(
        addr!(sender),
        cw721.clone(),
        &Cw721ExecuteMsg::SendNft {
            contract: receiver.to_string(),
            token_id: token_id.to_string(),
            msg,
        },
        &[],
    )
}

pub fn mint_nft(
    app: &mut App,
    cw721: &Addr,
    sender: &str,
    receiver: &str,
    token_id: &str,
) -> AnyResult<AppResponse> {
    app.execute_contract(
        addr!(sender),
        cw721.clone(),
        &cw721_base::ExecuteMsg::<MetadataExt, ExecuteExt>::Mint {
            token_id: token_id.to_string(),
            owner: receiver.to_string(),
            token_uri: None,
            extension: MetadataExt { weight: 1 },
        },
        &[],
    )
}

pub fn update_config(
    app: &mut App,
    module: &Addr,
    sender: &str,
    owner: Option<&str>,
    duration: Option<Duration>,
) -> AnyResult<AppResponse> {
    app.execute_contract(
        addr!(sender),
        module.clone(),
        &ExecuteMsg::UpdateConfig {
            owner: owner.map(str::to_string),
        },
        &[],
    )
}

pub fn add_hook(app: &mut App, module: &Addr, sender: &str, hook: &str) -> AnyResult<AppResponse> {
    app.execute_contract(
        addr!(sender),
        module.clone(),
        &ExecuteMsg::AddHook {
            addr: hook.to_string(),
        },
        &[],
    )
}

pub fn remove_hook(
    app: &mut App,
    module: &Addr,
    sender: &str,
    hook: &str,
) -> AnyResult<AppResponse> {
    app.execute_contract(
        addr!(sender),
        module.clone(),
        &ExecuteMsg::RemoveHook {
            addr: hook.to_string(),
        },
        &[],
    )
}
