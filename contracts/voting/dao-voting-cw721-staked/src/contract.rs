use crate::hooks::{stake_hook_msgs, unstake_hook_msgs};
use crate::msg::{ActiveThresholdResponse, NftContract};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{
    register_staked_nft, register_unstaked_nfts, Config, ACTIVE_THRESHOLD, CONFIG, DAO, HOOKS,
    INITITIAL_NFTS, MAX_CLAIMS, NFT_BALANCES, NFT_CLAIMS, STAKED_NFTS_PER_OWNER, TOTAL_STAKED_NFTS,
};
use crate::ContractError;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, CosmosMsg, Decimal, Deps, DepsMut, Empty, Env,
    MessageInfo, Reply, Response, StdResult, SubMsg, Uint128, Uint256, WasmMsg,
};
use cw2::set_contract_version;
use cw721::{Cw721ReceiveMsg, NumTokensResponse};
use cw_storage_plus::Bound;
use cw_utils::{parse_reply_instantiate_data, Duration};
use dao_interface::state::Admin;
use dao_interface::voting::IsActiveResponse;
use dao_voting::threshold::ActiveThreshold;

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-voting-cw721-staked";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_NFT_CONTRACT_REPLY_ID: u64 = 0;

// We multiply by this when calculating needed power for being active
// when using active threshold with percent
const PRECISION_FACTOR: u128 = 10u128.pow(9);

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<Empty>, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    DAO.save(deps.storage, &info.sender)?;

    let owner = msg
        .owner
        .as_ref()
        .map(|owner| match owner {
            Admin::Address { addr } => deps.api.addr_validate(addr),
            Admin::CoreModule {} => Ok(info.sender.clone()),
        })
        .transpose()?;

    if let Some(active_threshold) = msg.active_threshold.as_ref() {
        match active_threshold {
            ActiveThreshold::Percentage { percent } => {
                if percent > &Decimal::percent(100) || percent.is_zero() {
                    return Err(ContractError::InvalidActivePercentage {});
                }
            }
            ActiveThreshold::AbsoluteCount { count } => {
                if count.is_zero() {
                    return Err(ContractError::ZeroActiveCount {});
                }
            }
        }
        ACTIVE_THRESHOLD.save(deps.storage, active_threshold)?;
    }

    TOTAL_STAKED_NFTS.save(deps.storage, &Uint128::zero(), env.block.height)?;

    match msg.nft_contract {
        NftContract::Existing { address } => {
            let config = Config {
                owner: owner.clone(),
                nft_address: deps.api.addr_validate(&address)?,
                unstaking_duration: msg.unstaking_duration,
            };
            CONFIG.save(deps.storage, &config)?;

            Ok(Response::default()
                .add_attribute("method", "instantiate")
                .add_attribute("nft_contract", address)
                .add_attribute(
                    "owner",
                    owner
                        .map(|a| a.into_string())
                        .unwrap_or_else(|| "None".to_string()),
                ))
        }
        NftContract::New {
            code_id,
            label,
            msg: instantiate_msg,
            initial_nfts,
        } => {
            // The instantiate message for the NFT contract is likely to be a cw721 or sg721
            // instantiate message. We need to deserialize it and update the minter to be this
            // contract.
            //
            // Custom NFT contracts with Instantiate messages that do fit the cw721 or sg721
            // interface will not work with this contract. Use NftContract::Existing instead.
            let cw721_instantiate_msg = from_binary::<cw721_base::InstantiateMsg>(&instantiate_msg);
            let sg721_instantiate_msg = from_binary::<sg721::InstantiateMsg>(&instantiate_msg);

            // Update the minter to be this contract, reserialize into Binary
            let instantiate_msg = match (cw721_instantiate_msg, sg721_instantiate_msg) {
                (Ok(mut cw721_instantiate_msg), _) => {
                    cw721_instantiate_msg.minter = env.contract.address.to_string();
                    to_binary(&cw721_instantiate_msg)?
                }
                (_, Ok(mut sg721_instantiate_msg)) => {
                    sg721_instantiate_msg.minter = env.contract.address.to_string();
                    to_binary(&sg721_instantiate_msg)?
                }
                _ => return Err(ContractError::NftInstantiateError {}),
            };

            // Check there is at least one NFT to initialize
            if initial_nfts.is_empty() {
                return Err(ContractError::NoInitialNfts {});
            }

            // Save config with empty nft_address
            let config = Config {
                owner: owner.clone(),
                nft_address: Addr::unchecked(""),
                unstaking_duration: msg.unstaking_duration,
            };
            CONFIG.save(deps.storage, &config)?;

            // Save initial NFTs for use in reply
            INITITIAL_NFTS.save(deps.storage, &initial_nfts)?;

            // Create instantiate submessage for NFT roles contract
            let instantiate_msg = SubMsg::reply_on_success(
                WasmMsg::Instantiate {
                    code_id,
                    funds: vec![],
                    admin: Some(info.sender.to_string()),
                    label,
                    msg: instantiate_msg,
                },
                INSTANTIATE_NFT_CONTRACT_REPLY_ID,
            );

            Ok(Response::default()
                .add_submessage(instantiate_msg)
                .add_attribute(
                    "owner",
                    owner
                        .map(|a| a.into_string())
                        .unwrap_or_else(|| "None".to_string()),
                ))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<Empty>, ContractError> {
    match msg {
        ExecuteMsg::ReceiveNft(msg) => execute_stake(deps, env, info, msg),
        ExecuteMsg::Unstake { token_ids } => execute_unstake(deps, env, info, token_ids),
        ExecuteMsg::ClaimNfts {} => execute_claim_nfts(deps, env, info),
        ExecuteMsg::UpdateConfig { owner, duration } => {
            execute_update_config(info, deps, owner, duration)
        }
        ExecuteMsg::AddHook { addr } => execute_add_hook(deps, info, addr),
        ExecuteMsg::RemoveHook { addr } => execute_remove_hook(deps, info, addr),
        ExecuteMsg::UpdateActiveThreshold { new_threshold } => {
            execute_update_active_threshold(deps, env, info, new_threshold)
        }
    }
}

pub fn execute_stake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.nft_address {
        return Err(ContractError::InvalidToken {
            received: info.sender,
            expected: config.nft_address,
        });
    }
    let staker = deps.api.addr_validate(&wrapper.sender)?;
    register_staked_nft(deps.storage, env.block.height, &staker, &wrapper.token_id)?;
    let hook_msgs = stake_hook_msgs(deps.storage, staker.clone(), wrapper.token_id.clone())?;
    Ok(Response::default()
        .add_submessages(hook_msgs)
        .add_attribute("action", "stake")
        .add_attribute("from", staker)
        .add_attribute("token_id", wrapper.token_id))
}

pub fn execute_unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_ids: Vec<String>,
) -> Result<Response, ContractError> {
    if token_ids.is_empty() {
        return Err(ContractError::ZeroUnstake {});
    }

    register_unstaked_nfts(deps.storage, env.block.height, &info.sender, &token_ids)?;

    // Provided that the backing cw721 contract is non-malicious:
    //
    // 1. no token that has been staked may be staked again before
    //    first being unstaked.
    //
    // Provided that the other methods on this contract are functional:
    //
    // 2. there will never exist a pending claim for a token that is
    //    unstaked.
    // 3. (6) => claims may only be created for tokens that are staked.
    // 4. (1) && (2) && (3) => there will never be a staked NFT for
    //    which there is also a pending claim.
    //
    // (aside: the requirement on (1) for (4) may be confusing. it is
    // needed because if a token could be staked more than once, a
    // token could be staked, moved into the claims queue, and then
    // staked again, in which case the token is both staked and has a
    // pending claim.)
    //
    // If we reach this point in execution, `register_unstaked_nfts`
    // has not errored and thus:
    //
    // 5. token_ids contains no duplicate values.
    // 6. all NFTs in token_ids were staked by `info.sender`
    // 7. (4) && (6) => none of the tokens in token_ids are in the
    //    claims queue for `info.sender`
    //
    // (5) && (7) are the invariants for calling `create_nft_claims`
    // so if we reach this point in execution, we may safely create
    // claims.

    let hook_msgs = unstake_hook_msgs(deps.storage, info.sender.clone(), token_ids.clone())?;

    let config = CONFIG.load(deps.storage)?;
    match config.unstaking_duration {
        None => {
            let return_messages = token_ids
                .into_iter()
                .map(|token_id| -> StdResult<WasmMsg> {
                    Ok(cosmwasm_std::WasmMsg::Execute {
                        contract_addr: config.nft_address.to_string(),
                        msg: to_binary(&cw721::Cw721ExecuteMsg::TransferNft {
                            recipient: info.sender.to_string(),
                            token_id,
                        })?,
                        funds: vec![],
                    })
                })
                .collect::<StdResult<Vec<_>>>()?;

            Ok(Response::default()
                .add_messages(return_messages)
                .add_submessages(hook_msgs)
                .add_attribute("action", "unstake")
                .add_attribute("from", info.sender)
                .add_attribute("claim_duration", "None"))
        }

        Some(duration) => {
            let outstanding_claims = NFT_CLAIMS
                .query_claims(deps.as_ref(), &info.sender)?
                .nft_claims;
            if outstanding_claims.len() + token_ids.len() > MAX_CLAIMS as usize {
                return Err(ContractError::TooManyClaims {});
            }

            // Out of gas here is fine - just try again with fewer
            // tokens.
            NFT_CLAIMS.create_nft_claims(
                deps.storage,
                &info.sender,
                token_ids,
                duration.after(&env.block),
            )?;

            Ok(Response::default()
                .add_attribute("action", "unstake")
                .add_submessages(hook_msgs)
                .add_attribute("from", info.sender)
                .add_attribute("claim_duration", format!("{duration}")))
        }
    }
}

