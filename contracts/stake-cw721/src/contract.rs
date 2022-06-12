use crate::hooks::{stake_hook_msgs, unstake_hook_msgs};
#[cfg(not(feature = "library"))]
use crate::msg::{
    ExecuteMsg, GetConfigResponse, GetHooksResponse, InstantiateMsg, QueryMsg,
    StakedBalanceAtHeightResponse, StakedValueResponse, TotalStakedAtHeightResponse,
    TotalValueResponse,
};
use crate::state::{
    Config, CONFIG, HOOKS, MAX_CLAIMS, NFT_CLAIMS, REWARD_BALANCE, REWARD_CLAIMS,
    STAKED_NFTS_PER_OWNER, TOTAL_STAKED_NFTS,
};
use crate::ContractError;
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError,
    StdResult, Uint128,
};
use cw2::set_contract_version;
use cw20::Cw20ReceiveMsg;
use cw721::Cw721ReceiveMsg;
use cw721_controllers::NftClaimsResponse;
use cw_controllers::ClaimsResponse;
use cw_utils::Duration;
use std::collections::HashSet;
use std::convert::{From, TryFrom};

const CONTRACT_NAME: &str = "crates.io:stake_cw721";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<Empty>, ContractError> {
    let owner = msg.owner.map(|h| deps.api.addr_validate(&h)).transpose()?;
    let manager = msg
        .manager
        .map(|h| deps.api.addr_validate(&h))
        .transpose()?;
    let reward_token = msg
        .reward_token_address
        .map(|h| deps.api.addr_validate(&h))
        .transpose()?;

    let config = Config {
        owner,
        manager,
        nft_address: deps.api.addr_validate(&msg.nft_address)?,
        reward_token_address: reward_token,
        unstaking_duration: msg.unstaking_duration,
    };
    CONFIG.save(deps.storage, &config)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<Empty>, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => execute_fund(deps, env, info, msg),
        ExecuteMsg::ReceiveNft(msg) => execute_stake(deps, env, info, msg),
        ExecuteMsg::Unstake {
            token_id,
            reward_wallet_address,
        } => execute_unstake(deps, env, info, token_id, reward_wallet_address),
        ExecuteMsg::ClaimNfts {} => execute_claim_nfts(deps, env, info),
        ExecuteMsg::ClaimRewards {} => execute_claim_rewards(deps, env, info),
        ExecuteMsg::UpdateConfig {
            owner,
            manager,
            duration,
        } => execute_update_config(info, deps, owner, manager, duration),
        ExecuteMsg::AddHook { addr } => execute_add_hook(deps, env, info, addr),
        ExecuteMsg::RemoveHook { addr } => execute_remove_hook(deps, env, info, addr),
    }
}

pub fn execute_fund(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if let Some(reward_token_address) = config.reward_token_address {
        if info.sender != reward_token_address {
            return Err(ContractError::InvalidToken {
                received: info.sender,
                expected: reward_token_address,
            });
        }

        let sender = deps.api.addr_validate(&wrapper.sender)?;
        REWARD_BALANCE.update(deps.storage, |old| {
            old.checked_add(wrapper.amount).map_err(StdError::overflow)
        })?;

        Ok(Response::new()
            .add_attribute("action", "fund")
            .add_attribute("from", &sender)
            .add_attribute("amount", wrapper.amount))
    } else {
        Err(ContractError::NothingToFund {})
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
        |total_staked_nfts| -> StdResult<HashSet<String>> {
            let mut updated_total_staked_nfts = total_staked_nfts.unwrap_or_default();
            updated_total_staked_nfts.insert(wrapper.token_id.clone());
            Ok(updated_total_staked_nfts)
        },
    )?;

    let hook_msgs = stake_hook_msgs(deps.storage, sender.clone(), wrapper.token_id.clone())?;
    Ok(Response::new()
        .add_submessages(hook_msgs)
        .add_attribute("action", "stake")
        .add_attribute("from", sender)
        .add_attribute("token_id", wrapper.token_id))
}

