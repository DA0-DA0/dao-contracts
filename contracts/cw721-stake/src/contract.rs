use crate::hooks::{stake_hook_msgs, unstake_hook_msgs};
#[cfg(not(feature = "library"))]
use crate::msg::{
    ExecuteMsg, GetConfigResponse, GetHooksResponse, InstantiateMsg, Owner, QueryMsg,
    StakedBalanceAtHeightResponse, TotalStakedAtHeightResponse,
};
use crate::state::{
    Config, CONFIG, HOOKS, MAX_CLAIMS, NFT_CLAIMS, STAKED_NFTS_PER_OWNER, TOTAL_STAKED_NFTS,
};
use crate::ContractError;
use cosmwasm_std::{
    entry_point, to_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Response,
    StdError, StdResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw721::Cw721ReceiveMsg;
use cw721_controllers::NftClaimsResponse;
use cw_utils::Duration;
use std::collections::HashSet;
use std::convert::{From, TryFrom};

const CONTRACT_NAME: &str = "crates.io:stake_cw721";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<Empty>, ContractError> {
    let owner = msg
        .owner
        .as_ref()
        .map(|owner| match owner {
            Owner::Addr(address) => deps.api.addr_validate(address),
            Owner::Instantiator {} => Ok(info.sender),
        })
        .transpose()?;
    let manager = msg
        .manager
        .as_ref()
        .map(|h| deps.api.addr_validate(h))
        .transpose()?;

    let config = Config {
        owner: owner.clone(),
        manager,
        nft_address: deps.api.addr_validate(&msg.nft_address)?,
        unstaking_duration: msg.unstaking_duration,
    };
    CONFIG.save(deps.storage, &config)?;
    TOTAL_STAKED_NFTS.save(deps.storage, &Uint128::zero(), env.block.height)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default()
        .add_attribute("method", "instantiate")
        .add_attribute("nft_contract", msg.nft_address)
        .add_attribute(
            "owner",
            owner
                .map(|a| a.into_string())
                .unwrap_or_else(|| "None".to_string()),
        )
        .add_attribute("manager", msg.manager.unwrap_or_else(|| "None".to_string())))
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
        ExecuteMsg::UpdateConfig {
            owner,
            manager,
            duration,
        } => execute_update_config(info, deps, owner, manager, duration),
        ExecuteMsg::AddHook { addr } => execute_add_hook(deps, env, info, addr),
        ExecuteMsg::RemoveHook { addr } => execute_remove_hook(deps, env, info, addr),
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

    let sender = deps.api.addr_validate(&wrapper.sender)?;
    STAKED_NFTS_PER_OWNER.update(
        deps.storage,
        &sender,
        env.block.height,
        |nft_collection| -> StdResult<HashSet<String>> {
            let mut updated_nft_collection = nft_collection.unwrap_or_default();
            updated_nft_collection.insert(wrapper.token_id.clone());
            Ok(updated_nft_collection)
        },
    )?;

    TOTAL_STAKED_NFTS.update(
        deps.storage,
        env.block.height,
        |total_staked| -> StdResult<_> {
            total_staked
                .unwrap()
                .checked_add(Uint128::new(1))
                .map_err(StdError::overflow)
        },
    )?;

    let hook_msgs = stake_hook_msgs(deps.storage, sender.clone(), wrapper.token_id.clone())?;
    Ok(Response::default()
        .add_submessages(hook_msgs)
        .add_attribute("action", "stake")
        .add_attribute("from", sender)
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

    let config = CONFIG.load(deps.storage)?;

    STAKED_NFTS_PER_OWNER.update(
        deps.storage,
        &info.sender,
        env.block.height,
        |nft_collection| -> Result<HashSet<String>, ContractError> {
            if let Some(mut nft_collection) = nft_collection {
                // Some benchmarking suggests this is actually the
                // fastest way to remove a list of items from a
                // HashSet.
                for token_id in token_ids.iter() {
                    // This will implicitly check for duplicates in
                    // the input vector as removing twice will fail
                    // the second time around.
                    let was_present = nft_collection.remove(token_id);
                    if !was_present {
                        return Err(ContractError::NotStaked {});
                    }
                }
                Ok(nft_collection)
            } else {
                Err(ContractError::NotStaked {})
            }
        },
    )?;

    TOTAL_STAKED_NFTS.update(
        deps.storage,
        env.block.height,
        |total_staked| -> StdResult<_> {
            total_staked
                .unwrap()
                .checked_sub(Uint128::new(token_ids.len() as u128))
                .map_err(StdError::overflow)
        },
    )?;

    let hook_msgs = unstake_hook_msgs(deps.storage, info.sender.clone(), token_ids.clone())?;
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
            if outstanding_claims.len() >= MAX_CLAIMS as usize {
                return Err(ContractError::TooManyClaims {});
            }

            // Out of gas here is fine - just try again with fewer
            // tokens.
            for token_id in token_ids.into_iter() {
                NFT_CLAIMS.create_nft_claim(
                    deps.storage,
                    &info.sender,
                    token_id.clone(),
                    duration.after(&env.block),
                )?;
            }

            Ok(Response::default()
                .add_attribute("action", "unstake")
                .add_submessages(hook_msgs)
                .add_attribute("from", info.sender)
                .add_attribute("claim_duration", format!("{}", duration)))
        }
    }
}

