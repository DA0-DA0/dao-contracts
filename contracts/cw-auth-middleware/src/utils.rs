use cosmwasm_std::{Addr, CosmosMsg, Deps};

use crate::msg::{IsAuthorizedResponse, QueryMsg};

pub fn check_authorization(
    deps: Deps,
    auths: &Vec<Addr>,
    msgs: &Vec<CosmosMsg>,
    sender: &Addr,
) -> bool {
    // Right now this defaults to an *or*. We should update the contract to
    // support a custom allow/reject behaviour (similarly to how it's done in
    // message-filter)
    auths.into_iter().any(|a| {
        deps.querier
            .query_wasm_smart(
                a.clone(),
                &QueryMsg::Authorize {
                    msgs: msgs.clone(),
                    sender: sender.clone(),
                },
            )
            .unwrap_or(IsAuthorizedResponse { authorized: false })
            .authorized
    })
}
