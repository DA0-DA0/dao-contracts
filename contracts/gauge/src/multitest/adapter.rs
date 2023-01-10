//! Gauge adapter contract to mock in tests.
//! I wrote it so that InstantiateMsg contains list of initially
//! available options. Query for CheckOption checks if option is already added,
//! otherwise returns true - option is valid.

use cosmwasm_std::{
    coin, to_binary, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Empty, Env, MessageInfo,
    Order, Response, StdError, StdResult,
};
use cw_multi_test::{Contract, ContractWrapper};
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

use crate::msg::{
    AdapterQueryMsg, AllOptionsResponse, CheckOptionResponse, SampleGaugeMsgsResponse,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstantiateMsg {
    pub options: Vec<String>,
    pub to_distribute: Coin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EmptyMsg {}

const OPTIONS: Map<String, bool> = Map::new("options");
const TO_DISTRIBUTE: Item<Coin> = Item::new("to_spend");

fn instantiate(
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

fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: EmptyMsg,
) -> Result<Response, StdError> {
    Ok(Response::new())
}

fn query(deps: Deps, _env: Env, msg: AdapterQueryMsg) -> Result<Binary, StdError> {
    match msg {
        AdapterQueryMsg::AllOptions {} => to_binary(&AllOptionsResponse {
            options: OPTIONS
                .keys(deps.storage, None, None, Order::Ascending)
                .collect::<StdResult<Vec<_>>>()?,
        }),
        AdapterQueryMsg::CheckOption { option: _ } => {
            to_binary(&CheckOptionResponse { valid: true })
        }
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
            to_binary(&SampleGaugeMsgsResponse { execute })
        }
    }
}

pub fn contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}