pub fn execute_claim_nfts(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let nfts = NFT_CLAIMS.claim_nfts(deps.storage, &info.sender, &_env.block)?;
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
        .add_attribute("from", info.sender.clone()))
}

pub fn execute_update_config(
    info: MessageInfo,
    deps: DepsMut,
    new_owner: Option<String>,
    new_manager: Option<String>,
    duration: Option<Duration>,
) -> Result<Response, ContractError> {
    let new_owner = new_owner
        .map(|new_owner| deps.api.addr_validate(&new_owner))
        .transpose()?;
    let new_manager = new_manager
        .map(|new_manager| deps.api.addr_validate(&new_manager))
        .transpose()?;

    let mut config: Config = CONFIG.load(deps.storage)?;
    if Some(info.sender.clone()) != config.owner && Some(info.sender.clone()) != config.manager {
        return Err(ContractError::Unauthorized {});
    };

    if Some(info.sender) != config.owner && new_owner != config.owner {
        return Err(ContractError::OnlyOwnerCanChangeOwner {});
    };

    config.owner = new_owner;
    config.manager = new_manager;
    config.unstaking_duration = duration;
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default()
        .add_attribute("action", "update_config")
        .add_attribute(
            "owner",
            config
                .owner
                .map(|a| a.to_string())
                .unwrap_or_else(|| "None".to_string()),
        )
        .add_attribute(
            "manager",
            config
                .manager
                .map(|a| a.to_string())
                .unwrap_or_else(|| "None".to_string()),
        ))
}

pub fn execute_add_hook(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    let addr = deps.api.addr_validate(&addr)?;
    let config: Config = CONFIG.load(deps.storage)?;
    if config.owner != Some(info.sender.clone()) && config.manager != Some(info.sender) {
        return Err(ContractError::Unauthorized {});
    };

    HOOKS.add_hook(deps.storage, addr.clone())?;

    Ok(Response::default()
        .add_attribute("action", "add_hook")
        .add_attribute("hook", addr))
}

pub fn execute_remove_hook(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    let addr = deps.api.addr_validate(&addr)?;
    let config: Config = CONFIG.load(deps.storage)?;
    if config.owner != Some(info.sender.clone()) && config.manager != Some(info.sender) {
        return Err(ContractError::Unauthorized {});
    };

    HOOKS.remove_hook(deps.storage, addr.clone())?;

    Ok(Response::default()
        .add_attribute("action", "remove_hook")
        .add_attribute("hook", addr))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
        QueryMsg::StakedBalanceAtHeight { address, height } => {
            to_binary(&query_staked_balance_at_height(deps, env, address, height)?)
        }
        QueryMsg::TotalStakedAtHeight { height } => {
            to_binary(&query_total_staked_at_height(deps, env, height)?)
        }
        QueryMsg::NftClaims { address } => to_binary(&query_nft_claims(deps, address)?),
        QueryMsg::GetHooks {} => to_binary(&query_hooks(deps)?),
        QueryMsg::VotingPowerAtHeight { address, height } => {
            query_voting_power_at_height(deps, env, address, height)
        }
        QueryMsg::TotalPowerAtHeight { height } => query_total_power_at_height(deps, env, height),
        QueryMsg::Info {} => query_info(deps),
    }
}

