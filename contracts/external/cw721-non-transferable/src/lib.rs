use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{CustomMsg, Empty};
use cw4::{
    Member, MemberChangedHookMsg, MemberDiff, MemberListResponse, MemberResponse,
    TotalWeightResponse,
};
pub use cw721_base::{
    entry::{execute as _execute, query as _query},
    Cw721Contract, ExecuteMsg, InstantiateMsg as Cw721BaseInstantiateMsg, MinterResponse, QueryMsg,
};
use cw_controllers::Hooks;
use cw_utils::maybe_addr;

// Hooks to contracts that will receive staking and unstaking messages.
pub const HOOKS: Hooks = Hooks::new("hooks");

mod error;
pub mod state;

pub use crate::error::RolesContractError as ContractError;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw721-soulbound-roles";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cw_serde]
pub struct MetadataExt {
    pub weight: u32,
}

#[cw_serde]
pub enum ExecuteExt {
    /// Add a new hook to be informed of all membership changes.
    /// Must be called by Admin
    AddHook { addr: String },
    /// Remove a hook. Must be called by Admin
    RemoveHook { addr: String },
}
impl CustomMsg for ExecuteExt {}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryExt {
    /// Total weight at a given height
    #[returns(cw4::TotalWeightResponse)]
    TotalWeight { at_height: Option<u64> },
    /// Returns the weight of a certain member
    #[returns(cw4::MemberResponse)]
    Member {
        addr: String,
        at_height: Option<u64>,
    },
    /// Shows all registered hooks.
    #[returns(cw_controllers::HooksResponse)]
    Hooks {},
}
impl CustomMsg for QueryExt {}

pub type Cw721NonTransferableContract<'a> =
    Cw721Contract<'a, MetadataExt, Empty, ExecuteExt, QueryExt>;

#[cfg(not(feature = "library"))]
pub mod entry {
    use super::*;
    use crate::state::{MEMBERS, TOTAL};
    use cosmwasm_std::{
        entry_point, from_binary, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order,
        Response, StdResult, SubMsg, Uint64,
    };
    use cw721::OwnerOfResponse;
    use cw_storage_plus::Bound;

    #[entry_point]
    pub fn instantiate(
        mut deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Cw721BaseInstantiateMsg,
    ) -> Result<Response, ContractError> {
        Cw721NonTransferableContract::default().instantiate(
            deps.branch(),
            env.clone(),
            info,
            msg,
        )?;

        // Initialize total weight to zero
        TOTAL.save(deps.storage, &0, env.block.height)?;

        cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

        Ok(Response::default()
            .add_attribute("contract_name", CONTRACT_NAME)
            .add_attribute("contract_version", CONTRACT_VERSION))
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg<MetadataExt, ExecuteExt>,
    ) -> Result<Response, ContractError> {
        cw_ownable::assert_owner(deps.storage, &info.sender)?;

        match msg {
            ExecuteMsg::Mint {
                token_id,
                owner,
                token_uri,
                extension,
            } => execute_mint(deps, env, info, token_id, owner, token_uri, extension),
            ExecuteMsg::Burn { token_id } => execute_burn(deps, env, info, token_id),
            ExecuteMsg::Extension { msg } => match msg {
                ExecuteExt::AddHook { addr } => execute_add_hook(deps, info, addr),
                ExecuteExt::RemoveHook { addr } => execute_remove_hook(deps, info, addr),
            },
            _ => Cw721NonTransferableContract::default()
                .execute(deps, env, info, msg)
                .map_err(Into::into),
        }
    }

    pub fn execute_mint(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        token_id: String,
        owner: String,
        token_uri: Option<String>,
        extension: MetadataExt,
    ) -> Result<Response, ContractError> {
        // TODO get weight from extension and add to members

        // Update member weights and total
        let mut total = Uint64::from(TOTAL.load(deps.storage)?);
        let mut diff = MemberDiff::new(owner.clone(), None, None);

        MEMBERS.update(
            deps.storage,
            &deps.api.addr_validate(&owner)?,
            env.block.height,
            |old| -> StdResult<_> {
                total = total.checked_sub(Uint64::from(old.unwrap_or_default()))?;
                total = total.checked_add(Uint64::from(extension.weight))?;
                diff = MemberDiff::new(owner.clone(), old, Some(extension.weight.into()));
                Ok(extension.weight.into())
            },
        )?;

        TOTAL.save(deps.storage, &total.u64(), env.block.height)?;

        let diffs = MemberChangedHookMsg { diffs: vec![diff] };

        // Prepare hook messages
        let msgs = HOOKS.prepare_hooks(deps.storage, |h| {
            diffs.clone().into_cosmos_msg(h).map(SubMsg::new)
        })?;

        // Call base mint
        let res = Cw721NonTransferableContract::default().execute(
            deps,
            env,
            info,
            ExecuteMsg::Mint {
                token_id,
                owner,
                token_uri,
                extension,
            },
        )?;

        Ok(res.add_submessages(msgs))
    }