pub fn execute_claim_nfts(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let nfts = NFT_CLAIMS.claim_nfts(deps.storage, &info.sender, &env.block)?;
    if nfts.is_empty() {
        return Err(ContractError::NothingToClaim {});
    }

    let config = CONFIG.load(deps.storage)?;

    let msgs = nfts
        .into_iter()
        .map(|nft| -> StdResult<CosmosMsg> {
            Ok(WasmMsg::Execute {
                contract_addr: config.nft_address.to_string(),
                msg: to_binary(&cw721::Cw721ExecuteMsg::TransferNft {
                    recipient: info.sender.to_string(),
                    token_id: nft,
                })?,
                funds: vec![],
            }
            .into())
        })
        .collect::<StdResult<Vec<_>>>()?;

    Ok(Response::default()
        .add_messages(msgs)
        .add_attribute("action", "claim_nfts")
        .add_attribute("from", info.sender))
}

pub fn execute_update_config(
    info: MessageInfo,
    deps: DepsMut,
    new_owner: Option<String>,
    duration: Option<Duration>,
) -> Result<Response, ContractError> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    if config.owner.map_or(true, |owner| owner != info.sender) {
        return Err(ContractError::NotOwner {});
    }

    let new_owner = new_owner
        .map(|new_owner| deps.api.addr_validate(&new_owner))
        .transpose()?;

    config.owner = new_owner;
    config.unstaking_duration = duration;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default()
        .add_attribute("action", "update_config")
        .add_attribute(
            "owner",
            config
                .owner
                .map(|a| a.to_string())
                .unwrap_or_else(|| "none".to_string()),
        )
        .add_attribute(
            "unstaking_duration",
            config
                .unstaking_duration
                .map(|d| d.to_string())
                .unwrap_or_else(|| "none".to_string()),
        ))
}

