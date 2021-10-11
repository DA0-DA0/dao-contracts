use cosmwasm_std::{Binary, DepsMut, Env, MessageInfo, Response, StdResult, Uint128};
use cw20_base::ContractError;

use crate::state::{DELEGATIONS, VOTING_POWER};

pub fn execute_transfer_from(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let rcpt_addr = deps.api.addr_validate(&recipient)?;
    let owner_addr = deps.api.addr_validate(&owner)?;
    let owner_delegation = DELEGATIONS
        .may_load(deps.storage, &owner_addr)?
        .unwrap_or_else(|| owner_addr.clone());
    let recipient_delegation = DELEGATIONS
        .may_load(deps.storage, &rcpt_addr)?
        .unwrap_or_else(|| rcpt_addr.clone());
    VOTING_POWER.update(
        deps.storage,
        &owner_delegation,
        env.block.height,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    VOTING_POWER.update(
        deps.storage,
        &recipient_delegation,
        env.block.height,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;

    cw20_base::allowances::execute_transfer_from(deps, env, info, owner, recipient, amount)
}

pub fn execute_burn_from(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let owner_addr = deps.api.addr_validate(&owner)?;
    let owner_delegation = DELEGATIONS
        .may_load(deps.storage, &owner_addr)?
        .unwrap_or_else(|| owner_addr.clone());
    // lower balance
    VOTING_POWER.update(
        deps.storage,
        &owner_delegation,
        env.block.height,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    cw20_base::allowances::execute_burn_from(deps, env, info, owner, amount)
}

pub fn execute_send_from(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    contract: String,
    amount: Uint128,
    msg: Binary,
) -> Result<Response, ContractError> {
    let rcpt_addr = deps.api.addr_validate(&contract)?;
    let owner_addr = deps.api.addr_validate(&owner)?;
    let owner_delegation = DELEGATIONS
        .may_load(deps.storage, &owner_addr)?
        .unwrap_or_else(|| owner_addr.clone());
    let recipient_delegation = DELEGATIONS
        .may_load(deps.storage, &rcpt_addr)?
        .unwrap_or_else(|| rcpt_addr.clone());
    // move the tokens to the contract
    VOTING_POWER.update(
        deps.storage,
        &owner_delegation,
        env.block.height,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    VOTING_POWER.update(
        deps.storage,
        &recipient_delegation,
        env.block.height,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;
    cw20_base::allowances::execute_send_from(deps, env, info, owner, contract, amount, msg)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, CosmosMsg, Deps, StdError, SubMsg, WasmMsg};
    use cw20::{AllowanceResponse, Cw20Coin, Cw20ReceiveMsg, Expiration, TokenInfoResponse};
    use cw20_base::allowances::query_allowance;
    use cw20_base::contract::{query_balance, query_token_info};
    use cw20_base::msg::InstantiateMsg;
    use cw20_base::ContractError;

    use crate::contract::{execute, instantiate, query_delegation, query_voting_power_at_height};
    use crate::msg::ExecuteMsg;

    use super::*;

    fn get_balance<T: Into<String>>(deps: Deps, address: T) -> Uint128 {
        query_balance(deps, address.into()).unwrap().balance
    }

    // this will set up the instantiation for other tests
    fn do_instantiate<T: Into<String>>(
        mut deps: DepsMut,
        addr: T,
        amount: Uint128,
    ) -> TokenInfoResponse {
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
    fn transfer_from_respects_limits() {
        let mut deps = mock_dependencies(&[]);
        let owner = String::from("addr0001");
        let spender = String::from("addr0002");
        let rcpt = String::from("addr0003");

        let start = Uint128::new(999999);
        do_instantiate(deps.as_mut(), &owner, start);

        // provide an allowance
        let allow1 = Uint128::new(77777);
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender.clone(),
            amount: allow1,
            expires: None,
        };
        let info = mock_info(owner.as_ref(), &[]);
        let env = mock_env();
        execute(deps.as_mut(), env, info, msg).unwrap();

        // valid transfer of part of the allowance
        let transfer = Uint128::new(44444);
        let msg = ExecuteMsg::TransferFrom {
            owner: owner.clone(),
            recipient: rcpt.clone(),
            amount: transfer,
        };
        let info = mock_info(spender.as_ref(), &[]);
        let env = mock_env();
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(res.attributes[0], attr("action", "transfer_from"));

        // make sure money arrived
        assert_eq!(
            get_balance(deps.as_ref(), owner.clone()),
            start.checked_sub(transfer).unwrap()
        );
        assert_eq!(get_balance(deps.as_ref(), rcpt.clone()), transfer);

        // ensure it looks good
        let allowance = query_allowance(deps.as_ref(), owner.clone(), spender.clone()).unwrap();
        let expect = AllowanceResponse {
            allowance: allow1.checked_sub(transfer).unwrap(),
            expires: Expiration::Never {},
        };
        assert_eq!(expect, allowance);

        // cannot send more than the allowance
        let msg = ExecuteMsg::TransferFrom {
            owner: owner.clone(),
            recipient: rcpt.clone(),
            amount: Uint128::new(33443),
        };
        let info = mock_info(spender.as_ref(), &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));

        // let us increase limit, but set the expiration (default env height is 12_345)
        let info = mock_info(owner.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender.clone(),
            amount: Uint128::new(1000),
            expires: Some(Expiration::AtHeight(env.block.height)),
        };
        execute(deps.as_mut(), env, info, msg).unwrap();

        // we should now get the expiration error
        let msg = ExecuteMsg::TransferFrom {
            owner,
            recipient: rcpt,
            amount: Uint128::new(33443),
        };
        let info = mock_info(spender.as_ref(), &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::Expired {});
    }

    #[test]
    fn burn_from_respects_limits() {
        let mut deps = mock_dependencies(&[]);
        let owner = String::from("addr0001");
        let spender = String::from("addr0002");

        let start = Uint128::new(999999);
        do_instantiate(deps.as_mut(), &owner, start);

        // provide an allowance
        let allow1 = Uint128::new(77777);
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender.clone(),
            amount: allow1,
            expires: None,
        };
        let info = mock_info(owner.as_ref(), &[]);
        let env = mock_env();
        execute(deps.as_mut(), env, info, msg).unwrap();

        // valid burn of part of the allowance
        let transfer = Uint128::new(44444);
        let msg = ExecuteMsg::BurnFrom {
            owner: owner.clone(),
            amount: transfer,
        };
        let info = mock_info(spender.as_ref(), &[]);
        let env = mock_env();
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(res.attributes[0], attr("action", "burn_from"));

        // make sure money burnt
        assert_eq!(
            get_balance(deps.as_ref(), owner.clone()),
            start.checked_sub(transfer).unwrap()
        );

        // ensure it looks good
        let allowance = query_allowance(deps.as_ref(), owner.clone(), spender.clone()).unwrap();
        let expect = AllowanceResponse {
            allowance: allow1.checked_sub(transfer).unwrap(),
            expires: Expiration::Never {},
        };
        assert_eq!(expect, allowance);

        // cannot burn more than the allowance
        let msg = ExecuteMsg::BurnFrom {
            owner: owner.clone(),
            amount: Uint128::new(33443),
        };
        let info = mock_info(spender.as_ref(), &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));

        // let us increase limit, but set the expiration (default env height is 12_345)
        let info = mock_info(owner.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender.clone(),
            amount: Uint128::new(1000),
            expires: Some(Expiration::AtHeight(env.block.height)),
        };
        execute(deps.as_mut(), env, info, msg).unwrap();

        // we should now get the expiration error
        let msg = ExecuteMsg::BurnFrom {
            owner,
            amount: Uint128::new(33443),
        };
        let info = mock_info(spender.as_ref(), &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::Expired {});
    }

    #[test]
    fn send_from_respects_limits() {
        let mut deps = mock_dependencies(&[]);
        let owner = String::from("addr0001");
        let spender = String::from("addr0002");
        let contract = String::from("cool-dex");
        let send_msg = Binary::from(r#"{"some":123}"#.as_bytes());

        let start = Uint128::new(999999);
        do_instantiate(deps.as_mut(), &owner, start);

        // provide an allowance
        let allow1 = Uint128::new(77777);
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender.clone(),
            amount: allow1,
            expires: None,
        };
        let info = mock_info(owner.as_ref(), &[]);
        let env = mock_env();
        execute(deps.as_mut(), env, info, msg).unwrap();

        // valid send of part of the allowance
        let transfer = Uint128::new(44444);
        let msg = ExecuteMsg::SendFrom {
            owner: owner.clone(),
            amount: transfer,
            contract: contract.clone(),
            msg: send_msg.clone(),
        };
        let info = mock_info(spender.as_ref(), &[]);
        let env = mock_env();
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(res.attributes[0], attr("action", "send_from"));
        assert_eq!(1, res.messages.len());

        // we record this as sent by the one who requested, not the one who was paying
        let binary_msg = Cw20ReceiveMsg {
            sender: spender.clone(),
            amount: transfer,
            msg: send_msg.clone(),
        }
        .into_binary()
        .unwrap();
        assert_eq!(
            res.messages[0],
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract.clone(),
                msg: binary_msg,
                funds: vec![],
            }))
        );

        // make sure money sent
        assert_eq!(
            get_balance(deps.as_ref(), owner.clone()),
            start.checked_sub(transfer).unwrap()
        );
        assert_eq!(get_balance(deps.as_ref(), contract.clone()), transfer);

        // ensure it looks good
        let allowance = query_allowance(deps.as_ref(), owner.clone(), spender.clone()).unwrap();
        let expect = AllowanceResponse {
            allowance: allow1.checked_sub(transfer).unwrap(),
            expires: Expiration::Never {},
        };
        assert_eq!(expect, allowance);

        // cannot send more than the allowance
        let msg = ExecuteMsg::SendFrom {
            owner: owner.clone(),
            amount: Uint128::new(33443),
            contract: contract.clone(),
            msg: send_msg.clone(),
        };
        let info = mock_info(spender.as_ref(), &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));

        // let us increase limit, but set the expiration to current block (expired)
        let info = mock_info(owner.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender.clone(),
            amount: Uint128::new(1000),
            expires: Some(Expiration::AtHeight(env.block.height)),
        };
        execute(deps.as_mut(), env, info, msg).unwrap();

        // we should now get the expiration error
        let msg = ExecuteMsg::SendFrom {
            owner,
            amount: Uint128::new(33443),
            contract,
            msg: send_msg,
        };
        let info = mock_info(spender.as_ref(), &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::Expired {});
    }

    #[test]
    fn delegate_and_transfer_from() {
        let mut deps = mock_dependencies(&[]);
        let owner = String::from("addr0001");
        let spender = String::from("addr0002");
        let rcpt = String::from("addr0003");
        let delegatee_1 = String::from("addr0004");
        let delegatee_2 = String::from("addr0005");

        let start = Uint128::new(999999);
        do_instantiate(deps.as_mut(), &owner, start);

        // delegate from owner to delegatee1
        let info = mock_info(owner.as_ref(), &[]);
        let mut env = mock_env();
        let msg = ExecuteMsg::DelegateVotes {
            recipient: delegatee_1.clone(),
        };
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        env.block.height += 1;
        assert_eq!(
            query_delegation(deps.as_ref(), owner.clone())
                .unwrap()
                .delegation,
            delegatee_1
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), owner.clone(), env.block.height)
                .unwrap()
                .balance,
            Uint128::zero()
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), delegatee_1.clone(), env.block.height)
                .unwrap()
                .balance,
            start
        );

        // delegate from recpt to delegatee2
        let info = mock_info(rcpt.as_ref(), &[]);
        let msg = ExecuteMsg::DelegateVotes {
            recipient: delegatee_2.clone(),
        };
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        env.block.height += 1;
        assert_eq!(
            query_delegation(deps.as_ref(), owner.clone())
                .unwrap()
                .delegation,
            delegatee_1
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), rcpt.clone(), env.block.height)
                .unwrap()
                .balance,
            Uint128::zero()
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), rcpt.clone(), env.block.height)
                .unwrap()
                .balance,
            Uint128::zero()
        );

        // provide an allowance
        let allow1 = Uint128::new(77777);
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender.clone(),
            amount: allow1,
            expires: None,
        };
        let info = mock_info(owner.as_ref(), &[]);
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // valid transfer of part of the allowance
        let transfer = Uint128::new(44444);
        let msg = ExecuteMsg::TransferFrom {
            owner: owner.clone(),
            recipient: rcpt.clone(),
            amount: transfer,
        };
        let info = mock_info(spender.as_ref(), &[]);
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        env.block.height += 1;
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), owner.clone(), env.block.height)
                .unwrap()
                .balance,
            Uint128::zero()
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), delegatee_1.clone(), env.block.height)
                .unwrap()
                .balance,
            start.checked_sub(transfer).unwrap()
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), delegatee_2.clone(), env.block.height)
                .unwrap()
                .balance,
            transfer
        );
    }

    #[test]
    fn delegate_and_burn_from() {
        let mut deps = mock_dependencies(&[]);
        let owner = String::from("addr0001");
        let spender = String::from("addr0002");
        let delegatee = String::from("addr0003");

        let start = Uint128::new(999999);
        do_instantiate(deps.as_mut(), &owner, start);

        // delegate from owner to delegatee1
        let info = mock_info(owner.as_ref(), &[]);
        let mut env = mock_env();
        let msg = ExecuteMsg::DelegateVotes {
            recipient: delegatee.clone(),
        };
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        env.block.height += 1;
        assert_eq!(
            query_delegation(deps.as_ref(), owner.clone())
                .unwrap()
                .delegation,
            delegatee
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), owner.clone(), env.block.height)
                .unwrap()
                .balance,
            Uint128::zero()
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), delegatee.clone(), env.block.height)
                .unwrap()
                .balance,
            start
        );

        // provide an allowance
        let allow1 = Uint128::new(77777);
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender.clone(),
            amount: allow1,
            expires: None,
        };
        let info = mock_info(owner.as_ref(), &[]);
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // valid burn of part of the allowance
        let burn = Uint128::new(44444);
        let msg = ExecuteMsg::BurnFrom {
            owner: owner.clone(),
            amount: burn,
        };
        let info = mock_info(spender.as_ref(), &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        env.block.height += 1;
        assert_eq!(res.attributes[0], attr("action", "burn_from"));
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), owner.clone(), env.block.height)
                .unwrap()
                .balance,
            Uint128::zero()
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), delegatee.clone(), env.block.height)
                .unwrap()
                .balance,
            start - burn
        );
    }

    #[test]
    fn delegate_and_send_from() {
        let mut deps = mock_dependencies(&[]);
        let owner = String::from("addr0001");
        let spender = String::from("addr0002");
        let rcpt = String::from("addr0003");
        let delegatee_1 = String::from("addr0004");
        let delegatee_2 = String::from("addr0005");

        let start = Uint128::new(999999);
        do_instantiate(deps.as_mut(), &owner, start);

        // delegate from owner to delegatee1
        let info = mock_info(owner.as_ref(), &[]);
        let mut env = mock_env();
        let msg = ExecuteMsg::DelegateVotes {
            recipient: delegatee_1.clone(),
        };
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        env.block.height += 1;
        assert_eq!(
            query_delegation(deps.as_ref(), owner.clone())
                .unwrap()
                .delegation,
            delegatee_1
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), owner.clone(), env.block.height)
                .unwrap()
                .balance,
            Uint128::zero()
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), delegatee_1.clone(), env.block.height)
                .unwrap()
                .balance,
            start
        );

        // delegate from recpt to delegatee2
        let info = mock_info(rcpt.as_ref(), &[]);
        let msg = ExecuteMsg::DelegateVotes {
            recipient: delegatee_2.clone(),
        };
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        env.block.height += 1;
        assert_eq!(
            query_delegation(deps.as_ref(), owner.clone())
                .unwrap()
                .delegation,
            delegatee_1
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), rcpt.clone(), env.block.height)
                .unwrap()
                .balance,
            Uint128::zero()
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), rcpt.clone(), env.block.height)
                .unwrap()
                .balance,
            Uint128::zero()
        );

        // provide an allowance
        let allow1 = Uint128::new(77777);
        let msg = ExecuteMsg::IncreaseAllowance {
            spender: spender.clone(),
            amount: allow1,
            expires: None,
        };
        let info = mock_info(owner.as_ref(), &[]);
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // valid transfer of part of the allowance
        let transfer = Uint128::new(44444);
        let send_msg = Binary::from(r#"{"some":123}"#.as_bytes());
        let msg = ExecuteMsg::SendFrom {
            owner: owner.clone(),
            contract: rcpt.clone(),
            amount: transfer,
            msg: send_msg,
        };
        let info = mock_info(spender.as_ref(), &[]);
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        env.block.height += 1;
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), owner.clone(), env.block.height)
                .unwrap()
                .balance,
            Uint128::zero()
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), delegatee_1.clone(), env.block.height)
                .unwrap()
                .balance,
            start.checked_sub(transfer).unwrap()
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), delegatee_2.clone(), env.block.height)
                .unwrap()
                .balance,
            transfer
        );
    }
}