    pub fn execute_burn(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        token_id: String,
    ) -> Result<Response, ContractError> {
        // Lookup the owner of the NFT
        let remove: OwnerOfResponse =
            from_binary(&Cw721NonTransferableContract::default().query(
                deps.as_ref(),
                env.clone(),
                QueryMsg::OwnerOf {
                    token_id: token_id.clone(),
                    include_expired: None,
                },
            )?)?;

        // Update member weights and total
        let mut total = Uint64::from(TOTAL.load(deps.storage)?);
        let mut diff = MemberDiff::new(remove.owner.clone(), None, None);

        let remove_addr = deps.api.addr_validate(&remove.owner)?;
        let old = MEMBERS.may_load(deps.storage, &remove_addr)?;

        // Only process this if they were actually in the list before
        if let Some(weight) = old {
            diff = MemberDiff::new(remove.owner, Some(weight), None);
            total = total.checked_sub(Uint64::from(weight))?;
            MEMBERS.remove(deps.storage, &remove_addr, env.block.height)?;
        }

        TOTAL.save(deps.storage, &total.u64(), env.block.height)?;

        let diffs = MemberChangedHookMsg { diffs: vec![diff] };

        // Prepare hook messages
        let msgs = HOOKS.prepare_hooks(deps.storage, |h| {
            diffs.clone().into_cosmos_msg(h).map(SubMsg::new)
        })?;

        // Call base burn
        let res = Cw721NonTransferableContract::default().execute(
            deps,
            env,
            info,
            ExecuteMsg::Burn { token_id },
        )?;

        Ok(res.add_submessages(msgs))
    }

    pub fn execute_add_hook(
        deps: DepsMut,
        info: MessageInfo,
        addr: String,
    ) -> Result<Response, ContractError> {
        cw_ownable::assert_owner(deps.storage, &info.sender)?;

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
        cw_ownable::assert_owner(deps.storage, &info.sender)?;

        let hook = deps.api.addr_validate(&addr)?;
        HOOKS.remove_hook(deps.storage, hook)?;

        Ok(Response::default()
            .add_attribute("action", "remove_hook")
            .add_attribute("hook", addr))
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg<QueryExt>) -> StdResult<Binary> {
        match msg {
            QueryMsg::Extension { msg } => match msg {
                QueryExt::Hooks {} => to_binary(&HOOKS.query_hooks(deps)?),
                QueryExt::Member { addr, at_height } => {
                    to_binary(&query_member(deps, addr, at_height)?)
                }
                QueryExt::TotalWeight { at_height } => {
                    to_binary(&query_total_weight(deps, at_height)?)
                }
            },
            _ => Cw721NonTransferableContract::default().query(deps, env, msg.into()),
        }
    }

    pub fn query_total_weight(deps: Deps, height: Option<u64>) -> StdResult<TotalWeightResponse> {
        let weight = match height {
            Some(h) => TOTAL.may_load_at_height(deps.storage, h),
            None => TOTAL.may_load(deps.storage),
        }?
        .unwrap_or_default();
        Ok(TotalWeightResponse { weight })
    }

    pub fn query_member(
        deps: Deps,
        addr: String,
        height: Option<u64>,
    ) -> StdResult<MemberResponse> {
        let addr = deps.api.addr_validate(&addr)?;
        let weight = match height {
            Some(h) => MEMBERS.may_load_at_height(deps.storage, &addr, h),
            None => MEMBERS.may_load(deps.storage, &addr),
        }?;
        Ok(MemberResponse { weight })
    }

    // settings for pagination
    const MAX_LIMIT: u32 = 30;
    const DEFAULT_LIMIT: u32 = 10;

    pub fn query_list_members(
        deps: Deps,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<MemberListResponse> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
        let addr = maybe_addr(deps.api, start_after)?;
        let start = addr.as_ref().map(Bound::exclusive);

        let members = MEMBERS
            .range(deps.storage, start, None, Order::Ascending)
            .take(limit)
            .map(|item| {
                item.map(|(addr, weight)| Member {
                    addr: addr.into(),
                    weight,
                })
            })
            .collect::<StdResult<_>>()?;

        Ok(MemberListResponse { members })
    }
}
