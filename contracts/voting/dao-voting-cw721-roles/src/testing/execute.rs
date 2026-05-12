use cosmwasm_std::{Addr, Empty};
use cw_multi_test::{App, AppResponse, Executor};
use dao_cw721_extensions::roles::{ExecuteExt, MetadataExt};

use anyhow::Result as AnyResult;

pub fn mint_nft(
    app: &mut App,
    cw721: &Addr,
    sender: &str,
    receiver: &str,
    token_id: &str,
) -> AnyResult<AppResponse> {
    app.execute_contract(
        Addr::unchecked(sender),
        cw721.clone(),
        &cw721::msg::Cw721ExecuteMsg::<MetadataExt, Empty, ExecuteExt>::Mint {
            token_id: token_id.to_string(),
            owner: receiver.to_string(),
            token_uri: None,
            extension: MetadataExt {
                role: Some("admin".to_string()),
                weight: 1,
            },
        },
        &[],
    )
}
