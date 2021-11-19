#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Storage, Uint128,
};
use cw20::Cw20Coin;
use cw20_base::allowances::{
    execute_decrease_allowance, execute_increase_allowance, query_allowance,
};
use cw20_base::contract::{
    execute_update_marketing, execute_upload_logo, query_balance, query_download_logo,
    query_marketing_info, query_minter, query_token_info,
};
use cw20_base::enumerable::{query_all_accounts, query_all_allowances};
use cw20_base::msg::InstantiateMsg;
use cw20_base::ContractError;

use crate::allowances::{execute_burn_from, execute_send_from, execute_transfer_from};

use crate::msg::{DelegationResponse, ExecuteMsg, QueryMsg, VotingPowerAtHeightResponse};
use crate::state::{DELEGATIONS, VOTING_POWER};
use cw20_base::state::BALANCES;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // create initial accounts
    create_voting_power(&mut deps, &msg.initial_balances)?;
    cw20_base::contract::instantiate(deps, _env, _info, msg)
}

pub fn create_voting_power(deps: &mut DepsMut, accounts: &[Cw20Coin]) -> StdResult<Uint128> {
    let mut total_supply = Uint128::zero();
    for row in accounts {
        let address = deps.api.addr_validate(&row.address)?;
        VOTING_POWER.save(deps.storage, &address, &row.amount, 0)?;
        total_supply += row.amount;
    }
    Ok(total_supply)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Transfer { recipient, amount } => {
            execute_transfer(deps, env, info, recipient, amount)
        }
        ExecuteMsg::Burn { amount } => execute_burn(deps, env, info, amount),
        ExecuteMsg::Send {
            contract,
            amount,
            msg,
        } => execute_send(deps, env, info, contract, amount, msg),
        ExecuteMsg::Mint { recipient, amount } => execute_mint(deps, env, info, recipient, amount),
        ExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => execute_increase_allowance(deps, env, info, spender, amount, expires),
        ExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => execute_decrease_allowance(deps, env, info, spender, amount, expires),
        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => execute_transfer_from(deps, env, info, owner, recipient, amount),
        ExecuteMsg::BurnFrom { owner, amount } => execute_burn_from(deps, env, info, owner, amount),
        ExecuteMsg::SendFrom {
            owner,
            contract,
            amount,
            msg,
        } => execute_send_from(deps, env, info, owner, contract, amount, msg),
        ExecuteMsg::UpdateMarketing {
            project,
            description,
            marketing,
        } => execute_update_marketing(deps, env, info, project, description, marketing),
        ExecuteMsg::UploadLogo(logo) => execute_upload_logo(deps, env, info, logo),
        ExecuteMsg::DelegateVotes { recipient } => {
            execute_delegate_votes(deps, env, info, recipient)
        }
    }
}

pub fn execute_transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let rcpt_addr = deps.api.addr_validate(&recipient)?;
    transfer_voting_power(deps.storage, &env, &info.sender, &rcpt_addr, amount)?;
    cw20_base::contract::execute_transfer(deps, env, info, recipient, amount)
}

pub fn transfer_voting_power(
    storage: &mut dyn Storage,
    env: &Env,
    sender: &Addr,
    recipient: &Addr,
    amount: Uint128,
) -> Result<(), ContractError> {
    let sender_delegation = DELEGATIONS
        .may_load(storage, &sender)?
        .unwrap_or_else(|| sender.clone());
    let recipient_delegation = DELEGATIONS
        .may_load(storage, &recipient)?
        .unwrap_or_else(|| recipient.clone());
    VOTING_POWER.update(
        storage,
        &sender_delegation,
        env.block.height,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    VOTING_POWER.update(
        storage,
        &recipient_delegation,
        env.block.height,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_add(amount)?)
        },
    )?;
    Ok(())
}

