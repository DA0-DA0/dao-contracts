use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, SubMsg,
    Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw20::Cw20Coin;
use cw20_staked_balance_voting::msg::{StakingInfo, TokenInfo};
use cw_utils::parse_reply_instantiate_data;
use oorandom::Rand32;
use stake_cw20::hooks::StakeChangedHookMsg;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{
    DAO_ADDRESS, HEIGHT_TO_TOTAL_POWER, STAKING_CONTRACT, STAKING_CONTRACT_CODE_ID,
    STAKING_CONTRACT_UNSTAKING_DURATION, TOKEN, VOTE_WEIGHTS,
};

const CONTRACT_NAME: &str = "crates.io:cw4-voting";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_TOKEN_REPLY_ID: u64 = 0;
const INSTANTIATE_STAKING_REPLY_ID: u64 = 1;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    match msg.token_info {
        TokenInfo::Existing {
            address,
            staking_contract,
        } => {
            let address = deps.api.addr_validate(&address)?;
            match staking_contract {
                StakingInfo::Existing {
                    staking_contract_address,
                } => {
                    let staking_contract_address =
                        deps.api.addr_validate(&staking_contract_address)?;
                    let resp: stake_cw20::msg::GetConfigResponse = deps.querier.query_wasm_smart(
                        &staking_contract_address,
                        &stake_cw20::msg::QueryMsg::GetConfig {},
                    )?;

                    if address != resp.token_address {
                        return Err(ContractError::StakingContractMismatch {});
                    }

                    STAKING_CONTRACT.save(deps.storage, &staking_contract_address)?;

                    Ok(Response::default()
                        .add_attribute("action", "instantiate")
                        .add_attribute("token", "existing_token")
                        .add_attribute("token_address", address)
                        .add_attribute("staking_contract", staking_contract_address))
                }
                StakingInfo::New {
                    staking_code_id,
                    unstaking_duration,
                } => {
                    let msg = WasmMsg::Instantiate {
                        code_id: staking_code_id,
                        funds: vec![],
                        admin: Some(info.sender.to_string()),
                        label: env.contract.address.to_string(),
                        msg: to_binary(&stake_cw20::msg::InstantiateMsg {
                            owner: Some(info.sender.to_string()),
                            unstaking_duration,
                            token_address: address.to_string(),
                            manager: None,
                        })?,
                    };
                    let msg = SubMsg::reply_on_success(msg, INSTANTIATE_STAKING_REPLY_ID);
                    Ok(Response::default()
                        .add_attribute("action", "instantiate")
                        .add_attribute("token", "existing_token")
                        .add_attribute("token_address", address)
                        .add_submessage(msg))
                }
            }
        }
        TokenInfo::New {
            code_id,
            label,
            name,
            symbol,
            decimals,
            mut initial_balances,
            marketing,
            staking_code_id,
            unstaking_duration,
            initial_dao_balance: _,
        } => {
            let initial_supply = initial_balances
                .iter()
                .fold(Uint128::zero(), |p, n| p + n.amount);
            // Cannot instantiate with no initial token owners because it would immediately lock the DAO.
            if initial_supply.is_zero() {
                return Err(ContractError::InitialBalancesError {});
            }

            // Add DAO initial balance to initial_balances vector if defined.
            if let Some(initial_dao_balance) = msg.initial_dao_balance {
                if initial_dao_balance > Uint128::zero() {
                    initial_balances.push(Cw20Coin {
                        address: info.sender.to_string(),
                        amount: initial_dao_balance,
                    });
                }
            }

            STAKING_CONTRACT_CODE_ID.save(deps.storage, &staking_code_id)?;
            STAKING_CONTRACT_UNSTAKING_DURATION.save(deps.storage, &unstaking_duration)?;

            let msg = WasmMsg::Instantiate {
                admin: Some(info.sender.to_string()),
                code_id,
                msg: to_binary(&cw20_base::msg::InstantiateMsg {
                    name,
                    symbol,
                    decimals,
                    initial_balances,
                    mint: Some(cw20::MinterResponse {
                        minter: info.sender.to_string(),
                        cap: None,
                    }),
                    marketing,
                })?,
                funds: vec![],
                label,
            };
            let msg = SubMsg::reply_on_success(msg, INSTANTIATE_TOKEN_REPLY_ID);

            Ok(Response::default()
                .add_attribute("action", "instantiate")
                .add_attribute("token", "new_token")
                .add_submessage(msg))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::StakeChangedHook(stake_changed_hook_msg) => {
            execute_stake_changed_hook(deps, env, info, stake_changed_hook_msg)?;
        }
    }
    Ok(Response::new().add_attribute("action", "execute"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::VotingPowerAtHeight { address, height } => {
            query_voting_power_at_height(deps, env, address, height)
        }
        QueryMsg::TotalPowerAtHeight { height } => query_total_power_at_height(deps, env, height),
        QueryMsg::Info {} => query_info(deps),
    }
}

pub fn query_voting_power_at_height(
    deps: Deps,
    env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<Binary> {
    let address = deps.api.addr_validate(&address)?;
    let power = VOTE_WEIGHTS
        .may_load_at_height(deps.storage, &address, height.unwrap_or(env.block.height))?
        .unwrap_or_default();

    to_binary(&cw_core_interface::voting::VotingPowerAtHeightResponse {
        power,
        height: env.block.height,
    })
}

pub fn execute_stake_changed_hook(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    stake_changed_hook_msg: StakeChangedHookMsg,
) -> Result<(), ContractError> {
    match stake_changed_hook_msg {
        StakeChangedHookMsg::Stake {
            addr,
            amount: _,
            staked_addresses_count,
        } => {
            shuffle_voting_power(deps, env, staked_addresses_count, Some(addr), None)?;
        }
        StakeChangedHookMsg::Unstake {
            addr,
            amount: _,
            staked_addresses_count,
        } => {
            shuffle_voting_power(deps, env, staked_addresses_count, None, Some(addr))?;
        }
    }
    Ok(())
}

// Randomly distributes vote power amongst addresses.
pub fn shuffle_voting_power(
    deps: DepsMut,
    env: Env,
    addresses_count: u64,
    addr_to_add: Option<Addr>,
    addr_to_remove: Option<Addr>,
) -> StdResult<()> {
    // Generate voting weights based on total staked amount and number of participating addresses.
    let weights = generate_random_voting_weights(env.block.height, addresses_count);

    // Remove unstaked address if exists
    if let Some(addr) = addr_to_remove {
        VOTE_WEIGHTS.remove(deps.storage, &addr, env.block.height)?;
    }

    // Collect addresses into vector.
    let mut addresses = VOTE_WEIGHTS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .into_iter()
        .map(|x| match x {
            Err(_) => Err(x.unwrap_err()),
            Ok(_) => Ok(x.unwrap().0),
        })
        .collect::<StdResult<Vec<Addr>>>()?;

    // Add staked address if exists
    if let Some(addr) = addr_to_add {
        addresses.push(addr);
    }

    // Assign random weights to addresses.
    for (i, addr) in addresses.into_iter().enumerate() {
        VOTE_WEIGHTS.save(deps.storage, &addr, &weights[i], env.block.height)?;
    }

    // Add sum of vote weights to total power map.
    let sum = VOTE_WEIGHTS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|x| match x {
            Err(_) => Err(x.unwrap_err()),
            Ok(_) => Ok(x.unwrap().1),
        })
        .collect::<StdResult<Vec<Uint128>>>()?
        .iter()
        .sum::<Uint128>();

    HEIGHT_TO_TOTAL_POWER.save(deps.storage, env.block.height, &sum)?;
    Ok(())
}

// This function uses block height as the seed for a minimal PRNG to generate our random values.
// This is to allow for every validator node to deterministically derive a sequence of weights.
// The downside to using block_height, of course, is that our random sequences will be predictable.
// This may not be a big issue given this doesn't allow for more ways to take over the DAO, just to know in advance the
// distribution of voting powers. In the future we can explore using an on-chain drand-based beacon as a seed for unpredictable randomness: https://github.com/confio/rand
pub fn generate_random_voting_weights(block_height: u64, addresses_count: u64) -> Vec<Uint128> {
    let mut rng = Rand32::new(block_height);
    let mut random_weights: Vec<Uint128> = vec![];
    let mut i = 0;

    // Generate addresses_count # of random numbers
    while i < addresses_count {
        random_weights.push(Uint128::from(
            rng.rand_range(1..addresses_count as u32) as u128
        ));
        i += 1;
    }

    random_weights
}

// Total voting power will differ from total staked power, given we have generated random weights.
// Returns the sum of voting power at the given height.
pub fn query_total_power_at_height(deps: Deps, env: Env, height: Option<u64>) -> StdResult<Binary> {
    let height = height.unwrap_or(env.block.height);
    let sum = HEIGHT_TO_TOTAL_POWER.load(deps.storage, height)?;
    to_binary(&cw_core_interface::voting::VotingPowerAtHeightResponse { power: sum, height })
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_binary(&cw_core_interface::voting::InfoResponse { info })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_TOKEN_REPLY_ID => {
            let res = parse_reply_instantiate_data(msg);
            match res {
                Ok(res) => {
                    let token = TOKEN.may_load(deps.storage)?;
                    if token.is_some() {
                        return Err(ContractError::DuplicateToken {});
                    }
                    let token = deps.api.addr_validate(&res.contract_address)?;
                    TOKEN.save(deps.storage, &token)?;
                    let staking_contract_code_id = STAKING_CONTRACT_CODE_ID.load(deps.storage)?;
                    let unstaking_duration =
                        STAKING_CONTRACT_UNSTAKING_DURATION.load(deps.storage)?;
                    let dao = DAO_ADDRESS.load(deps.storage)?;
                    let msg = WasmMsg::Instantiate {
                        code_id: staking_contract_code_id,
                        funds: vec![],
                        admin: Some(dao.to_string()),
                        label: env.contract.address.to_string(),
                        msg: to_binary(&stake_cw20::msg::InstantiateMsg {
                            owner: Some(dao.to_string()),
                            unstaking_duration,
                            token_address: token.to_string(),
                            manager: None,
                        })?,
                    };
                    let msg = SubMsg::reply_on_success(msg, INSTANTIATE_STAKING_REPLY_ID);
                    Ok(Response::default()
                        .add_attribute("token_address", token)
                        .add_submessage(msg))
                }
                Err(_) => Err(ContractError::TokenInstantiateError {}),
            }
        }
        INSTANTIATE_STAKING_REPLY_ID => {
            let res = parse_reply_instantiate_data(msg);
            match res {
                Ok(res) => {
                    // Validate contract address
                    let staking_contract_addr = deps.api.addr_validate(&res.contract_address)?;

                    // Check if we have a duplicate
                    let staking = STAKING_CONTRACT.may_load(deps.storage)?;
                    if staking.is_some() {
                        return Err(ContractError::DuplicateStakingContract {});
                    }

                    // Save staking contract addr
                    STAKING_CONTRACT.save(deps.storage, &staking_contract_addr)?;

                    Ok(Response::new().add_attribute("staking_contract", staking_contract_addr))
                }
                Err(_) => Err(ContractError::TokenInstantiateError {}),
            }
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}
