use cosmwasm_std::{CosmosMsg, Deps, StdError, StdResult};
use omniflix_std::types::omniflix::onft::v1beta1::{MsgTransferOnft, OnftQuerier};

pub fn query_onft_owner(deps: Deps, denom_id: &str, token_id: &str) -> StdResult<String> {
    let res = OnftQuerier::new(&deps.querier).onft(denom_id.to_string(), token_id.to_string())?;
    let owner = res
        .onft
        .ok_or(StdError::generic_err("ONFT not found"))?
        .owner;

    Ok(owner)
}

pub fn query_onft_supply(deps: Deps, id: &str) -> StdResult<u64> {
    let res = OnftQuerier::new(&deps.querier).supply(id.to_string(), "".to_string())?;
    Ok(res.amount)
}

pub fn get_onft_transfer_msg(
    denom_id: &str,
    token_id: &str,
    sender: &str,
    recipient: &str,
) -> CosmosMsg {
    MsgTransferOnft {
        denom_id: denom_id.to_string(),
        id: token_id.to_string(),
        sender: sender.to_string(),
        recipient: recipient.to_string(),
    }
    .into()
}
