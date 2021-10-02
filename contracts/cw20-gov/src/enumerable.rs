use cosmwasm_std::{Deps, Order, StdResult};
use cw20::{AllAccountsResponse, AllAllowancesResponse, AllowanceInfo};
use cw20_base::state::{ALLOWANCES};

use crate::state::{BALANCES};
use cw_storage_plus::Bound;

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn query_all_accounts(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<AllAccountsResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let accounts: Result<Vec<_>, _> = BALANCES
        .range(deps.storage, start, None, Order::Ascending)
        .filter_map(|x| x.map(|x| x.0).ok())
        .map(String::from_utf8)
        .take(limit)
        .collect();

    Ok(AllAccountsResponse {
        accounts: accounts?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, DepsMut, Uint128};
    use cw20::{Cw20Coin, Expiration, TokenInfoResponse};
    use cw20_base::contract::{query_token_info};
    use cw20_base::msg::InstantiateMsg;

    use crate::contract::{execute, instantiate};
    use crate::msg::{ExecuteMsg};

    // this will set up the instantiation for other tests
    fn do_instantiate(mut deps: DepsMut, addr: &str, amount: Uint128) -> TokenInfoResponse {
        let instantiate_msg = InstantiateMsg {
            name: "Auto Gen".to_string(),
            symbol: "AUTO".to_string(),
            decimals: 3,
            initial_balances: vec![Cw20Coin {
                address: addr.into(),
                amount,
            }],
            mint: None,
            marketing: None,
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();
        instantiate(deps.branch(), env, info, instantiate_msg).unwrap();
        query_token_info(deps.as_ref()).unwrap()
    }


    #[test]
    fn query_all_accounts_works() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        // insert order and lexicographical order are different
        let acct1 = String::from("acct01");
        let acct2 = String::from("zebra");
        let acct3 = String::from("nice");
        let acct4 = String::from("aaaardvark");
        let expected_order = [acct4.clone(), acct1.clone(), acct3.clone(), acct2.clone()];

        do_instantiate(deps.as_mut(), &acct1, Uint128::new(12340000));

        // put money everywhere (to create balanaces)
        let info = mock_info(acct1.as_ref(), &[]);
        let env = mock_env();
        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            ExecuteMsg::Transfer {
                recipient: acct2,
                amount: Uint128::new(222222),
            },
        )
        .unwrap();
        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            ExecuteMsg::Transfer {
                recipient: acct3,
                amount: Uint128::new(333333),
            },
        )
        .unwrap();
        execute(
            deps.as_mut(),
            env,
            info,
            ExecuteMsg::Transfer {
                recipient: acct4,
                amount: Uint128::new(444444),
            },
        )
        .unwrap();

        // make sure we get the proper results
        let accounts = query_all_accounts(deps.as_ref(), None, None).unwrap();
        assert_eq!(accounts.accounts, expected_order);

        // let's do pagination
        let accounts = query_all_accounts(deps.as_ref(), None, Some(2)).unwrap();
        assert_eq!(accounts.accounts, expected_order[0..2].to_vec());

        let accounts =
            query_all_accounts(deps.as_ref(), Some(accounts.accounts[1].clone()), Some(1)).unwrap();
        assert_eq!(accounts.accounts, expected_order[2..3].to_vec());

        let accounts =
            query_all_accounts(deps.as_ref(), Some(accounts.accounts[0].clone()), Some(777))
                .unwrap();
        assert_eq!(accounts.accounts, expected_order[3..].to_vec());
    }
}
