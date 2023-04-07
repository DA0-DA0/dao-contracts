use cosmwasm_std::{Addr, Binary, Empty};
use cw721::Cw721ExecuteMsg;
use cw721_roles::msg::{ExecuteExt, MetadataExt};
use cw_multi_test::{App, AppResponse, Executor};

use anyhow::Result as AnyResult;
use cw_utils::Duration;

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