pub fn execute_add_hook(
    deps: DepsMut,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;
    if config.owner.map_or(true, |owner| owner != info.sender) {
        return Err(ContractError::NotOwner {});
    }

    let hook = deps.api.addr_validate(&addr)?;
    HOOKS.add_hook(deps.storage, hook)?;

    Ok(Response::default()
        .add_attribute("action", "add_hook")
        .add_attribute("hook", addr))
}

pub fn execute_remove_hook(
    deps: DepsMut,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    let config: Config = CONFIG.load(deps.storage)?;
    if config.owner.map_or(true, |owner| owner != info.sender) {
        return Err(ContractError::NotOwner {});
    }

    let hook = deps.api.addr_validate(&addr)?;
    HOOKS.remove_hook(deps.storage, hook)?;

    Ok(Response::default()
        .add_attribute("action", "remove_hook")
        .add_attribute("hook", addr))
}

pub fn execute_update_active_threshold(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_active_threshold: Option<ActiveThreshold>,
) -> Result<Response, ContractError> {
    let dao = DAO.load(deps.storage)?;
    if info.sender != dao {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(active_threshold) = new_active_threshold {
        match active_threshold {
            ActiveThreshold::Percentage { percent } => {
                if percent > Decimal::percent(100) || percent.is_zero() {
                    return Err(ContractError::InvalidActivePercentage {});
                }
            }
            ActiveThreshold::AbsoluteCount { count } => {
                if count.is_zero() {
                    return Err(ContractError::ZeroActiveCount {});
                }
            }
        }
        ACTIVE_THRESHOLD.save(deps.storage, &active_threshold)?;
    } else {
        ACTIVE_THRESHOLD.remove(deps.storage);
    }

    Ok(Response::new().add_attribute("action", "update_active_threshold"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::ActiveThreshold {} => query_active_threshold(deps),
        QueryMsg::Config {} => query_config(deps),
        QueryMsg::Dao {} => query_dao(deps),
        QueryMsg::Info {} => query_info(deps),
        QueryMsg::IsActive {} => query_is_active(deps, env),
        QueryMsg::NftClaims { address } => query_nft_claims(deps, address),
        QueryMsg::Hooks {} => query_hooks(deps),
        QueryMsg::StakedNfts {
            address,
            start_after,
            limit,
        } => query_staked_nfts(deps, address, start_after, limit),
        QueryMsg::TotalPowerAtHeight { height } => query_total_power_at_height(deps, env, height),
        QueryMsg::VotingPowerAtHeight { address, height } => {
            query_voting_power_at_height(deps, env, address, height)
        }
    }
}

pub fn query_active_threshold(deps: Deps) -> StdResult<Binary> {
    to_binary(&ActiveThresholdResponse {
        active_threshold: ACTIVE_THRESHOLD.may_load(deps.storage)?,
    })
}

pub fn query_is_active(deps: Deps, env: Env) -> StdResult<Binary> {
    let threshold = ACTIVE_THRESHOLD.may_load(deps.storage)?;
    if let Some(threshold) = threshold {
        let config = CONFIG.load(deps.storage)?;
        let staked_nfts = TOTAL_STAKED_NFTS
            .may_load_at_height(deps.storage, env.block.height)?
            .unwrap_or_default();
        let total_nfts: NumTokensResponse = deps.querier.query_wasm_smart(
            config.nft_address,
            &cw721_base::msg::QueryMsg::<Empty>::NumTokens {},
        )?;

        match threshold {
            ActiveThreshold::AbsoluteCount { count } => to_binary(&IsActiveResponse {
                active: staked_nfts >= count,
            }),
            ActiveThreshold::Percentage { percent } => {
                // Check if there are any staked NFTs
                if staked_nfts.is_zero() {
                    return to_binary(&IsActiveResponse { active: false });
                }

                // percent is bounded between [0, 100]. decimal
                // represents percents in u128 terms as p *
                // 10^15. this bounds percent between [0, 10^17].
                //
                // total_potential_power is bounded between [0, 2^64]
                // as it tracks the count of NFT tokens which has
                // a max supply of 2^64.
                //
                // with our precision factor being 10^9:
                //
                // total_nfts <= 2^64 * 10^9 <= 2^256
                //
                // so we're good to put that in a u256.
                //
                // multiply_ratio promotes to a u512 under the hood,
                // so it won't overflow, multiplying by a percent less
                // than 100 is gonna make something the same size or
                // smaller, applied + 10^9 <= 2^128 * 10^9 + 10^9 <=
                // 2^256, so the top of the round won't overflow, and
                // rounding is rounding down, so the whole thing can
                // be safely unwrapped at the end of the day thank you
                // for coming to my ted talk.
                let total_nfts_count = Uint128::from(total_nfts.count).full_mul(PRECISION_FACTOR);

                // under the hood decimals are `atomics / 10^decimal_places`.
                // cosmwasm doesn't give us a Decimal * Uint256
                // implementation so we take the decimal apart and
                // multiply by the fraction.
                let applied = total_nfts_count.multiply_ratio(
                    percent.atomics(),
                    Uint256::from(10u64).pow(percent.decimal_places()),
                );
                let rounded = (applied + Uint256::from(PRECISION_FACTOR) - Uint256::from(1u128))
                    / Uint256::from(PRECISION_FACTOR);
                let count: Uint128 = rounded.try_into().unwrap();

                // staked_nfts >= total_nfts * percent
                to_binary(&IsActiveResponse {
                    active: staked_nfts >= count,
                })
            }
        }
    } else {
        to_binary(&IsActiveResponse { active: true })
    }
}

pub fn query_voting_power_at_height(
    deps: Deps,
    env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<Binary> {
    let address = deps.api.addr_validate(&address)?;
    let height = height.unwrap_or(env.block.height);
    let power = NFT_BALANCES
        .may_load_at_height(deps.storage, &address, height)?
        .unwrap_or_default();
    to_binary(&dao_interface::voting::VotingPowerAtHeightResponse { power, height })
}

pub fn query_total_power_at_height(deps: Deps, env: Env, height: Option<u64>) -> StdResult<Binary> {
    let height = height.unwrap_or(env.block.height);
    let power = TOTAL_STAKED_NFTS
        .may_load_at_height(deps.storage, height)?
        .unwrap_or_default();
    to_binary(&dao_interface::voting::TotalPowerAtHeightResponse { power, height })
}

pub fn query_config(deps: Deps) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    to_binary(&config)
}

pub fn query_dao(deps: Deps) -> StdResult<Binary> {
    let dao = DAO.load(deps.storage)?;
    to_binary(&dao)
}

pub fn query_nft_claims(deps: Deps, address: String) -> StdResult<Binary> {
    to_binary(&NFT_CLAIMS.query_claims(deps, &deps.api.addr_validate(&address)?)?)
}

pub fn query_hooks(deps: Deps) -> StdResult<Binary> {
    to_binary(&HOOKS.query_hooks(deps)?)
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_binary(&dao_interface::voting::InfoResponse { info })
}

pub fn query_staked_nfts(
    deps: Deps,
    address: String,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let prefix = deps.api.addr_validate(&address)?;
    let prefix = STAKED_NFTS_PER_OWNER.prefix(&prefix);

    let start_after = start_after.as_deref().map(Bound::exclusive);
    let range = prefix.keys(
        deps.storage,
        start_after,
        None,
        cosmwasm_std::Order::Ascending,
    );
    let range: StdResult<Vec<String>> = match limit {
        Some(l) => range.take(l as usize).collect(),
        None => range.collect(),
    };
    to_binary(&range?)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_NFT_CONTRACT_REPLY_ID => {
            let res = parse_reply_instantiate_data(msg);
            match res {
                Ok(res) => {
                    let dao = DAO.load(deps.storage)?;
                    let nft_contract = res.contract_address;

                    // Save NFT contract to config
                    let mut config = CONFIG.load(deps.storage)?;
                    config.nft_address = deps.api.addr_validate(&nft_contract)?;
                    CONFIG.save(deps.storage, &config)?;

                    let initial_nfts = INITITIAL_NFTS.load(deps.storage)?;

                    // Add mint submessages
                    let mint_submessages: Vec<SubMsg> = initial_nfts
                        .iter()
                        .flat_map(|nft| -> Result<SubMsg, ContractError> {
                            Ok(SubMsg::new(WasmMsg::Execute {
                                contract_addr: nft_contract.clone(),
                                funds: vec![],
                                msg: nft.clone(),
                            }))
                        })
                        .collect::<Vec<SubMsg>>();

                    // Clear space
                    INITITIAL_NFTS.remove(deps.storage);

                    // Update minter message
                    let update_minter_msg = WasmMsg::Execute {
                        contract_addr: nft_contract.clone(),
                        msg: to_binary(
                            &cw721_base::msg::ExecuteMsg::<Empty, Empty>::UpdateOwnership(
                                cw721_base::Action::TransferOwnership {
                                    new_owner: dao.to_string(),
                                    expiry: None,
                                },
                            ),
                        )?,
                        funds: vec![],
                    };

                    Ok(Response::default()
                        .add_attribute("method", "instantiate")
                        .add_attribute("nft_contract", nft_contract)
                        .add_message(update_minter_msg)
                        .add_submessages(mint_submessages))
                }
                Err(_) => Err(ContractError::NftInstantiateError {}),
            }
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}