pub fn execute_burn(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let sender_delegation = DELEGATIONS
        .may_load(deps.storage, &info.sender)?
        .unwrap_or_else(|| info.sender.clone());
    VOTING_POWER.update(
        deps.storage,
        &sender_delegation,
        env.block.height,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    cw20_base::contract::execute_burn(deps, env, info, amount)
}

pub fn execute_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let rcpt_addr = deps.api.addr_validate(&recipient)?;
    let recipient_delegation = DELEGATIONS
        .may_load(deps.storage, &rcpt_addr)?
        .unwrap_or_else(|| rcpt_addr.clone());
    VOTING_POWER.update(
        deps.storage,
        &recipient_delegation,
        env.block.height,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;

    cw20_base::contract::execute_mint(deps, env, info, recipient, amount)
}

pub fn execute_send(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    contract: String,
    amount: Uint128,
    msg: Binary,
) -> Result<Response, ContractError> {
    let contract_addr = deps.api.addr_validate(&contract)?;
    transfer_voting_power(deps.storage, &env, &info.sender, &contract_addr, amount)?;
    cw20_base::contract::execute_send(deps, env, info, contract, amount, msg)
}

pub fn execute_delegate_votes(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
) -> Result<Response, ContractError> {
    let rcpt_addr = deps.api.addr_validate(&recipient)?;
    let amount = BALANCES
        .may_load(deps.storage, &info.sender)?
        .unwrap_or_default();
    let old_delegation = DELEGATIONS
        .may_load(deps.storage, &info.sender)?
        .unwrap_or_else(|| info.sender.clone());
    DELEGATIONS.update(deps.storage, &info.sender, |_| -> StdResult<_> {
        Ok(rcpt_addr.clone())
    })?;
    VOTING_POWER.update(
        deps.storage,
        &old_delegation,
        env.block.height,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    VOTING_POWER.update(
        deps.storage,
        &rcpt_addr,
        env.block.height,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // Custom queries
        QueryMsg::VotingPowerAtHeight { address, height } => {
            to_binary(&query_voting_power_at_height(deps, address, height)?)
        }
        // Inherited from cw20_base
        QueryMsg::Balance { address } => to_binary(&query_balance(deps, address)?),
        QueryMsg::Delegation { address } => to_binary(&query_delegation(deps, address)?),
        QueryMsg::TokenInfo {} => to_binary(&query_token_info(deps)?),
        QueryMsg::Minter {} => to_binary(&query_minter(deps)?),
        QueryMsg::Allowance { owner, spender } => {
            to_binary(&query_allowance(deps, owner, spender)?)
        }
        QueryMsg::AllAllowances {
            owner,
            start_after,
            limit,
        } => to_binary(&query_all_allowances(deps, owner, start_after, limit)?),
        QueryMsg::AllAccounts { start_after, limit } => {
            to_binary(&query_all_accounts(deps, start_after, limit)?)
        }
        QueryMsg::MarketingInfo {} => to_binary(&query_marketing_info(deps)?),
        QueryMsg::DownloadLogo {} => to_binary(&query_download_logo(deps)?),
    }
}

pub fn query_voting_power_at_height(
    deps: Deps,
    address: String,
    height: u64,
) -> StdResult<VotingPowerAtHeightResponse> {
    let address = deps.api.addr_validate(&address)?;
    let balance = VOTING_POWER
        .may_load_at_height(deps.storage, &address, height)?
        .unwrap_or_default();
    Ok(VotingPowerAtHeightResponse { balance, height })
}

pub fn query_delegation(deps: Deps, address: String) -> StdResult<DelegationResponse> {
    let address_addr = deps.api.addr_validate(&address)?;
    let delegation = DELEGATIONS
        .may_load(deps.storage, &address_addr)?
        .unwrap_or(address_addr);
    Ok(DelegationResponse {
        delegation: delegation.into(),
    })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_binary, CosmosMsg, StdError, SubMsg, WasmMsg};
    use cw20::{BalanceResponse, Cw20ReceiveMsg, MinterResponse, TokenInfoResponse};

    use super::*;

    fn get_balance<T: Into<String>>(deps: Deps, address: T) -> Uint128 {
        query_balance(deps, address.into()).unwrap().balance
    }

    // this will set up the instantiation for other tests
    fn do_instantiate_with_minter(
        deps: DepsMut,
        addr: &str,
        amount: Uint128,
        minter: &str,
        cap: Option<Uint128>,
    ) -> TokenInfoResponse {
        _do_instantiate(
            deps,
            addr,
            amount,
            Some(MinterResponse {
                minter: minter.to_string(),
                cap,
            }),
        )
    }

    // this will set up the instantiation for other tests
    fn do_instantiate(deps: DepsMut, addr: &str, amount: Uint128) -> TokenInfoResponse {
        _do_instantiate(deps, addr, amount, None)
    }

    // this will set up the instantiation for other tests
    fn _do_instantiate(
        mut deps: DepsMut,
        addr: &str,
        amount: Uint128,
        mint: Option<MinterResponse>,
    ) -> TokenInfoResponse {
        let instantiate_msg = InstantiateMsg {
            name: "Auto Gen".to_string(),
            symbol: "AUTO".to_string(),
            decimals: 3,
            initial_balances: vec![Cw20Coin {
                address: addr.to_string(),
                amount,
            }],
            mint: mint.clone(),
            marketing: None,
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();
        let res = instantiate(deps.branch(), env, info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let meta = query_token_info(deps.as_ref()).unwrap();
        assert_eq!(
            meta,
            TokenInfoResponse {
                name: "Auto Gen".to_string(),
                symbol: "AUTO".to_string(),
                decimals: 3,
                total_supply: amount,
            }
        );
        assert_eq!(get_balance(deps.as_ref(), addr), amount);
        assert_eq!(query_minter(deps.as_ref()).unwrap(), mint,);
        meta
    }

    mod instantiate {
        use super::*;

        #[test]
        fn basic() {
            let mut deps = mock_dependencies();
            let amount = Uint128::from(11223344u128);
            let instantiate_msg = InstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: vec![Cw20Coin {
                    address: String::from("addr0000"),
                    amount,
                }],
                mint: None,
                marketing: None,
            };
            let info = mock_info("creator", &[]);
            let env = mock_env();
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());

            assert_eq!(
                query_token_info(deps.as_ref()).unwrap(),
                TokenInfoResponse {
                    name: "Cash Token".to_string(),
                    symbol: "CASH".to_string(),
                    decimals: 9,
                    total_supply: amount,
                }
            );
            assert_eq!(
                get_balance(deps.as_ref(), "addr0000"),
                Uint128::new(11223344)
            );
        }

        #[test]
        fn mintable() {
            let mut deps = mock_dependencies();
            let amount = Uint128::new(11223344);
            let minter = String::from("asmodat");
            let limit = Uint128::new(511223344);
            let instantiate_msg = InstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: vec![Cw20Coin {
                    address: "addr0000".into(),
                    amount,
                }],
                mint: Some(MinterResponse {
                    minter: minter.clone(),
                    cap: Some(limit),
                }),
                marketing: None,
            };
            let info = mock_info("creator", &[]);
            let env = mock_env();
            let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
            assert_eq!(0, res.messages.len());

            assert_eq!(
                query_token_info(deps.as_ref()).unwrap(),
                TokenInfoResponse {
                    name: "Cash Token".to_string(),
                    symbol: "CASH".to_string(),
                    decimals: 9,
                    total_supply: amount,
                }
            );
            assert_eq!(
                get_balance(deps.as_ref(), "addr0000"),
                Uint128::new(11223344)
            );
            assert_eq!(
                query_minter(deps.as_ref()).unwrap(),
                Some(MinterResponse {
                    minter,
                    cap: Some(limit),
                }),
            );
        }

        #[test]
        fn mintable_over_cap() {
            let mut deps = mock_dependencies();
            let amount = Uint128::new(11223344);
            let minter = String::from("asmodat");
            let limit = Uint128::new(11223300);
            let instantiate_msg = InstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: vec![Cw20Coin {
                    address: String::from("addr0000"),
                    amount,
                }],
                mint: Some(MinterResponse {
                    minter,
                    cap: Some(limit),
                }),
                marketing: None,
            };
            let info = mock_info("creator", &[]);
            let env = mock_env();
            let err = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap_err();
            assert_eq!(
                err,
                StdError::generic_err("Initial supply greater than cap").into()
            );
        }
    }

    #[test]
    fn can_mint_by_minter() {
        let mut deps = mock_dependencies();

        let genesis = String::from("genesis");
        let amount = Uint128::new(11223344);
        let minter = String::from("asmodat");
        let limit = Uint128::new(511223344);
        do_instantiate_with_minter(deps.as_mut(), &genesis, amount, &minter, Some(limit));

        // minter can mint coins to some winner
        let winner = String::from("lucky");
        let prize = Uint128::new(222_222_222);
        let msg = ExecuteMsg::Mint {
            recipient: winner.clone(),
            amount: prize,
        };

        let info = mock_info(minter.as_ref(), &[]);
        let env = mock_env();
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(get_balance(deps.as_ref(), genesis), amount);
        assert_eq!(get_balance(deps.as_ref(), winner.clone()), prize);

        // but cannot mint nothing
        let msg = ExecuteMsg::Mint {
            recipient: winner.clone(),
            amount: Uint128::zero(),
        };
        let info = mock_info(minter.as_ref(), &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::InvalidZeroAmount {});

        // but if it exceeds cap (even over multiple rounds), it fails
        // cap is enforced
        let msg = ExecuteMsg::Mint {
            recipient: winner,
            amount: Uint128::new(333_222_222),
        };
        let info = mock_info(minter.as_ref(), &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::CannotExceedCap {});
    }

    #[test]
    fn others_cannot_mint() {
        let mut deps = mock_dependencies();
        do_instantiate_with_minter(
            deps.as_mut(),
            &String::from("genesis"),
            Uint128::new(1234),
            &String::from("minter"),
            None,
        );

        let msg = ExecuteMsg::Mint {
            recipient: String::from("lucky"),
            amount: Uint128::new(222),
        };
        let info = mock_info("anyone else", &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }

    #[test]
    fn no_one_mints_if_minter_unset() {
        let mut deps = mock_dependencies();
        do_instantiate(deps.as_mut(), &String::from("genesis"), Uint128::new(1234));

        let msg = ExecuteMsg::Mint {
            recipient: String::from("lucky"),
            amount: Uint128::new(222),
        };
        let info = mock_info("genesis", &[]);
        let env = mock_env();
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }

    #[test]
    fn instantiate_multiple_accounts() {
        let mut deps = mock_dependencies();
        let amount1 = Uint128::from(11223344u128);
        let addr1 = String::from("addr0001");
        let amount2 = Uint128::from(7890987u128);
        let addr2 = String::from("addr0002");
        let instantiate_msg = InstantiateMsg {
            name: "Bash Shell".to_string(),
            symbol: "BASH".to_string(),
            decimals: 6,
            initial_balances: vec![
                Cw20Coin {
                    address: addr1.clone(),
                    amount: amount1,
                },
                Cw20Coin {
                    address: addr2.clone(),
                    amount: amount2,
                },
            ],
            mint: None,
            marketing: None,
        };
        let info = mock_info("creator", &[]);
        let env = mock_env();
        let res = instantiate(deps.as_mut(), env, info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        assert_eq!(
            query_token_info(deps.as_ref()).unwrap(),
            TokenInfoResponse {
                name: "Bash Shell".to_string(),
                symbol: "BASH".to_string(),
                decimals: 6,
                total_supply: amount1 + amount2,
            }
        );
        assert_eq!(get_balance(deps.as_ref(), addr1), amount1);
        assert_eq!(get_balance(deps.as_ref(), addr2), amount2);
    }

    #[test]
    fn queries_work() {
        let mut deps = mock_dependencies();
        let addr1 = String::from("addr0001");
        let amount1 = Uint128::from(12340000u128);

        let expected = do_instantiate(deps.as_mut(), &addr1, amount1);

        // check meta query
        let loaded = query_token_info(deps.as_ref()).unwrap();
        assert_eq!(expected, loaded);

        let _info = mock_info("test", &[]);
        let env = mock_env();
        // check balance query (full)
        let data = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Balance { address: addr1 },
        )
        .unwrap();
        let loaded: BalanceResponse = from_binary(&data).unwrap();
        assert_eq!(loaded.balance, amount1);

        // check balance query (empty)
        let data = query(
            deps.as_ref(),
            env,
            QueryMsg::Balance {
                address: String::from("addr0002"),
            },
        )
        .unwrap();
        let loaded: BalanceResponse = from_binary(&data).unwrap();
        assert_eq!(loaded.balance, Uint128::zero());
    }

    #[test]
    fn get_voting_power_at_height() {
        let mut deps = mock_dependencies();
        let addr1 = String::from("addr0001");
        let addr2 = String::from("addr0002");
        let amount1 = Uint128::from(12340000u128);
        let transfer = Uint128::from(76543u128);

        let mut env = mock_env();
        let start_height = env.block.height;

        do_instantiate(deps.as_mut(), &addr1, amount1);

        env.block.height = env.block.height + 1;

        // valid transfer
        let info = mock_info(addr1.as_ref(), &[]);
        let msg = ExecuteMsg::Transfer {
            recipient: addr2.clone(),
            amount: transfer,
        };
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        env.block.height = env.block.height + 1;

        let remainder = amount1.checked_sub(transfer).unwrap();
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), addr1.clone().into(), start_height)
                .unwrap()
                .balance,
            amount1
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), addr1.clone().into(), env.block.height)
                .unwrap()
                .balance,
            remainder
        );

        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), addr2.clone().into(), start_height)
                .unwrap()
                .balance,
            Uint128::zero()
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), addr2.clone().into(), env.block.height)
                .unwrap()
                .balance,
            transfer
        );
    }

    #[test]
    fn transfer() {
        let mut deps = mock_dependencies();
        let addr1 = String::from("addr0001");
        let addr2 = String::from("addr0002");
        let amount1 = Uint128::from(12340000u128);
        let transfer = Uint128::from(76543u128);
        let too_much = Uint128::from(12340321u128);

        do_instantiate(deps.as_mut(), &addr1, amount1);

        // cannot transfer nothing
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Transfer {
            recipient: addr2.clone(),
            amount: Uint128::zero(),
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::InvalidZeroAmount {});

        // cannot send more than we have
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Transfer {
            recipient: addr2.clone(),
            amount: too_much,
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));

        // cannot send from empty account
        let info = mock_info(addr2.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Transfer {
            recipient: addr1.clone(),
            amount: transfer,
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));

        // valid transfer
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Transfer {
            recipient: addr2.clone(),
            amount: transfer,
        };
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        let remainder = amount1.checked_sub(transfer).unwrap();
        assert_eq!(get_balance(deps.as_ref(), addr1), remainder);
        assert_eq!(get_balance(deps.as_ref(), addr2), transfer);
        assert_eq!(
            query_token_info(deps.as_ref()).unwrap().total_supply,
            amount1
        );
    }

    #[test]
    fn burn() {
        let mut deps = mock_dependencies();
        let addr1 = String::from("addr0001");
        let amount1 = Uint128::from(12340000u128);
        let burn = Uint128::from(76543u128);
        let too_much = Uint128::from(12340321u128);

        do_instantiate(deps.as_mut(), &addr1, amount1);

        // cannot burn nothing
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Burn {
            amount: Uint128::zero(),
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::InvalidZeroAmount {});
        assert_eq!(
            query_token_info(deps.as_ref()).unwrap().total_supply,
            amount1
        );

        // cannot burn more than we have
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Burn { amount: too_much };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));
        assert_eq!(
            query_token_info(deps.as_ref()).unwrap().total_supply,
            amount1
        );

        // valid burn reduces total supply
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Burn { amount: burn };
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);

        let remainder = amount1.checked_sub(burn).unwrap();
        assert_eq!(get_balance(deps.as_ref(), addr1), remainder);
        assert_eq!(
            query_token_info(deps.as_ref()).unwrap().total_supply,
            remainder
        );
    }

    #[test]
    fn send() {
        let mut deps = mock_dependencies();
        let addr1 = String::from("addr0001");
        let contract = String::from("addr0002");
        let amount1 = Uint128::from(12340000u128);
        let transfer = Uint128::from(76543u128);
        let too_much = Uint128::from(12340321u128);
        let send_msg = Binary::from(r#"{"some":123}"#.as_bytes());

        do_instantiate(deps.as_mut(), &addr1, amount1);

        // cannot send nothing
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Send {
            contract: contract.clone(),
            amount: Uint128::zero(),
            msg: send_msg.clone(),
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::InvalidZeroAmount {});

        // cannot send more than we have
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Send {
            contract: contract.clone(),
            amount: too_much,
            msg: send_msg.clone(),
        };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::Overflow { .. })));

        // valid transfer
        let info = mock_info(addr1.as_ref(), &[]);
        let env = mock_env();
        let msg = ExecuteMsg::Send {
            contract: contract.clone(),
            amount: transfer,
            msg: send_msg.clone(),
        };
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(res.messages.len(), 1);

        // ensure proper send message sent
        // this is the message we want delivered to the other side
        let binary_msg = Cw20ReceiveMsg {
            sender: addr1.clone(),
            amount: transfer,
            msg: send_msg,
        }
        .into_binary()
        .unwrap();
        // and this is how it must be wrapped for the vm to process it
        assert_eq!(
            res.messages[0],
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract.clone(),
                msg: binary_msg,
                funds: vec![],
            }))
        );

        // ensure balance is properly transferred
        let remainder = amount1.checked_sub(transfer).unwrap();
        assert_eq!(get_balance(deps.as_ref(), addr1), remainder);
        assert_eq!(get_balance(deps.as_ref(), contract), transfer);
        assert_eq!(
            query_token_info(deps.as_ref()).unwrap().total_supply,
            amount1
        );
    }

    #[test]
    fn delegate_and_transfer() {
        let mut deps = mock_dependencies();
        let addr1 = String::from("addr0001");
        let addr2 = String::from("addr0002");
        let addr3 = String::from("addr0003");
        let amount1 = Uint128::from(12340000u128);
        let transfer = Uint128::from(76543u128);

        do_instantiate(deps.as_mut(), &addr1, amount1);

        // delegate from addr1 to addr2
        let info = mock_info(addr1.as_ref(), &[]);
        let mut env = mock_env();
        let msg = ExecuteMsg::DelegateVotes {
            recipient: addr2.clone(),
        };
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        env.block.height += 1;
        assert_eq!(
            query_delegation(deps.as_ref(), addr1.clone())
                .unwrap()
                .delegation,
            addr2
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), addr1.clone(), env.block.height)
                .unwrap()
                .balance,
            Uint128::zero()
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), addr2.clone(), env.block.height)
                .unwrap()
                .balance,
            amount1
        );

        // send tokens and assert delegation changes
        let info = mock_info(addr1.as_ref(), &[]);
        let msg = ExecuteMsg::Transfer {
            recipient: addr3.clone(),
            amount: transfer,
        };
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        env.block.height += 1;
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), addr1.clone(), env.block.height)
                .unwrap()
                .balance,
            Uint128::zero()
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), addr2.clone(), env.block.height)
                .unwrap()
                .balance,
            amount1.checked_sub(transfer).unwrap()
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), addr3.clone(), env.block.height)
                .unwrap()
                .balance,
            transfer
        );
    }

    #[test]
    fn delegate_and_send() {
        let mut deps = mock_dependencies();
        let addr1 = String::from("addr0001");
        let addr2 = String::from("addr0002");
        let contract = String::from("addr0003");
        let amount1 = Uint128::from(12340000u128);
        let transfer = Uint128::from(76543u128);
        let send_msg = Binary::from(r#"{"some":123}"#.as_bytes());

        do_instantiate(deps.as_mut(), &addr1, amount1);

        // delegate from addr1 to addr2
        let info = mock_info(addr1.as_ref(), &[]);
        let mut env = mock_env();
        let msg = ExecuteMsg::DelegateVotes {
            recipient: addr2.clone(),
        };
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        env.block.height += 1;
        assert_eq!(
            query_delegation(deps.as_ref(), addr1.clone())
                .unwrap()
                .delegation,
            addr2
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), addr1.clone(), env.block.height)
                .unwrap()
                .balance,
            Uint128::zero()
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), addr2.clone(), env.block.height)
                .unwrap()
                .balance,
            amount1
        );

        let info = mock_info(addr1.as_ref(), &[]);
        let msg = ExecuteMsg::Send {
            contract: contract.clone(),
            amount: transfer,
            msg: send_msg.clone(),
        };
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        env.block.height += 1;
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), addr1.clone(), env.block.height)
                .unwrap()
                .balance,
            Uint128::zero()
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), addr2.clone(), env.block.height)
                .unwrap()
                .balance,
            amount1.checked_sub(transfer).unwrap()
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), contract.clone(), env.block.height)
                .unwrap()
                .balance,
            transfer
        );
    }

    #[test]
    fn delegate_and_mint() {
        let _deps = mock_dependencies();

        let mut deps = mock_dependencies();
        let addr1 = String::from("addr0001");
        let addr2 = String::from("addr0002");
        let minter = String::from("addr0003");
        let genesis_amount = Uint128::from(12340000u128);
        let mint_amount = Uint128::from(76543u128);
        let _send_msg = Binary::from(r#"{"some":123}"#.as_bytes());
        do_instantiate_with_minter(deps.as_mut(), &addr1, genesis_amount, &minter, None);

        // delegate from addr1 to addr2
        let info = mock_info(addr1.as_ref(), &[]);
        let mut env = mock_env();
        let msg = ExecuteMsg::DelegateVotes {
            recipient: addr2.clone(),
        };
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        env.block.height += 1;
        assert_eq!(
            query_delegation(deps.as_ref(), addr1.clone())
                .unwrap()
                .delegation,
            addr2
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), addr1.clone(), env.block.height)
                .unwrap()
                .balance,
            Uint128::zero()
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), addr2.clone(), env.block.height)
                .unwrap()
                .balance,
            genesis_amount
        );

        // minted coins increase delegation
        let msg = ExecuteMsg::Mint {
            recipient: addr1.clone(),
            amount: mint_amount,
        };

        let info = mock_info(minter.as_ref(), &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        env.block.height += 1;
        assert_eq!(0, res.messages.len());
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), addr1.clone(), env.block.height)
                .unwrap()
                .balance,
            Uint128::zero()
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), addr2.clone(), env.block.height)
                .unwrap()
                .balance,
            genesis_amount + mint_amount
        );
    }

    #[test]
    fn delegate_and_burn() {
        let mut deps = mock_dependencies();
        let addr1 = String::from("addr0001");
        let addr2 = String::from("addr0002");
        let genesis_amount = Uint128::from(12340000u128);
        let burn_amount = Uint128::from(76543u128);

        do_instantiate(deps.as_mut(), &addr1, genesis_amount);

        // delegate from addr1 to addr2
        let info = mock_info(addr1.as_ref(), &[]);
        let mut env = mock_env();
        let msg = ExecuteMsg::DelegateVotes {
            recipient: addr2.clone(),
        };
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        env.block.height += 1;
        assert_eq!(
            query_delegation(deps.as_ref(), addr1.clone())
                .unwrap()
                .delegation,
            addr2
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), addr1.clone(), env.block.height)
                .unwrap()
                .balance,
            Uint128::zero()
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), addr2.clone(), env.block.height)
                .unwrap()
                .balance,
            genesis_amount
        );

        // valid burn reduces total supply
        let info = mock_info(addr1.as_ref(), &[]);
        let msg = ExecuteMsg::Burn {
            amount: burn_amount,
        };
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        env.block.height += 1;
        assert_eq!(res.messages.len(), 0);
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), addr1.clone(), env.block.height)
                .unwrap()
                .balance,
            Uint128::zero()
        );
        assert_eq!(
            query_voting_power_at_height(deps.as_ref(), addr2.clone(), env.block.height)
                .unwrap()
                .balance,
            genesis_amount - burn_amount
        );
    }
}