pub fn query_staked_balance_at_height(
    deps: Deps,
    env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<StakedBalanceAtHeightResponse> {
    let address = deps.api.addr_validate(&address)?;
    let height = height.unwrap_or(env.block.height);
    let nft_collection = STAKED_NFTS_PER_OWNER
        .may_load_at_height(deps.storage, &address, height)?
        .unwrap_or_default();

    Ok(StakedBalanceAtHeightResponse {
        balance: Uint128::from(u128::try_from(nft_collection.len()).unwrap()),
        height,
    })
}

pub fn query_voting_power_at_height(
    deps: Deps,
    env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<Binary> {
    let address = deps.api.addr_validate(&address)?;
    let height = height.unwrap_or(env.block.height);
    let collection = STAKED_NFTS_PER_OWNER
        .may_load_at_height(deps.storage, &address, height)?
        .unwrap_or_default();
    let power = Uint128::new(collection.len() as u128);

    to_binary(&cw_core_interface::voting::VotingPowerAtHeightResponse { power, height })
}

pub fn query_total_staked_at_height(
    deps: Deps,
    env: Env,
    height: Option<u64>,
) -> StdResult<TotalStakedAtHeightResponse> {
    let height = height.unwrap_or(env.block.height);
    let total_staked_nfts = TOTAL_STAKED_NFTS
        .may_load_at_height(deps.storage, height)?
        .unwrap_or_default();

    Ok(TotalStakedAtHeightResponse {
        total: total_staked_nfts,
        height,
    })
}

pub fn query_total_power_at_height(deps: Deps, env: Env, height: Option<u64>) -> StdResult<Binary> {
    let height = height.unwrap_or(env.block.height);
    let power = TOTAL_STAKED_NFTS
        .may_load_at_height(deps.storage, height)?
        .unwrap_or_default();
    to_binary(&cw_core_interface::voting::TotalPowerAtHeightResponse { power, height })
}

pub fn query_config(deps: Deps) -> StdResult<GetConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(GetConfigResponse {
        owner: config.owner.map(|a| a.to_string()),
        manager: config.manager.map(|a| a.to_string()),
        unstaking_duration: config.unstaking_duration,
        nft_address: config.nft_address.to_string(),
    })
}

pub fn query_nft_claims(deps: Deps, address: String) -> StdResult<NftClaimsResponse> {
    NFT_CLAIMS.query_claims(deps, &deps.api.addr_validate(&address)?)
}

pub fn query_hooks(deps: Deps) -> StdResult<GetHooksResponse> {
    Ok(GetHooksResponse {
        hooks: HOOKS.query_hooks(deps)?.hooks,
    })
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_binary(&cw_core_interface::voting::InfoResponse { info })
}

#[cfg(test)]
mod tests {
    use crate::contract::{CONTRACT_NAME, CONTRACT_VERSION};
    use crate::msg::{
        ExecuteMsg, GetConfigResponse, Owner, QueryMsg, StakedBalanceAtHeightResponse,
        TotalStakedAtHeightResponse,
    };
    use crate::state::MAX_CLAIMS;
    use crate::ContractError;
    use anyhow::Result as AnyResult;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{to_binary, Addr, Empty, MessageInfo, Uint128};
    use cw721_controllers::NftClaim;
    use cw_multi_test::{next_block, App, AppResponse, Contract, ContractWrapper, Executor};
    use cw_utils::Duration;
    use cw_utils::Expiration::AtHeight;
    use std::borrow::BorrowMut;
    use std::convert::TryFrom;

    const ADDR1: &str = "addr0001";
    const ADDR2: &str = "addr0002";
    const ADDR3: &str = "addr0003";
    const ADDR4: &str = "addr0004";
    const NFT_ID1: &str = "fake_nft1";
    const NFT_ID2: &str = "fake_nft2";
    const NFT_ID3: &str = "fake_nft3";
    const NFT_ID4: &str = "fake_nft4";

    fn contract_staking() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        Box::new(contract)
    }

    fn contract_cw721() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw721_base::entry::execute,
            cw721_base::entry::instantiate,
            cw721_base::entry::query,
        );
        Box::new(contract)
    }

    fn mock_app() -> App {
        App::default()
    }

    fn get_nft_balance<T: Into<String>, U: Into<String>>(
        app: &App,
        contract_addr: T,
        address: U,
    ) -> Uint128 {
        let msg = cw721::Cw721QueryMsg::Tokens {
            owner: address.into(),
            start_after: None,
            limit: None,
        };
        let result: cw721::TokensResponse =
            app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
        Uint128::from(u128::try_from(result.tokens.len()).unwrap())
    }

    fn instantiate_cw721(app: &mut App) -> Addr {
        let cw721_id = app.store_code(contract_cw721());
        let msg = cw721_base::msg::InstantiateMsg {
            name: "Test".to_string(),
            symbol: "Test".to_string(),
            minter: ADDR1.to_string(),
        };

        app.instantiate_contract(cw721_id, Addr::unchecked(ADDR1), &msg, &[], "cw721", None)
            .unwrap()
    }

    fn instantiate_staking(
        app: &mut App,
        cw721: Addr,
        unstaking_duration: Option<Duration>,
    ) -> Addr {
        let staking_code_id = app.store_code(contract_staking());
        let msg = crate::msg::InstantiateMsg {
            owner: Some(Owner::Addr("owner".to_string())),
            manager: Some("manager".to_string()),
            nft_address: cw721.to_string(),
            unstaking_duration,
        };
        app.instantiate_contract(
            staking_code_id,
            Addr::unchecked(ADDR1),
            &msg,
            &[],
            "staking",
            None,
        )
        .unwrap()
    }

    fn setup_test_case(app: &mut App, unstaking_duration: Option<Duration>) -> (Addr, Addr) {
        // Instantiate cw721 contract
        let cw721_addr = instantiate_cw721(app);
        app.update_block(next_block);
        // Instantiate staking contract
        let staking_addr = instantiate_staking(app, cw721_addr.clone(), unstaking_duration);
        app.update_block(next_block);
        (staking_addr, cw721_addr)
    }

    fn query_staked_balance<T: Into<String>, U: Into<String>>(
        app: &App,
        contract_addr: T,
        address: U,
    ) -> Uint128 {
        let msg = QueryMsg::StakedBalanceAtHeight {
            address: address.into(),
            height: None,
        };
        let result: StakedBalanceAtHeightResponse =
            app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
        result.balance
    }

    fn query_voting_power<T: Into<String>, U: Into<String>>(
        app: &App,
        contract_addr: T,
        address: U,
        height: Option<u64>,
    ) -> Uint128 {
        let msg = QueryMsg::VotingPowerAtHeight {
            height,
            address: address.into(),
        };
        let result: cw_core_interface::voting::VotingPowerAtHeightResponse =
            app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
        result.power
    }

    fn query_config<T: Into<String>>(app: &App, contract_addr: T) -> GetConfigResponse {
        let msg = QueryMsg::GetConfig {};
        app.wrap().query_wasm_smart(contract_addr, &msg).unwrap()
    }

    fn query_total_staked<T: Into<String>>(app: &App, contract_addr: T) -> Uint128 {
        let msg = QueryMsg::TotalStakedAtHeight { height: None };
        let result: TotalStakedAtHeightResponse =
            app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
        result.total
    }

    fn query_total_power_at_height<T: Into<String>>(
        app: &App,
        contract_addr: T,
        height: Option<u64>,
    ) -> Uint128 {
        let msg = QueryMsg::TotalPowerAtHeight { height };
        let result: cw_core_interface::voting::TotalPowerAtHeightResponse =
            app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
        result.power
    }

    fn query_nft_claims<T: Into<String>, U: Into<String>>(
        app: &App,
        contract_addr: T,
        address: U,
    ) -> Vec<NftClaim> {
        let msg = QueryMsg::NftClaims {
            address: address.into(),
        };
        let result: cw721_controllers::NftClaimsResponse =
            app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
        result.nft_claims
    }

    fn mint_nft(
        app: &mut App,
        cw721_addr: &Addr,
        token_id: String,
        recipient: String,
        info: MessageInfo,
    ) -> AnyResult<AppResponse> {
        let msg = cw721_base::msg::ExecuteMsg::Mint(cw721_base::msg::MintMsg::<Option<Empty>> {
            token_id,
            owner: recipient,
            token_uri: None,
            extension: None,
        });
        app.execute_contract(info.sender, cw721_addr.clone(), &msg, &[])
    }

    fn stake_nft(
        app: &mut App,
        staking_addr: &Addr,
        cw721_addr: &Addr,
        token_id: String,
        info: MessageInfo,
    ) -> AnyResult<AppResponse> {
        let msg = cw721::Cw721ExecuteMsg::SendNft {
            contract: staking_addr.to_string(),
            token_id,
            msg: to_binary("Test").unwrap(),
        };
        app.execute_contract(info.sender, cw721_addr.clone(), &msg, &[])
    }

    fn update_config(
        app: &mut App,
        staking_addr: &Addr,
        info: MessageInfo,
        owner: Option<Addr>,
        manager: Option<Addr>,
        duration: Option<Duration>,
    ) -> AnyResult<AppResponse> {
        let msg = ExecuteMsg::UpdateConfig {
            owner: owner.map(|a| a.to_string()),
            manager: manager.map(|a| a.to_string()),
            duration,
        };
        app.execute_contract(info.sender, staking_addr.clone(), &msg, &[])
    }

    fn unstake_tokens(
        app: &mut App,
        staking_addr: &Addr,
        info: MessageInfo,
        token_ids: Vec<String>,
    ) -> AnyResult<AppResponse> {
        let msg = ExecuteMsg::Unstake { token_ids };
        app.execute_contract(info.sender, staking_addr.clone(), &msg, &[])
    }

    fn claim_nfts(app: &mut App, staking_addr: &Addr, info: MessageInfo) -> AnyResult<AppResponse> {
        let msg = ExecuteMsg::ClaimNfts {};
        app.execute_contract(info.sender, staking_addr.clone(), &msg, &[])
    }

    #[test]
    fn test_update_config() {
        let _deps = mock_dependencies();

        let mut app = mock_app();
        let (staking_addr, _cw721_addr) = setup_test_case(&mut app, None);

        let info = mock_info("owner", &[]);
        let _env = mock_env();
        // Test update admin
        update_config(
            &mut app,
            &staking_addr,
            info,
            Some(Addr::unchecked("owner2")),
            None,
            Some(Duration::Height(100)),
        )
        .unwrap();

        let config = query_config(&app, &staking_addr);
        assert_eq!(config.owner, Some("owner2".to_string()));
        assert_eq!(config.unstaking_duration, Some(Duration::Height(100)));

        // Try updating owner with original owner, which is now invalid
        let info = mock_info("owner", &[]);
        let _err = update_config(
            &mut app,
            &staking_addr,
            info,
            Some(Addr::unchecked("owner3")),
            None,
            Some(Duration::Height(100)),
        )
        .unwrap_err();

        // Add manager
        let info = mock_info("owner2", &[]);
        let _env = mock_env();
        update_config(
            &mut app,
            &staking_addr,
            info,
            Some(Addr::unchecked("owner2")),
            Some(Addr::unchecked("manager")),
            Some(Duration::Height(100)),
        )
        .unwrap();

        let config = query_config(&app, &staking_addr);
        assert_eq!(config.owner, Some("owner2".to_string()));
        assert_eq!(config.manager, Some("manager".to_string()));

        // Manager can update unstaking duration
        let info = mock_info("manager", &[]);
        let _env = mock_env();
        update_config(
            &mut app,
            &staking_addr,
            info,
            Some(Addr::unchecked("owner2")),
            Some(Addr::unchecked("manager")),
            Some(Duration::Height(50)),
        )
        .unwrap();
        let config = query_config(&app, &staking_addr);
        assert_eq!(config.owner, Some("owner2".to_string()));
        assert_eq!(config.unstaking_duration, Some(Duration::Height(50)));

        // Manager cannot update owner
        let info = mock_info("manager", &[]);
        let _env = mock_env();
        update_config(
            &mut app,
            &staking_addr,
            info,
            Some(Addr::unchecked("manager")),
            Some(Addr::unchecked("manager")),
            Some(Duration::Height(50)),
        )
        .unwrap_err();

        // Manager can update manager
        let info = mock_info("owner2", &[]);
        let _env = mock_env();
        update_config(
            &mut app,
            &staking_addr,
            info,
            Some(Addr::unchecked("owner2")),
            None,
            Some(Duration::Height(50)),
        )
        .unwrap();

        let config = query_config(&app, &staking_addr);
        assert_eq!(config.owner, Some("owner2".to_string()));
        assert_eq!(config.manager, None);

        // Remove owner
        let info = mock_info("owner2", &[]);
        let _env = mock_env();
        update_config(
            &mut app,
            &staking_addr,
            info,
            None,
            None,
            Some(Duration::Height(100)),
        )
        .unwrap();

        // Assert no further updates can be made
        let info = mock_info("owner2", &[]);
        let _env = mock_env();
        let err: ContractError = update_config(
            &mut app,
            &staking_addr,
            info,
            None,
            None,
            Some(Duration::Height(100)),
        )
        .unwrap_err()
        .downcast()
        .unwrap();
        assert_eq!(err, ContractError::Unauthorized {});

        let info = mock_info("manager", &[]);
        let _env = mock_env();
        let err: ContractError = update_config(
            &mut app,
            &staking_addr,
            info,
            None,
            None,
            Some(Duration::Height(100)),
        )
        .unwrap_err()
        .downcast()
        .unwrap();
        assert_eq!(err, ContractError::Unauthorized {})
    }

    #[test]
    fn test_staking() {
        let _deps = mock_dependencies();

        let mut app = mock_app();
        let _token_address = Addr::unchecked("token_address");
        let (staking_addr, cw721_addr) = setup_test_case(&mut app, None);

        // Ensure this is propoerly initialized to zero.
        assert_eq!(
            query_total_power_at_height(&app, &staking_addr, None),
            Uint128::zero()
        );

        let info = mock_info(ADDR1, &[]);
        let _env = mock_env();

        // Successful bond
        mint_nft(
            &mut app,
            &cw721_addr,
            NFT_ID1.to_string(),
            ADDR1.to_string(),
            info.clone(),
        )
        .unwrap();
        mint_nft(
            &mut app,
            &cw721_addr,
            NFT_ID2.to_string(),
            ADDR1.to_string(),
            info.clone(),
        )
        .unwrap();
        stake_nft(
            &mut app,
            &staking_addr,
            &cw721_addr,
            NFT_ID1.to_string(),
            info.clone(),
        )
        .unwrap();

        let start_block = app.block_info().height;

        // Very important that this balances is not reflected until
        // the next block. This protects us from flash loan hostile
        // takeovers.
        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
            Uint128::zero()
        );
        assert_eq!(
            query_voting_power(&app, &staking_addr, ADDR1.to_string(), None),
            Uint128::zero()
        );

        app.update_block(next_block);

        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
            Uint128::from(1u128)
        );
        assert_eq!(
            query_total_staked(&app, &staking_addr),
            Uint128::from(1u128)
        );
        assert_eq!(
            query_total_power_at_height(&app, &staking_addr, None),
            Uint128::new(1)
        );

        assert_eq!(
            get_nft_balance(&app, &cw721_addr, ADDR1.to_string()),
            Uint128::from(1u128)
        );

        assert_eq!(
            query_voting_power(&app, &staking_addr, ADDR1.to_string(), None),
            Uint128::from(1u128)
        );
        // Back in time query.
        assert_eq!(
            query_voting_power(&app, &staking_addr, ADDR1.to_string(), Some(start_block)),
            Uint128::from(0u128)
        );

        // Can't transfer bonded amount
        let msg = cw721::Cw721ExecuteMsg::TransferNft {
            recipient: ADDR2.to_string(),
            token_id: NFT_ID1.to_string(),
        };

        let _err = app
            .borrow_mut()
            .execute_contract(info.sender.clone(), cw721_addr.clone(), &msg, &[])
            .unwrap_err();

        // Sucessful transfer of unbonded amount
        let msg = cw721::Cw721ExecuteMsg::TransferNft {
            recipient: ADDR2.to_string(),
            token_id: NFT_ID2.to_string(),
        };
        let _res = app
            .borrow_mut()
            .execute_contract(info.sender, cw721_addr.clone(), &msg, &[])
            .unwrap();

        assert_eq!(
            get_nft_balance(&app, &cw721_addr, ADDR1),
            Uint128::from(0u128)
        );
        assert_eq!(
            get_nft_balance(&app, &cw721_addr, ADDR2),
            Uint128::from(1u128)
        );

        // Addr 2 successful bond
        let info = mock_info(ADDR2, &[]);
        stake_nft(
            &mut app,
            &staking_addr,
            &cw721_addr,
            NFT_ID2.to_string(),
            info,
        )
        .unwrap();

        app.update_block(next_block);

        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR2),
            Uint128::from(1u128)
        );
        assert_eq!(
            query_total_staked(&app, &staking_addr),
            Uint128::from(2u128)
        );

        // Can't unstake other's staked
        let info = mock_info(ADDR2, &[]);
        let _err =
            unstake_tokens(&mut app, &staking_addr, info, vec![NFT_ID1.to_string()]).unwrap_err();

        // Successful unstake
        let info = mock_info(ADDR2, &[]);
        let _res =
            unstake_tokens(&mut app, &staking_addr, info, vec![NFT_ID2.to_string()]).unwrap();
        app.update_block(next_block);

        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR2),
            Uint128::from(0u128)
        );
        assert_eq!(
            query_total_staked(&app, &staking_addr),
            Uint128::from(1u128)
        );

        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1),
            Uint128::from(1u128)
        );
    }

    #[test]
    fn test_info_query() {
        let mut app = mock_app();
        let unstaking_blocks = 1u64;
        let _token_address = Addr::unchecked("token_address");
        let (staking_addr, _) = setup_test_case(&mut app, Some(Duration::Height(unstaking_blocks)));
        let info: cw_core_interface::voting::InfoResponse = app
            .wrap()
            .query_wasm_smart(staking_addr, &QueryMsg::Info {})
            .unwrap();

        assert_eq!(
            info,
            cw_core_interface::voting::InfoResponse {
                info: cw2::ContractVersion {
                    contract: CONTRACT_NAME.to_string(),
                    version: CONTRACT_VERSION.to_string(),
                }
            }
        )
    }

    #[test]
    fn test_max_claims() {
        let mut app = mock_app();
        let unstaking_blocks = 1u64;
        let _token_address = Addr::unchecked("token_address");
        let (staking_addr, cw721_addr) =
            setup_test_case(&mut app, Some(Duration::Height(unstaking_blocks)));

        let info = mock_info(ADDR1, &[]);

        // Create the max number of claims
        for claim in 0..MAX_CLAIMS {
            mint_nft(
                &mut app,
                &cw721_addr,
                claim.to_string(),
                ADDR1.to_string(),
                info.clone(),
            )
            .unwrap();
            stake_nft(
                &mut app,
                &staking_addr,
                &cw721_addr,
                claim.to_string(),
                info.clone(),
            )
            .unwrap();
        }
        // Unstake all together.
        unstake_tokens(
            &mut app,
            &staking_addr,
            info.clone(),
            (0..MAX_CLAIMS).map(|i| i.to_string()).collect(),
        )
        .unwrap();

        mint_nft(
            &mut app,
            &cw721_addr,
            NFT_ID1.to_string(),
            ADDR1.to_string(),
            info.clone(),
        )
        .unwrap();
        stake_nft(
            &mut app,
            &staking_addr,
            &cw721_addr,
            NFT_ID1.to_string(),
            info.clone(),
        )
        .unwrap();
        mint_nft(
            &mut app,
            &cw721_addr,
            NFT_ID2.to_string(),
            ADDR1.to_string(),
            info.clone(),
        )
        .unwrap();
        stake_nft(
            &mut app,
            &staking_addr,
            &cw721_addr,
            NFT_ID2.to_string(),
            info.clone(),
        )
        .unwrap();

        // Additional unstaking attempts ought to fail.
        unstake_tokens(
            &mut app,
            &staking_addr,
            info.clone(),
            vec![NFT_ID1.to_string()],
        )
        .unwrap_err();

        // Clear out the claims list.
        app.update_block(next_block);
        claim_nfts(&mut app, &staking_addr, info.clone()).unwrap();

        // Unstaking now allowed again.
        unstake_tokens(
            &mut app,
            &staking_addr,
            info.clone(),
            vec![NFT_ID1.to_string()],
        )
        .unwrap();
        app.update_block(next_block);
        unstake_tokens(&mut app, &staking_addr, info, vec![NFT_ID2.to_string()]).unwrap();

        assert_eq!(
            get_nft_balance(&app, &cw721_addr, ADDR1),
            Uint128::from(10u128)
        );
    }

    #[test]
    fn test_unstaking_with_claims() {
        let _deps = mock_dependencies();

        let mut app = mock_app();
        let unstaking_blocks = 10u64;
        let _token_address = Addr::unchecked("token_address");
        let (staking_addr, cw721_addr) =
            setup_test_case(&mut app, Some(Duration::Height(unstaking_blocks)));

        let info = mock_info(ADDR1, &[]);

        // Successful bond
        mint_nft(
            &mut app,
            &cw721_addr,
            NFT_ID1.to_string(),
            ADDR1.to_string(),
            info.clone(),
        )
        .unwrap();
        let _res = stake_nft(
            &mut app,
            &staking_addr,
            &cw721_addr,
            NFT_ID1.to_string(),
            info,
        )
        .unwrap();
        app.update_block(next_block);

        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1),
            Uint128::from(1u128)
        );
        assert_eq!(
            query_total_staked(&app, &staking_addr),
            Uint128::from(1u128)
        );
        assert_eq!(
            get_nft_balance(&app, &cw721_addr, ADDR1),
            Uint128::from(0u128)
        );

        // Unstake
        let info = mock_info(ADDR1, &[]);
        let _res =
            unstake_tokens(&mut app, &staking_addr, info, vec![NFT_ID1.to_string()]).unwrap();
        app.update_block(next_block);

        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1),
            Uint128::from(0u128)
        );
        assert_eq!(
            query_total_staked(&app, &staking_addr),
            Uint128::from(0u128)
        );
        assert_eq!(
            get_nft_balance(&app, &cw721_addr, ADDR1),
            Uint128::from(0u128)
        );

        // Cannot claim when nothing is available
        let info = mock_info(ADDR1, &[]);
        let _err: ContractError = claim_nfts(&mut app, &staking_addr, info)
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(_err, ContractError::NothingToClaim {});

        // Successful claim
        app.update_block(|b| b.height += unstaking_blocks);
        let info = mock_info(ADDR1, &[]);
        claim_nfts(&mut app, &staking_addr, info).unwrap();

        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1),
            Uint128::from(0u128)
        );
        assert_eq!(
            query_total_staked(&app, &staking_addr),
            Uint128::from(0u128)
        );
        assert_eq!(
            get_nft_balance(&app, &cw721_addr, ADDR1),
            Uint128::from(1u128)
        );
    }

    #[test]
    fn multiple_address_staking() {
        let mut app = mock_app();
        let amount1 = Uint128::from(1u128);
        let unstaking_blocks = 10u64;
        let _token_address = Addr::unchecked("token_address");
        let (staking_addr, cw721_addr) =
            setup_test_case(&mut app, Some(Duration::Height(unstaking_blocks)));

        let minter_info = mock_info(ADDR1, &[]);
        // Successful bond
        mint_nft(
            &mut app,
            &cw721_addr,
            NFT_ID1.to_string(),
            ADDR1.to_string(),
            minter_info.clone(),
        )
        .unwrap();
        stake_nft(
            &mut app,
            &staking_addr,
            &cw721_addr,
            NFT_ID1.to_string(),
            minter_info.clone(),
        )
        .unwrap();
        app.update_block(next_block);

        let info = mock_info(ADDR2, &[]);
        // Successful bond
        mint_nft(
            &mut app,
            &cw721_addr,
            NFT_ID2.to_string(),
            ADDR2.to_string(),
            minter_info.clone(),
        )
        .unwrap();
        stake_nft(
            &mut app,
            &staking_addr,
            &cw721_addr,
            NFT_ID2.to_string(),
            info,
        )
        .unwrap();
        app.update_block(next_block);

        let info = mock_info(ADDR3, &[]);
        // Successful bond
        mint_nft(
            &mut app,
            &cw721_addr,
            NFT_ID3.to_string(),
            ADDR3.to_string(),
            minter_info.clone(),
        )
        .unwrap();
        stake_nft(
            &mut app,
            &staking_addr,
            &cw721_addr,
            NFT_ID3.to_string(),
            info,
        )
        .unwrap();
        app.update_block(next_block);

        let info = mock_info(ADDR4, &[]);
        // Successful bond
        mint_nft(
            &mut app,
            &cw721_addr,
            NFT_ID4.to_string(),
            ADDR4.to_string(),
            minter_info,
        )
        .unwrap();
        stake_nft(
            &mut app,
            &staking_addr,
            &cw721_addr,
            NFT_ID4.to_string(),
            info,
        )
        .unwrap();
        app.update_block(next_block);

        assert_eq!(query_staked_balance(&app, &staking_addr, ADDR1), amount1);
        assert_eq!(query_staked_balance(&app, &staking_addr, ADDR2), amount1);
        assert_eq!(query_staked_balance(&app, &staking_addr, ADDR3), amount1);
        assert_eq!(query_staked_balance(&app, &staking_addr, ADDR4), amount1);

        assert_eq!(
            query_total_staked(&app, &staking_addr),
            amount1.checked_mul(Uint128::new(4)).unwrap()
        );

        assert_eq!(get_nft_balance(&app, &cw721_addr, ADDR1), Uint128::zero());
        assert_eq!(get_nft_balance(&app, &cw721_addr, ADDR2), Uint128::zero());
        assert_eq!(get_nft_balance(&app, &cw721_addr, ADDR3), Uint128::zero());
        assert_eq!(get_nft_balance(&app, &cw721_addr, ADDR4), Uint128::zero());
    }

    #[test]
    fn test_simple_unstaking_with_duration() {
        let _deps = mock_dependencies();

        let mut app = mock_app();
        let _token_address = Addr::unchecked("token_address");
        let (staking_addr, cw721_addr) = setup_test_case(&mut app, Some(Duration::Height(1)));

        // Bond Address 1
        let minter_info = mock_info(ADDR1, &[]);
        let _env = mock_env();
        mint_nft(
            &mut app,
            &cw721_addr,
            NFT_ID1.to_string(),
            ADDR1.to_string(),
            minter_info.clone(),
        )
        .unwrap();
        stake_nft(
            &mut app,
            &staking_addr,
            &cw721_addr,
            NFT_ID1.to_string(),
            minter_info.clone(),
        )
        .unwrap();

        // Bond Address 2
        let info = mock_info(ADDR2, &[]);
        let _env = mock_env();
        mint_nft(
            &mut app,
            &cw721_addr,
            NFT_ID2.to_string(),
            ADDR2.to_string(),
            minter_info,
        )
        .unwrap();
        stake_nft(
            &mut app,
            &staking_addr,
            &cw721_addr,
            NFT_ID2.to_string(),
            info,
        )
        .unwrap();
        app.update_block(next_block);
        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
            Uint128::from(1u128)
        );
        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
            Uint128::from(1u128)
        );

        // Unstake Addr1
        let info = mock_info(ADDR1, &[]);
        let _env = mock_env();
        unstake_tokens(&mut app, &staking_addr, info, vec![NFT_ID1.to_string()]).unwrap();

        // Unstake Addr2
        let info = mock_info(ADDR2, &[]);
        let _env = mock_env();
        unstake_tokens(&mut app, &staking_addr, info, vec![NFT_ID2.to_string()]).unwrap();

        app.update_block(next_block);

        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
            Uint128::from(0u128)
        );
        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR2.to_string()),
            Uint128::from(0u128)
        );

        // Claim
        assert_eq!(
            query_nft_claims(&app, &staking_addr, ADDR1),
            vec![NftClaim {
                token_id: NFT_ID1.to_string(),
                release_at: AtHeight(12349)
            }]
        );
        assert_eq!(
            query_nft_claims(&app, &staking_addr, ADDR2),
            vec![NftClaim {
                token_id: NFT_ID2.to_string(),
                release_at: AtHeight(12349)
            }]
        );

        let info = mock_info(ADDR1, &[]);
        claim_nfts(&mut app, &staking_addr, info).unwrap();
        assert_eq!(
            get_nft_balance(&app, &cw721_addr, ADDR1),
            Uint128::from(1u128)
        );

        let info = mock_info(ADDR2, &[]);
        claim_nfts(&mut app, &staking_addr, info).unwrap();
        assert_eq!(
            get_nft_balance(&app, &cw721_addr, ADDR2),
            Uint128::from(1u128)
        );
    }

    #[test]
    fn test_simple_unstaking_without_rewards_with_duration() {
        let _deps = mock_dependencies();

        let mut app = mock_app();
        let _token_address = Addr::unchecked("token_address");
        let (staking_addr, cw721_addr) = setup_test_case(&mut app, Some(Duration::Height(1)));

        // Bond Address 1
        let minter_info = mock_info(ADDR1, &[]);
        let _env = mock_env();
        mint_nft(
            &mut app,
            &cw721_addr,
            NFT_ID1.to_string(),
            ADDR1.to_string(),
            minter_info.clone(),
        )
        .unwrap();
        stake_nft(
            &mut app,
            &staking_addr,
            &cw721_addr,
            NFT_ID1.to_string(),
            minter_info.clone(),
        )
        .unwrap();

        // Bond Address 2
        let info = mock_info(ADDR2, &[]);
        let _env = mock_env();
        mint_nft(
            &mut app,
            &cw721_addr,
            NFT_ID2.to_string(),
            ADDR2.to_string(),
            minter_info,
        )
        .unwrap();
        stake_nft(
            &mut app,
            &staking_addr,
            &cw721_addr,
            NFT_ID2.to_string(),
            info,
        )
        .unwrap();
        app.update_block(next_block);
        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
            Uint128::from(1u128)
        );
        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
            Uint128::from(1u128)
        );

        // Unstake Addr1
        let info = mock_info(ADDR1, &[]);
        let _env = mock_env();
        unstake_tokens(&mut app, &staking_addr, info, vec![NFT_ID1.to_string()]).unwrap();

        // Unstake Addr2
        let info = mock_info(ADDR2, &[]);
        let _env = mock_env();
        unstake_tokens(&mut app, &staking_addr, info, vec![NFT_ID2.to_string()]).unwrap();

        app.update_block(next_block);

        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
            Uint128::from(0u128)
        );
        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR2.to_string()),
            Uint128::from(0u128)
        );

        // Claim
        assert_eq!(
            query_nft_claims(&app, &staking_addr, ADDR1),
            vec![NftClaim {
                token_id: NFT_ID1.to_string(),
                release_at: AtHeight(12349)
            }]
        );
        assert_eq!(
            query_nft_claims(&app, &staking_addr, ADDR2),
            vec![NftClaim {
                token_id: NFT_ID2.to_string(),
                release_at: AtHeight(12349)
            }]
        );

        let info = mock_info(ADDR1, &[]);
        claim_nfts(&mut app, &staking_addr, info).unwrap();
        assert_eq!(
            get_nft_balance(&app, &cw721_addr, ADDR1),
            Uint128::from(1u128)
        );

        let info = mock_info(ADDR2, &[]);
        claim_nfts(&mut app, &staking_addr, info).unwrap();
        assert_eq!(
            get_nft_balance(&app, &cw721_addr, ADDR2),
            Uint128::from(1u128)
        );
    }
}
