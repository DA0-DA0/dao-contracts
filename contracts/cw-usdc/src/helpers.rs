use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, CustomQuery, Deps, DepsMut, MessageInfo, Querier,
    QuerierWrapper, StdResult, Uint128, WasmMsg, WasmQuery,
};

use crate::{
    msg::{ExecuteMsg, QueryMsg},
    state::CONFIG,
    ContractError,
};

/// CwTemplateContract is a wrapper around Addr that provides a lot of helpers
/// for working with this.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CwTemplateContract(pub Addr);

impl CwTemplateContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
        let msg = to_binary(&msg.into())?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds: vec![],
        }
        .into())
    }

    // /// Get Count
    // pub fn count<Q, T, CQ>(&self, querier: &Q) -> StdResult<CountResponse>
    // where
    //     Q: Querier,
    //     T: Into<String>,
    //     CQ: CustomQuery,
    // {
    //     let msg = QueryMsg::GetCount {};
    //     let query = WasmQuery::Smart {
    //         contract_addr: self.addr().into(),
    //         msg: to_binary(&msg)?,
    //     }
    //     .into();
    //     let res: CountResponse = QuerierWrapper::<CQ>::new(querier).query(&query)?;
    //     Ok(res)
    // }
}

pub fn build_denom(creator: &Addr, subdenom: &str) -> Result<String, ContractError> {
    // Minimum validation checks on the full denom.
    // https://github.com/cosmos/cosmos-sdk/blob/2646b474c7beb0c93d4fafd395ef345f41afc251/types/coin.go#L706-L711
    // https://github.com/cosmos/cosmos-sdk/blob/2646b474c7beb0c93d4fafd395ef345f41afc251/types/coin.go#L677
    let full_denom = format!("factory/{}/{}", creator, subdenom);
    if full_denom.len() < 3
        || full_denom.len() > 128
        || creator.as_str().contains('/')
        || subdenom.len() > 44
        || creator.as_str().len() > 75
    {
        return Err(ContractError::InvalidDenom {
            denom: full_denom,
            message: "".to_string(),
        });
    }
    Ok(full_denom)
}

pub fn check_funds(denom: String, funds: &Vec<Coin>, amount: Uint128) -> Result<(), ContractError> {
    // TODO: Do we want to check for too much funds here? Otherwise all the excess funds will remain locked in the contract 4evers

    if let Some(osmo) = funds.iter().find(|c| c.denom == denom) {
        if osmo.amount < amount {
            Err(ContractError::NotEnoughFunds {
                denom,
                funds: osmo.amount.u128(),
                needed: amount.u128(),
            })
        } else {
            Ok(())
        }
    } else {
        Err(ContractError::NotEnoughFunds {
            denom,
            funds: 0u128,
            needed: amount.u128(),
        })
    }
}

pub fn set_bool_allowance(
    deps: DepsMut,
    address: &String,
    allowances: Map<&Addr, bool>,
    status: bool,
) -> Result<bool, ContractError> {
    return allowances.update(
        deps.storage,
        &deps.api.addr_validate(address.as_str())?,
        |mut stat| -> Result<_, ContractError> {
            if let Some(current_status) = stat {
                if current_status == status {
                    return Err(ContractError::FreezerStatusUnchanged { status });
                }
            }
            stat = Some(status);
            Ok(status)
        },
    );
}

pub fn check_is_contract_owner(deps: Deps, sender: Addr) -> Result<(), ContractError> {
    let config = CONFIG.load(deps.storage).unwrap();
    if config.owner != sender {
        Err(ContractError::Unauthorized {})
    } else {
        Ok(())
    }
}

pub fn set_int_allowance(
    deps: DepsMut,
    allowances: Map<&Addr, Uint128>,
    address: &String,
    amount: Uint128,
) -> Result<(), ContractError> {
    // TODO What if the allowance doesnt change, like i check for at bool_allowance
    let res = allowances.save(
        deps.storage,
        &deps.api.addr_validate(address.as_str())?,
        &amount,
    );
    match res {
        Ok(()) => Ok(()),
        Err(error) => Err(ContractError::Std(error)),
    }
}

pub fn check_bool_allowance(
    deps: &Deps,
    info: MessageInfo,
    allowances: Map<&Addr, bool>,
) -> Result<(), ContractError> {
    let res = allowances.load(deps.storage, &info.sender);
    match res {
        Ok(authorized) => {
            if !authorized {
                return Err(ContractError::Unauthorized {});
            }
        }
        Err(error) => {
            if let cosmwasm_std::StdError::NotFound { .. } = error {
                return Err(ContractError::Unauthorized {});
            } else {
                return Err(ContractError::Std(error));
            }
        }
    }
    Ok(())
}