pub fn execute_unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
    reward_wallet_address: Option<String>,
) -> Result<Response, ContractError> {
    let validated_reward_wallet = reward_wallet_address
        .map(|h| deps.api.addr_validate(&h))
        .transpose()?;
    let sender = info.sender.clone();
    let config = CONFIG.load(deps.storage)?;
    let sender_staked_nft_collection = STAKED_NFTS_PER_OWNER
        .may_load(deps.storage, &sender)?
        .unwrap_or_default();
    if !sender_staked_nft_collection.contains(&token_id) {
        return Err(ContractError::Unauthorized {});
    }
    let balance = REWARD_BALANCE.load(deps.storage).unwrap_or_default();
    let total_staked_nfts = TOTAL_STAKED_NFTS.load(deps.storage)?;
    let amount_to_claim = balance
        .checked_div(Uint128::from(
            u128::try_from(total_staked_nfts.len()).unwrap(),
        ))
        .map_err(StdError::divide_by_zero)?;

    REWARD_BALANCE.save(
        deps.storage,
        &balance
            .checked_sub(amount_to_claim)
            .map_err(StdError::overflow)?,
    )?;

    STAKED_NFTS_PER_OWNER.update(
        deps.storage,
        &info.sender,
        env.block.height,
        |nft_collection| -> StdResult<HashSet<String>> {
            let mut updated_nft_collection = nft_collection.unwrap_or_default();
            updated_nft_collection.remove(&token_id);
            Ok(updated_nft_collection)
        },
    )?;

    TOTAL_STAKED_NFTS.update(
        deps.storage,
        env.block.height,
        |total_staked_nfts| -> StdResult<HashSet<String>> {
            let mut updated_total_staked_nfts = total_staked_nfts.unwrap_or_default();
            updated_total_staked_nfts.remove(&token_id);
            Ok(updated_total_staked_nfts)
        },
    )?;

    let hook_msgs = unstake_hook_msgs(deps.storage, info.sender.clone(), token_id.clone())?;
    let response = Response::new();
    match config.unstaking_duration {
        None => {
            let cw_transfer_msg = cw721::Cw721ExecuteMsg::TransferNft {
                recipient: info.sender.to_string(),
                token_id: token_id.clone(),
            };

            let wasm_unstake_nft_msg = cosmwasm_std::WasmMsg::Execute {
                contract_addr: config.nft_address.to_string(),
                msg: to_binary(&cw_transfer_msg)?,
                funds: vec![],
            };

            if amount_to_claim.is_zero() || validated_reward_wallet == None {
                return Ok(response
                    .add_message(wasm_unstake_nft_msg)
                    .add_submessages(hook_msgs)
                    .add_attribute("action", "unstake")
                    .add_attribute("from", info.sender)
                    .add_attribute("token_id", token_id)
                    .add_attribute("claim_duration", "None"));
            }

            let cw_send_msg = cw20::Cw20ExecuteMsg::Transfer {
                recipient: validated_reward_wallet.clone().unwrap().to_string(),
                amount: amount_to_claim,
            };

            let wasm_claim_rewards_msg = cosmwasm_std::WasmMsg::Execute {
                contract_addr: config.reward_token_address.unwrap().to_string(),
                msg: to_binary(&cw_send_msg)?,
                funds: vec![],
            };

            Ok(response
                .add_message(wasm_unstake_nft_msg)
                .add_submessages(hook_msgs)
                .add_attribute("action", "unstake")
                .add_attribute("from", info.sender)
                .add_attribute("token_id", token_id)
                .add_attribute("claim_duration", "None")
                .add_message(wasm_claim_rewards_msg)
                .add_attribute("reward_wallet", validated_reward_wallet.unwrap())
                .add_attribute("reward_amount", amount_to_claim))
        }

        Some(duration) => {
            let outstanding_claims = NFT_CLAIMS
                .query_claims(deps.as_ref(), &info.sender)?
                .nft_claims;
            if outstanding_claims.len() >= MAX_CLAIMS as usize {
                return Err(ContractError::TooManyClaims {});
            }

            NFT_CLAIMS.create_nft_claim(
                deps.storage,
                &info.sender,
                token_id.clone(),
                duration.after(&env.block),
            )?;

            if amount_to_claim.is_zero() || validated_reward_wallet == None {
                return Ok(Response::new()
                    .add_attribute("action", "unstake")
                    .add_submessages(hook_msgs)
                    .add_attribute("from", info.sender)
                    .add_attribute("token_id", token_id)
                    .add_attribute("claim_duration", format!("{}", duration)));
            }

            let outstanding_reward_claims = REWARD_CLAIMS
                .query_claims(deps.as_ref(), &info.sender)?
                .claims;
            if outstanding_reward_claims.len() >= MAX_CLAIMS as usize {
                return Err(ContractError::TooManyClaims {});
            }

            REWARD_CLAIMS.create_claim(
                deps.storage,
                &validated_reward_wallet.clone().unwrap(),
                amount_to_claim,
                duration.after(&env.block),
            )?;

            Ok(Response::new()
                .add_attribute("action", "unstake")
                .add_submessages(hook_msgs)
                .add_attribute("from", info.sender)
                .add_attribute("token_id", token_id)
                .add_attribute("claim_duration", format!("{}", duration))
                .add_attribute("reward_wallet", validated_reward_wallet.unwrap())
                .add_attribute("reward_amount", amount_to_claim))
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
    let mut msgs = Vec::new();
    for nft in nfts.iter() {
        let cw_transfer_msg = cw721::Cw721ExecuteMsg::TransferNft {
            recipient: info.sender.to_string(),
            token_id: nft.clone(),
        };

        let wasm_msg = cosmwasm_std::WasmMsg::Execute {
            contract_addr: config.nft_address.to_string(),
            msg: to_binary(&cw_transfer_msg)?,
            funds: vec![],
        };
        msgs.push(wasm_msg);
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "claim_nfts")
        .add_attribute("from", info.sender.clone()))
}

pub fn execute_claim_rewards(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let reward_amount =
        REWARD_CLAIMS.claim_tokens(deps.storage, &info.sender, &_env.block, None)?;

    if reward_amount.is_zero() {
        return Err(ContractError::NothingToClaim {});
    }

    if config.reward_token_address == None {
        return Err(ContractError::NothingToFund {});
    }

    let cw_send_msg = cw20::Cw20ExecuteMsg::Transfer {
        recipient: info.sender.to_string(),
        amount: reward_amount,
    };

    let wasm_msg = cosmwasm_std::WasmMsg::Execute {
        contract_addr: config.reward_token_address.unwrap().to_string(),
        msg: to_binary(&cw_send_msg)?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(wasm_msg)
        .add_attribute("action", "claim_reward")
        .add_attribute("from", info.sender)
        .add_attribute("amount", reward_amount))
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

    Ok(Response::new()
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

    Ok(Response::new()
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

    Ok(Response::new()
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
        QueryMsg::StakedValue { address } => to_binary(&query_staked_value(deps, env, address)?),
        QueryMsg::TotalValue {} => to_binary(&query_total_value(deps, env)?),
        QueryMsg::NftClaims { address } => to_binary(&query_nft_claims(deps, address)?),
        QueryMsg::RewardClaims { address } => to_binary(&query_reward_claims(deps, address)?),
        QueryMsg::GetHooks {} => to_binary(&query_hooks(deps)?),
    }
}

pub fn query_staked_balance_at_height(
    deps: Deps,
    _env: Env,
    address: String,
    height: Option<u64>,
) -> StdResult<StakedBalanceAtHeightResponse> {
    let address = deps.api.addr_validate(&address)?;
    let height = height.unwrap_or(_env.block.height);
    let nft_collection = STAKED_NFTS_PER_OWNER
        .may_load_at_height(deps.storage, &address, height)?
        .unwrap_or_default();

    Ok(StakedBalanceAtHeightResponse {
        balance: Uint128::from(u128::try_from(nft_collection.len()).unwrap()),
        height,
    })
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
        total: Uint128::from(u128::try_from(total_staked_nfts.len()).unwrap()),
        height,
    })
}

pub fn query_staked_value(
    deps: Deps,
    _env: Env,
    address: String,
) -> StdResult<StakedValueResponse> {
    let address = deps.api.addr_validate(&address)?;
    let balance = REWARD_BALANCE.load(deps.storage).unwrap_or_default();
    let staked = STAKED_NFTS_PER_OWNER
        .load(deps.storage, &address)
        .unwrap_or_default();
    let total = TOTAL_STAKED_NFTS.load(deps.storage).unwrap_or_default();
    if balance == Uint128::zero() || staked.is_empty() || total.is_empty() {
        Ok(StakedValueResponse {
            value: Uint128::zero(),
        })
    } else {
        let value = Uint128::from(u128::try_from(staked.len()).unwrap())
            .checked_mul(balance)
            .map_err(StdError::overflow)?
            .checked_div(Uint128::from(u128::try_from(total.len()).unwrap()))
            .map_err(StdError::divide_by_zero)?;
        Ok(StakedValueResponse { value })
    }
}

pub fn query_total_value(deps: Deps, _env: Env) -> StdResult<TotalValueResponse> {
    let balance = REWARD_BALANCE.load(deps.storage).unwrap_or_default();
    Ok(TotalValueResponse { total: balance })
}

pub fn query_config(deps: Deps) -> StdResult<GetConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(GetConfigResponse {
        owner: config.owner.map(|a| a.to_string()),
        manager: config.manager.map(|a| a.to_string()),
        unstaking_duration: config.unstaking_duration,
        nft_address: config.nft_address.to_string(),
        reward_token_address: Some(config.reward_token_address.unwrap().to_string()),
    })
}

