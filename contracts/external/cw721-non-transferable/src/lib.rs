pub use crate::msg::{InstantiateMsg, QueryMsg};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Addr, Empty};
use cw4::{
    Member, MemberChangedHookMsg, MemberDiff, MemberListResponse, MemberResponse,
    TotalWeightResponse,
};
pub use cw721_base::{
    entry::{execute as _execute, query as _query},
    ContractError, Cw721Contract, ExecuteMsg, InstantiateMsg as Cw721BaseInstantiateMsg,
    MinterResponse,
};
use cw_controllers::Hooks;

// Hooks to contracts that will receive staking and unstaking messages.
pub const HOOKS: Hooks = Hooks::new("hooks");

pub mod state;

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

pub type Cw721NonTransferableContract<'a> =
    Cw721Contract<'a, MetadataExt, Empty, ExecuteExt, QueryExt>;

#[cfg(not(feature = "library"))]
pub mod entry {
    use super::*;
    use crate::state::TOTAL;
    use cosmwasm_std::{
        entry_point, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    };

    #[entry_point]
    pub fn instantiate(
        mut deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        Cw721NonTransferableContract::default().instantiate(deps.branch(), env, info, msg)?;

        // Initialize total weight to zero
        TOTAL.save(deps.storage, 0);

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
        msg: ExecuteMsg<MetadataExtension, ExecuteExt>,
    ) -> Result<Response, cw721_base::ContractError> {
        let owner = cw_ownable::assert_owner(deps.storage, &info.sender)?;
        match owner {
            Some(admin) => {
                if admin == info.sender {
                    match msg {
                        ExecuteMsg::Mint {
                            token_id,
                            owner,
                            token_uri,
                            extension,
                        } => execute_mint(deps, env, info, token_id, owner, token_uri, extension),
                        ExecuteMsg::Burn { id } => execute_burn(deps, env, info, id),
                        ExecuteMsg::Extension { msg } => match msg {
                            ExecuteExt::AddHook { addr } => execute_add_hook(deps, info, addr),
                            ExecuteExt::RemoveHook { addr } => {
                                execute_remove_hook(deps, info, addr)
                            }
                        },
                        _ => _execute(deps, env, info, msg),
                    }
                } else {
                    Err(ContractError::Ownership(
                        cw721_base::OwnershipError::NotOwner,
                    ))
                }
            }
            // TODO Error should be "no owner", this contract is immutable
            None => Err(ContractError::Ownership(
                cw721_base::OwnershipError::NotOwner,
            )),
        }
    }

    pub fn execute_mint(
        deps: Deps,
        env: Env,
        info: MessageInfo,
        token_id: String,
        owner: String,
        token_uri: String,
        extension: MetadataExt,
    ) -> Result<Response, ContractError> {
        // Update member weights and total
        let mut total = Uint64::from(TOTAL.load(deps.storage)?);
        let mut diff: MemberDiff;

        MEMBERS.update(deps.storage, &add_addr, height, |old| -> StdResult<_> {
            total = total.checked_sub(Uint64::from(old.unwrap_or_default()))?;
            total = total.checked_add(Uint64::from(add.weight))?;
            diff = MemberDiff::new(add.addr, old, Some(add.weight));
            Ok(add.weight)
        })?;

        TOTAL.save(deps.storage, &total.u64(), height)?;

        let diffs = MembershipChangedHookMsg {
            diffs: vec![diff]
        }

        // Prepare hook messages
        let msgs = HOOKS.prepare_hooks(deps.storage, |h| {
            diffs.clone().into_cosmos_msg(h).map(SubMsg::new)
        })?;

        // Call base mint
        let mut res =_execute(deps, info, msg ExecuteMsg::Mint{
            token_id,
            owner,
            token_uri,
            extension
        })?;

        Ok(res.add_messages(msgs))
    }

    pub fn execute_burn(
        deps: Deps,
        env: Env,
        info: MessageInfo,
        id: String,
    ) -> Result<Response, ContractError> {
        // Update member weights and total
        let mut total = Uint64::from(TOTAL.load(deps.storage)?);
        let mut diff: MemberDiff;

        let remove_addr = deps.api.addr_validate(&remove)?;
        let old = MEMBERS.may_load(deps.storage, &remove_addr)?;

        // Only process this if they were actually in the list before
        if let Some(weight) = old {
            diff = MemberDiff::new(remove, Some(weight), None);
            total = total.checked_sub(Uint64::from(weight))?;
            MEMBERS.remove(deps.storage, &remove_addr, height)?;
        }

        TOTAL.save(deps.storage, &total.u64(), height)?;

        let diffs = MembershipChangedHookMsg {
            diffs: vec![diff]
        }

        // Prepare hook messages
        let msgs = HOOKS.prepare_hooks(deps.storage, |h| {
            diffs.clone().into_cosmos_msg(h).map(SubMsg::new)
        })?;

        // Call base burn
        let mut res =_execute(deps, env, info, ExecuteMsg::Burn {id})

            Ok(res.add_messages(msgs))
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

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
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
            _ => _query(deps, env, msg.into()),
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
