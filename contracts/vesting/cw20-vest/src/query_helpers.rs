use cosmwasm_std::{Addr, Uint128, WasmQuery, to_binary, Deps, StdResult};
use cw20::{Cw20QueryMsg, BalanceResponse};
use cw_controllers::Claim;

pub fn query_balance(deps: Deps, token_address: &Addr, address: &Addr) -> StdResult<Uint128> {
    let query = WasmQuery::Smart {
        contract_addr: token_address.to_string(),
        msg: to_binary(&Cw20QueryMsg::Balance {
            address: address.to_string(),
        })?,
    };
    let res: BalanceResponse = deps.querier.query(&query.into())?;
    Ok(res.balance)
}

pub fn query_staking_claims(deps: Deps, staking_contract: &Addr, address: &Addr) -> StdResult<Vec<Claim>> {
    let query = WasmQuery::Smart {
        contract_addr: staking_contract.to_string(),
        msg: to_binary(&cw20_stake::msg::QueryMsg::Claims {
            address: address.to_string(),
        })?,
    };
    let res: cw20_stake::msg::ClaimsResponse = deps.querier.query(&query.into())?;
    Ok(res.claims)
}

pub fn query_staking_config(deps: Deps, staking_contract: &Addr) -> StdResult<cw20_stake::state::Config> {
    let query = WasmQuery::Smart {
        contract_addr: staking_contract.to_string(),
        msg: to_binary(&cw20_stake::msg::QueryMsg::GetConfig {})?,
    };
    let config: cw20_stake::state::Config = deps.querier.query(&query.into())?;
    Ok(config)
}