pub fn query_nft_claims(deps: Deps, address: String) -> StdResult<NftClaimsResponse> {
    NFT_CLAIMS.query_claims(deps, &deps.api.addr_validate(&address)?)
}

pub fn query_reward_claims(deps: Deps, address: String) -> StdResult<ClaimsResponse> {
    REWARD_CLAIMS.query_claims(deps, &deps.api.addr_validate(&address)?)
}

pub fn query_hooks(deps: Deps) -> StdResult<GetHooksResponse> {
    Ok(GetHooksResponse {
        hooks: HOOKS.query_hooks(deps)?.hooks,
    })
}

#[cfg(test)]
mod tests {

    use crate::msg::{
        ExecuteMsg, GetConfigResponse, QueryMsg, StakedBalanceAtHeightResponse,
        TotalStakedAtHeightResponse,
    };
    use crate::state::MAX_CLAIMS;
    use crate::ContractError;
    use anyhow::Result as AnyResult;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{to_binary, Addr, Empty, MessageInfo, Uint128};
    use cw20::Cw20Coin;
    use cw721_controllers::NftClaim;
    use cw_controllers::{Claim, ClaimsResponse};
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

    pub fn contract_staking() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        Box::new(contract)
    }

    pub fn contract_cw721() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw721_base::entry::execute,
            cw721_base::entry::instantiate,
            cw721_base::entry::query,
        );
        Box::new(contract)
    }

    pub fn contract_cw20() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw20_base::contract::execute,
            cw20_base::contract::instantiate,
            cw20_base::contract::query,
        );
        Box::new(contract)
    }

    fn mock_app() -> App {
        App::default()
    }

    fn get_reward_token_balance<T: Into<String>, U: Into<String>>(
        app: &App,
        contract_addr: T,
        address: U,
    ) -> Uint128 {
        let msg = cw20::Cw20QueryMsg::Balance {
            address: address.into(),
        };
        let result: cw20::BalanceResponse =
            app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
        result.balance
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

    fn instantiate_cw20(app: &mut App, initial_balances: Vec<Cw20Coin>) -> Addr {
        let cw20_id = app.store_code(contract_cw20());
        let msg = cw20_base::msg::InstantiateMsg {
            name: String::from("Test"),
            symbol: String::from("Test"),
            decimals: 6,
            initial_balances,
            mint: None,
            marketing: None,
        };

        app.instantiate_contract(cw20_id, Addr::unchecked(ADDR1), &msg, &[], "cw20", None)
            .unwrap()
    }

    fn instantiate_cw721(app: &mut App) -> Addr {
        let cw721_id = app.store_code(contract_cw721());
        let msg = cw721_base::msg::InstantiateMsg {
            name: String::from("Test"),
            symbol: String::from("Test"),
            minter: Addr::unchecked(ADDR1).to_string(),
        };

        app.instantiate_contract(cw721_id, Addr::unchecked(ADDR1), &msg, &[], "cw721", None)
            .unwrap()
    }

    fn instantiate_staking(
        app: &mut App,
        cw20: Addr,
        cw721: Addr,
        unstaking_duration: Option<Duration>,
    ) -> Addr {
        let staking_code_id = app.store_code(contract_staking());
        let msg = crate::msg::InstantiateMsg {
            owner: Some("owner".to_string()),
            manager: Some("manager".to_string()),
            nft_address: cw721.to_string(),
            reward_token_address: Some(cw20.to_string()),
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

    fn setup_test_case(
        app: &mut App,
        initial_balances: Vec<Cw20Coin>,
        unstaking_duration: Option<Duration>,
    ) -> (Addr, Addr, Addr) {
        // Instantiate cw20 contract
        let cw20_addr = instantiate_cw20(app, initial_balances);
        // Instantiate cw721 contract
        let cw721_addr = instantiate_cw721(app);
        app.update_block(next_block);
        // Instantiate staking contract
        let staking_addr = instantiate_staking(
            app,
            cw20_addr.clone(),
            cw721_addr.clone(),
            unstaking_duration,
        );
        app.update_block(next_block);
        (staking_addr, cw20_addr, cw721_addr)
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

    fn query_reward_claims<T: Into<String>, U: Into<String>>(
        app: &App,
        contract_addr: T,
        address: U,
    ) -> Vec<Claim> {
        let msg = QueryMsg::RewardClaims {
            address: address.into(),
        };
        let result: ClaimsResponse = app.wrap().query_wasm_smart(contract_addr, &msg).unwrap();
        result.claims
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
        token_id: String,
        reward_wallet_address: Option<String>,
    ) -> AnyResult<AppResponse> {
        let msg = ExecuteMsg::Unstake {
            token_id,
            reward_wallet_address,
        };
        app.execute_contract(info.sender, staking_addr.clone(), &msg, &[])
    }

    fn claim_nfts(app: &mut App, staking_addr: &Addr, info: MessageInfo) -> AnyResult<AppResponse> {
        let msg = ExecuteMsg::ClaimNfts {};
        app.execute_contract(info.sender, staking_addr.clone(), &msg, &[])
    }

    fn claim_rewards(
        app: &mut App,
        staking_addr: &Addr,
        info: MessageInfo,
    ) -> AnyResult<AppResponse> {
        let msg = ExecuteMsg::ClaimRewards {};
        app.execute_contract(info.sender, staking_addr.clone(), &msg, &[])
    }

    #[test]
    fn test_update_config() {
        let _deps = mock_dependencies();

        let mut app = mock_app();
        let initial_balances = vec![];
        let (staking_addr, _cw20_addr, _cw721_addr) =
            setup_test_case(&mut app, initial_balances, None);

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
        let initial_balances = vec![];
        let (staking_addr, cw20_addr, cw721_addr) =
            setup_test_case(&mut app, initial_balances, None);

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

        // Very important that this balances is not reflected until
        // the next block. This protects us from flash loan hostile
        // takeovers.
        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1.to_string()),
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
            get_nft_balance(&app, &cw721_addr, ADDR1.to_string()),
            Uint128::from(1u128)
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
        assert_eq!(
            get_reward_token_balance(&app, &cw20_addr, ADDR2),
            Uint128::zero()
        );

        // Can't unstake other's staked
        let info = mock_info(ADDR2, &[]);
        let _err =
            unstake_tokens(&mut app, &staking_addr, info, NFT_ID1.to_string(), None).unwrap_err();

        // Successful unstake
        let info = mock_info(ADDR2, &[]);
        let _res =
            unstake_tokens(&mut app, &staking_addr, info, NFT_ID2.to_string(), None).unwrap();
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
            get_reward_token_balance(&app, &cw20_addr, ADDR2),
            Uint128::from(0u128)
        );

        assert_eq!(
            query_staked_balance(&app, &staking_addr, ADDR1),
            Uint128::from(1u128)
        );
        assert_eq!(
            get_reward_token_balance(&app, &cw20_addr, ADDR1),
            Uint128::from(0u128)
        );
    }

    #[test]
    fn test_max_claims() {
        let mut app = mock_app();
        let amount1 = Uint128::from(MAX_CLAIMS + 1);
        let unstaking_blocks = 1u64;
        let _token_address = Addr::unchecked("token_address");
        let initial_balances = vec![Cw20Coin {
            address: ADDR1.to_string(),
            amount: amount1,
        }];
        let (staking_addr, _, cw721_addr) = setup_test_case(
            &mut app,
            initial_balances,
            Some(Duration::Height(unstaking_blocks)),
        );

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
            unstake_tokens(
                &mut app,
                &staking_addr,
                info.clone(),
                claim.to_string(),
                None,
            )
            .unwrap();
        }

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
            NFT_ID1.to_string(),
            None,
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
            NFT_ID1.to_string(),
            None,
        )
        .unwrap();
        app.update_block(next_block);
        unstake_tokens(&mut app, &staking_addr, info, NFT_ID2.to_string(), None).unwrap();

        assert_eq!(
            get_nft_balance(&app, &cw721_addr, ADDR1),
            Uint128::from(10u128)
        );
    }

    #[test]
    fn test_unstaking_with_claims() {
        let _deps = mock_dependencies();

        let mut app = mock_app();
        let amount1 = Uint128::from(100u128);
        let unstaking_blocks = 10u64;
        let _token_address = Addr::unchecked("token_address");
        let initial_balances = vec![Cw20Coin {
            address: ADDR1.to_string(),
            amount: amount1,
        }];
        let (staking_addr, cw20_addr, cw721_addr) = setup_test_case(
            &mut app,
            initial_balances,
            Some(Duration::Height(unstaking_blocks)),
        );

        let info = mock_info(ADDR1, &[]);

        let msg = cw20::Cw20ExecuteMsg::Send {
            contract: staking_addr.to_string(),
            amount: amount1,
            msg: to_binary("Test").unwrap(),
        };
        app.borrow_mut()
            .execute_contract(Addr::unchecked(ADDR1), cw20_addr.clone(), &msg, &[])
            .unwrap();

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
        assert_eq!(
            get_reward_token_balance(&app, &cw20_addr, ADDR1),
            Uint128::from(0u128)
        );

        // Unstake
        let info = mock_info(ADDR1, &[]);
        let _res = unstake_tokens(
            &mut app,
            &staking_addr,
            info,
            NFT_ID1.to_string(),
            Some(Addr::unchecked(ADDR1).to_string()),
        )
        .unwrap();
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
        assert_eq!(
            get_reward_token_balance(&app, &cw20_addr, ADDR1),
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
        claim_nfts(&mut app, &staking_addr, info.clone()).unwrap();
        claim_rewards(&mut app, &staking_addr, info).unwrap();
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
        assert_eq!(
            get_reward_token_balance(&app, &cw20_addr, ADDR1),
            Uint128::from(100u128)
        );
    }

    #[test]
    fn multiple_address_staking() {
        let initial_balances = vec![];
        let mut app = mock_app();
        let amount1 = Uint128::from(1u128);
        let unstaking_blocks = 10u64;
        let _token_address = Addr::unchecked("token_address");
        let (staking_addr, _, cw721_addr) = setup_test_case(
            &mut app,
            initial_balances,
            Some(Duration::Height(unstaking_blocks)),
        );

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
        let amount1 = Uint128::from(100u128);
        let _token_address = Addr::unchecked("token_address");
        let initial_balances = vec![Cw20Coin {
            address: ADDR1.to_string(),
            amount: amount1,
        }];
        let (staking_addr, cw20_addr, cw721_addr) =
            setup_test_case(&mut app, initial_balances, Some(Duration::Height(1)));

        // Fund Contract
        let msg = cw20::Cw20ExecuteMsg::Send {
            contract: staking_addr.to_string(),
            amount: amount1,
            msg: to_binary("Test").unwrap(),
        };
        app.borrow_mut()
            .execute_contract(Addr::unchecked(ADDR1), cw20_addr.clone(), &msg, &[])
            .unwrap();

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
        assert_eq!(
            get_reward_token_balance(&app, &cw20_addr, ADDR1),
            Uint128::from(0u128)
        );
        assert_eq!(
            get_reward_token_balance(&app, &cw20_addr, ADDR2),
            Uint128::from(0u128)
        );

        // Unstake Addr1
        let info = mock_info(ADDR1, &[]);
        let _env = mock_env();
        let reward_amount1 = Uint128::new(50);
        unstake_tokens(
            &mut app,
            &staking_addr,
            info,
            NFT_ID1.to_string(),
            Some(Addr::unchecked(ADDR1).to_string()),
        )
        .unwrap();

        // Unstake Addr2
        let info = mock_info(ADDR2, &[]);
        let _env = mock_env();
        let reward_amount2 = Uint128::new(50);
        unstake_tokens(
            &mut app,
            &staking_addr,
            info,
            NFT_ID2.to_string(),
            Some(Addr::unchecked(ADDR2).to_string()),
        )
        .unwrap();

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
        assert_eq!(
            query_reward_claims(&app, &staking_addr, ADDR1),
            vec![Claim {
                amount: reward_amount1,
                release_at: AtHeight(12349)
            }]
        );
        assert_eq!(
            query_reward_claims(&app, &staking_addr, ADDR2),
            vec![Claim {
                amount: reward_amount2,
                release_at: AtHeight(12349)
            }]
        );

        let info = mock_info(ADDR1, &[]);
        claim_nfts(&mut app, &staking_addr, info.clone()).unwrap();
        assert_eq!(
            get_nft_balance(&app, &cw721_addr, ADDR1),
            Uint128::from(1u128)
        );

        claim_rewards(&mut app, &staking_addr, info).unwrap();
        assert_eq!(
            get_reward_token_balance(&app, &cw20_addr, ADDR1),
            reward_amount1
        );

        let info = mock_info(ADDR2, &[]);
        claim_nfts(&mut app, &staking_addr, info.clone()).unwrap();
        assert_eq!(
            get_nft_balance(&app, &cw721_addr, ADDR2),
            Uint128::from(1u128)
        );

        claim_rewards(&mut app, &staking_addr, info).unwrap();
        assert_eq!(
            get_reward_token_balance(&app, &cw20_addr, ADDR1),
            reward_amount2
        );
    }

    #[test]
    fn test_simple_unstaking_without_rewards_with_duration() {
        let _deps = mock_dependencies();

        let mut app = mock_app();
        let _token_address = Addr::unchecked("token_address");
        let initial_balances = vec![];
        let (staking_addr, cw20_addr, cw721_addr) =
            setup_test_case(&mut app, initial_balances, Some(Duration::Height(1)));

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
        assert_eq!(
            get_reward_token_balance(&app, &cw20_addr, ADDR1),
            Uint128::from(0u128)
        );
        assert_eq!(
            get_reward_token_balance(&app, &cw20_addr, ADDR2),
            Uint128::from(0u128)
        );

        // Unstake Addr1
        let info = mock_info(ADDR1, &[]);
        let _env = mock_env();
        unstake_tokens(
            &mut app,
            &staking_addr,
            info,
            NFT_ID1.to_string(),
            Some(Addr::unchecked(ADDR1).to_string()),
        )
        .unwrap();

        // Unstake Addr2
        let info = mock_info(ADDR2, &[]);
        let _env = mock_env();
        unstake_tokens(
            &mut app,
            &staking_addr,
            info,
            NFT_ID2.to_string(),
            Some(Addr::unchecked(ADDR2).to_string()),
        )
        .unwrap();

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
        assert_eq!(query_reward_claims(&app, &staking_addr, ADDR1), vec![]);
        assert_eq!(query_reward_claims(&app, &staking_addr, ADDR2), vec![]);

        let info = mock_info(ADDR1, &[]);
        claim_nfts(&mut app, &staking_addr, info.clone()).unwrap();
        assert_eq!(
            get_nft_balance(&app, &cw721_addr, ADDR1),
            Uint128::from(1u128)
        );

        claim_rewards(&mut app, &staking_addr, info).unwrap_err();
        assert_eq!(
            get_reward_token_balance(&app, &cw20_addr, ADDR1),
            Uint128::from(0u128)
        );

        let info = mock_info(ADDR2, &[]);
        claim_nfts(&mut app, &staking_addr, info.clone()).unwrap();
        assert_eq!(
            get_nft_balance(&app, &cw721_addr, ADDR2),
            Uint128::from(1u128)
        );

        claim_rewards(&mut app, &staking_addr, info).unwrap_err();
        assert_eq!(
            get_reward_token_balance(&app, &cw20_addr, ADDR1),
            Uint128::from(0u128)
        );
    }
}
