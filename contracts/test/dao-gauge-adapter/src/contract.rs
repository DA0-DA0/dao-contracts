

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    coin, to_json_binary, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo,
    Order, Response, StdError, StdResult,
};
use cw_storage_plus::{Item, Map};


use gauge_orchestrator::msg::{
    AdapterQueryMsg, AllOptionsResponse, CheckOptionResponse, SampleGaugeMsgsResponse,
};

#[cw_serde]
pub struct InstantiateMsg {
    pub options: Vec<String>,
    pub to_distribute: Coin,
}

#[cw_serde]
#[derive(cw_orch::ExecuteFns)]
pub enum ExecuteMsg {
    InvalidateOption { option: String },
    AddValidOption { option: String },
}

#[cw_serde]
struct EmptyMsg {}

const OPTIONS: Map<String, bool> = Map::new("options");
const TO_DISTRIBUTE: Item<Coin> = Item::new("to_spend");

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, StdError> {
    msg.options
        .into_iter()
        .try_for_each(|option| OPTIONS.save(deps.storage, option, &true))?;
    TO_DISTRIBUTE.save(deps.storage, &msg.to_distribute)?;
    Ok(Response::default())
}

pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, StdError> {
    match msg {
        ExecuteMsg::InvalidateOption { option } => {
            OPTIONS.remove(deps.storage, option);
        }
        ExecuteMsg::AddValidOption { option } => {
            OPTIONS.save(deps.storage, option, &true)?;
        }
    }
    Ok(Response::new())
}

pub fn query(deps: Deps, _env: Env, msg: AdapterQueryMsg) -> Result<Binary, StdError> {
    match msg {
        AdapterQueryMsg::AllOptions {} => to_json_binary(&AllOptionsResponse {
            options: OPTIONS
                .keys(deps.storage, None, None, Order::Ascending)
                .collect::<StdResult<Vec<_>>>()?,
        }),
        AdapterQueryMsg::CheckOption { option } => to_json_binary(&CheckOptionResponse {
            valid: OPTIONS.has(deps.storage, option),
        }),
        AdapterQueryMsg::SampleGaugeMsgs { selected } => {
            let to_distribute = TO_DISTRIBUTE.load(deps.storage)?;
            let mut weights_sum = Decimal::zero();
            let execute = selected
                .into_iter()
                .map(|(option, weight)| {
                    weights_sum += weight;
                    CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
                        to_address: option,
                        amount: vec![coin(
                            (to_distribute.amount * weight).u128(),
                            to_distribute.denom.clone(),
                        )],
                    })
                })
                .collect::<Vec<CosmosMsg>>();
            to_json_binary(&SampleGaugeMsgsResponse { execute })
        }
    }
}