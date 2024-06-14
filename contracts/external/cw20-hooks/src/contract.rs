#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Reply, Response,
    StdError, StdResult, SubMsg, Uint128, WasmMsg,
};

use cw2::{ensure_from_older_version, get_contract_version, set_contract_version};
use cw20::{Cw20ReceiveMsg, EmbeddedLogo, Logo, LogoInfo};
use cw20_base::allowances::deduct_allowance;
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;
use cw20_base::state::{
    MinterData, ALLOWANCES, ALLOWANCES_SPENDER, BALANCES, LOGO, MARKETING_INFO, TOKEN_INFO,
};

use crate::error::ContractError;
use crate::hooks::{Cw20HookExecuteMsg, Cw20HookMsg};
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{CAP, HOOKS};

// Version info, for migration info
const CONTRACT_NAME: &str = "crates.io:cw20-hooks";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const HOOK_REPLY_ID: u64 = 0;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // Call cw20-base instantiate to set everything up.
    cw20_base::contract::instantiate(
        deps.branch(),
        env,
        info,
        Cw20InstantiateMsg {
            name: msg.name,
            symbol: msg.symbol,
            decimals: msg.decimals,
            initial_balances: msg.initial_balances,
            mint: msg.mint.clone(),
            marketing: msg.marketing,
        },
    )?;

    // cw20-base::contract::instantiate sets the contract version, so overwrite
    // it here.
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Initialize owner.
    cw_ownable::initialize_owner(deps.storage, deps.api, msg.owner.as_deref())?;

    // Initialize cap.
    CAP.save(deps.storage, &msg.mint.and_then(|m| m.cap))?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // NEW VARIANTS FOR CW20-HOOKS
        ExecuteMsg::AddHook { addr } => execute_add_hook(deps, info, addr),
        ExecuteMsg::RemoveHook { addr } => execute_remove_hook(deps, info, addr),
        ExecuteMsg::UpdateOwnership(action) => execute_update_owner(deps, info, env, action),

        // COPIED FROM CW20-BASE
        ExecuteMsg::Transfer { recipient, amount } => {
            Ok(execute_transfer(deps, env, info, recipient, amount)?)
        }
        ExecuteMsg::Burn { amount } => {
            Ok(cw20_base::contract::execute_burn(deps, env, info, amount)?)
        }
        ExecuteMsg::Send {
            contract,
            amount,
            msg,
        } => Ok(execute_send(deps, env, info, contract, amount, msg)?),
        ExecuteMsg::Mint { recipient, amount } => Ok(cw20_base::contract::execute_mint(
            deps, env, info, recipient, amount,
        )?),
        ExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(cw20_base::allowances::execute_increase_allowance(
            deps, env, info, spender, amount, expires,
        )?),
        ExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => Ok(cw20_base::allowances::execute_decrease_allowance(
            deps, env, info, spender, amount, expires,
        )?),
        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => Ok(execute_transfer_from(
            deps, env, info, owner, recipient, amount,
        )?),
        ExecuteMsg::BurnFrom { owner, amount } => Ok(cw20_base::allowances::execute_burn_from(
            deps, env, info, owner, amount,
        )?),
        ExecuteMsg::SendFrom {
            owner,
            contract,
            amount,
            msg,
        } => Ok(execute_send_from(
            deps, env, info, owner, contract, amount, msg,
        )?),
        ExecuteMsg::UpdateMarketing {
            project,
            description,
            marketing,
        } => Ok(execute_update_marketing(
            deps,
            env,
            info,
            project,
            description,
            marketing,
        )?),
        ExecuteMsg::UploadLogo(logo) => Ok(execute_upload_logo(deps, env, info, logo)?),
        ExecuteMsg::UpdateMinter { new_minter } => {
            execute_update_minter(deps, env, info, new_minter)
        }
    }
}

pub fn execute_update_owner(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    action: cw_ownable::Action,
) -> Result<Response, ContractError> {
    let ownership = cw_ownable::update_ownership(deps, &env.block, &info.sender, action)?;
    Ok(Response::default().add_attributes(ownership.into_attributes()))
}

pub fn execute_add_hook(
    deps: DepsMut,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    // Check that the sender is the owner.
    let ownership = cw_ownable::get_ownership(deps.storage)?;
    if ownership.owner.map_or(true, |owner| owner != info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let hook = deps.api.addr_validate(&addr)?;
    HOOKS.add_hook(deps.storage, hook.clone())?;

    Ok(Response::new()
        .add_attribute("action", "add_hook")
        .add_attribute("hook", hook))
}

pub fn execute_remove_hook(
    deps: DepsMut,
    info: MessageInfo,
    addr: String,
) -> Result<Response, ContractError> {
    // Check that the sender is the owner.
    let ownership = cw_ownable::get_ownership(deps.storage)?;
    if ownership.owner.map_or(true, |owner| owner != info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let hook = deps.api.addr_validate(&addr)?;
    HOOKS.remove_hook(deps.storage, hook.clone())?;

    Ok(Response::new()
        .add_attribute("action", "remove_hook")
        .add_attribute("hook", hook))
}

// Copied from cw20-base and modified to add hooks.
pub fn execute_transfer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let rcpt_addr = deps.api.addr_validate(&recipient)?;

    BALANCES.update(
        deps.storage,
        &info.sender,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    BALANCES.update(
        deps.storage,
        &rcpt_addr,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;

    // Add hooks.
    let hooks = HOOKS.prepare_hooks(deps.storage, |h| {
        Ok(SubMsg::reply_on_error(
            WasmMsg::Execute {
                contract_addr: h.to_string(),
                msg: to_json_binary(&Cw20HookExecuteMsg::Cw20Hook(Cw20HookMsg::Transfer {
                    sender: info.sender.to_string(),
                    recipient: recipient.clone(),
                    amount,
                }))?,
                funds: vec![],
            },
            HOOK_REPLY_ID,
        ))
    })?;

    let res = Response::new()
        .add_attribute("action", "transfer")
        .add_attribute("from", info.sender)
        .add_attribute("to", recipient)
        .add_attribute("amount", amount)
        .add_submessages(hooks);
    Ok(res)
}

// Copied from cw20-base and modified to add hooks.
pub fn execute_send(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    contract: String,
    amount: Uint128,
    msg: Binary,
) -> Result<Response, ContractError> {
    let rcpt_addr = deps.api.addr_validate(&contract)?;

    // move the tokens to the contract
    BALANCES.update(
        deps.storage,
        &info.sender,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    BALANCES.update(
        deps.storage,
        &rcpt_addr,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;

    // Add hooks.
    let hooks = HOOKS.prepare_hooks(deps.storage, |h| {
        Ok(SubMsg::reply_on_error(
            WasmMsg::Execute {
                contract_addr: h.to_string(),
                msg: to_json_binary(&Cw20HookExecuteMsg::Cw20Hook(Cw20HookMsg::Send {
                    sender: info.sender.to_string(),
                    contract: contract.clone(),
                    amount,
                    msg: msg.clone(),
                }))?,
                funds: vec![],
            },
            HOOK_REPLY_ID,
        ))
    })?;

    let res = Response::new()
        .add_attribute("action", "send")
        .add_attribute("from", &info.sender)
        .add_attribute("to", &contract)
        .add_attribute("amount", amount)
        .add_message(
            Cw20ReceiveMsg {
                sender: info.sender.into(),
                amount,
                msg,
            }
            .into_cosmos_msg(contract)?,
        )
        .add_submessages(hooks);
    Ok(res)
}

// Copied from cw20-base and modified to add hooks.
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

    // deduct allowance before doing anything else have enough allowance
    deduct_allowance(deps.storage, &owner_addr, &info.sender, &env.block, amount)?;

    BALANCES.update(
        deps.storage,
        &owner_addr,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    BALANCES.update(
        deps.storage,
        &rcpt_addr,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;

    // Add hooks.
    let hooks = HOOKS.prepare_hooks(deps.storage, |h| {
        Ok(SubMsg::reply_on_error(
            WasmMsg::Execute {
                contract_addr: h.to_string(),
                msg: to_json_binary(&Cw20HookExecuteMsg::Cw20Hook(Cw20HookMsg::Transfer {
                    sender: info.sender.to_string(),
                    recipient: recipient.clone(),
                    amount,
                }))?,
                funds: vec![],
            },
            HOOK_REPLY_ID,
        ))
    })?;

    let res = Response::new()
        .add_attributes(vec![
            attr("action", "transfer_from"),
            attr("from", owner),
            attr("to", recipient),
            attr("by", info.sender),
            attr("amount", amount),
        ])
        .add_submessages(hooks);
    Ok(res)
}

// Copied from cw20-base and modified to add hooks.
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

    // deduct allowance before doing anything else have enough allowance
    deduct_allowance(deps.storage, &owner_addr, &info.sender, &env.block, amount)?;

    // move the tokens to the contract
    BALANCES.update(
        deps.storage,
        &owner_addr,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    BALANCES.update(
        deps.storage,
        &rcpt_addr,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;

    let attrs = vec![
        attr("action", "send_from"),
        attr("from", &owner),
        attr("to", &contract),
        attr("by", &info.sender),
        attr("amount", amount),
    ];

    // Add hooks.
    let hooks = HOOKS.prepare_hooks(deps.storage, |h| {
        Ok(SubMsg::reply_on_error(
            WasmMsg::Execute {
                contract_addr: h.to_string(),
                msg: to_json_binary(&Cw20HookExecuteMsg::Cw20Hook(Cw20HookMsg::Send {
                    sender: info.sender.to_string(),
                    contract: contract.clone(),
                    amount,
                    msg: msg.clone(),
                }))?,
                funds: vec![],
            },
            HOOK_REPLY_ID,
        ))
    })?;

    // create a send message
    let msg = Cw20ReceiveMsg {
        sender: info.sender.into(),
        amount,
        msg,
    }
    .into_cosmos_msg(contract)?;

    let res = Response::new()
        .add_message(msg)
        .add_attributes(attrs)
        .add_submessages(hooks);
    Ok(res)
}

// Copied from cw20-base and modified to allow only the owner to modify minter.
pub fn execute_update_minter(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_minter: Option<String>,
) -> Result<Response, ContractError> {
    // Check that the sender is the owner.
    let ownership = cw_ownable::get_ownership(deps.storage)?;
    if ownership.owner.map_or(true, |owner| owner != info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let mut config = TOKEN_INFO.load(deps.storage)?;

    let minter_data: Option<MinterData> = new_minter
        .map(|new_minter| deps.api.addr_validate(&new_minter))
        .transpose()?
        .map(|minter| {
            Ok::<MinterData, StdError>(MinterData {
                minter,
                // Load cap from storage item.
                cap: CAP.load(deps.storage)?,
            })
        })
        .transpose()?;

    config.mint = minter_data;

    TOKEN_INFO.save(deps.storage, &config)?;

    Ok(Response::default()
        .add_attribute("action", "update_minter")
        .add_attribute(
            "new_minter",
            config
                .mint
                .map(|m| m.minter.into_string())
                .unwrap_or_else(|| "None".to_string()),
        ))
}

// Copied from cw20-base and modified to allow owner to modify marketing info as
// well as the marketing address.
pub fn execute_update_marketing(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    project: Option<String>,
    description: Option<String>,
    marketing: Option<String>,
) -> Result<Response, ContractError> {
    let mut marketing_info = MARKETING_INFO.may_load(deps.storage)?.unwrap_or_default();

    // Check sender is owner or marketer.
    let ownership = cw_ownable::get_ownership(deps.storage)?;
    let is_owner = ownership.owner.map_or(false, |owner| owner == info.sender);
    let is_marketer = marketing_info
        .marketing
        .as_ref()
        .map_or(false, |m| *m == info.sender);
    if !is_owner && !is_marketer {
        return Err(ContractError::Unauthorized {});
    }

    match project {
        Some(empty) if empty.trim().is_empty() => marketing_info.project = None,
        Some(project) => marketing_info.project = Some(project),
        None => (),
    }

    match description {
        Some(empty) if empty.trim().is_empty() => marketing_info.description = None,
        Some(description) => marketing_info.description = Some(description),
        None => (),
    }

    match marketing {
        Some(empty) if empty.trim().is_empty() => marketing_info.marketing = None,
        Some(marketing) => marketing_info.marketing = Some(deps.api.addr_validate(&marketing)?),
        None => (),
    }

    if marketing_info.project.is_none()
        && marketing_info.description.is_none()
        && marketing_info.marketing.is_none()
        && marketing_info.logo.is_none()
    {
        MARKETING_INFO.remove(deps.storage);
    } else {
        MARKETING_INFO.save(deps.storage, &marketing_info)?;
    }

    let res = Response::new().add_attribute("action", "update_marketing");
    Ok(res)
}

// Copied from cw20-base and modified to allow owner to modify marketing info as
// well as the marketing address.
pub fn execute_upload_logo(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    logo: Logo,
) -> Result<Response, ContractError> {
    let mut marketing_info = MARKETING_INFO.may_load(deps.storage)?.unwrap_or_default();

    verify_logo(&logo)?;

    // Check sender is owner or marketer.
    let ownership = cw_ownable::get_ownership(deps.storage)?;
    let is_owner = ownership.owner.map_or(false, |owner| owner == info.sender);
    let is_marketer = marketing_info
        .marketing
        .as_ref()
        .map_or(false, |m| *m == info.sender);
    if !is_owner && !is_marketer {
        return Err(ContractError::Unauthorized {});
    }

    LOGO.save(deps.storage, &logo)?;

    let logo_info = match logo {
        Logo::Url(url) => LogoInfo::Url(url),
        Logo::Embedded(_) => LogoInfo::Embedded,
    };

    marketing_info.logo = Some(logo_info);
    MARKETING_INFO.save(deps.storage, &marketing_info)?;

    let res = Response::new().add_attribute("action", "upload_logo");
    Ok(res)
}

const LOGO_SIZE_CAP: usize = 5 * 1024;

/// Checks if data starts with XML preamble
fn verify_xml_preamble(data: &[u8]) -> Result<(), ContractError> {
    // The easiest way to perform this check would be just match on regex, however regex
    // compilation is heavy and probably not worth it.

    let preamble = data
        .split_inclusive(|c| *c == b'>')
        .next()
        .ok_or(ContractError::Cw20(
            cw20_base::ContractError::InvalidXmlPreamble {},
        ))?;

    const PREFIX: &[u8] = b"<?xml ";
    const POSTFIX: &[u8] = b"?>";

    if !(preamble.starts_with(PREFIX) && preamble.ends_with(POSTFIX)) {
        Err(ContractError::Cw20(
            cw20_base::ContractError::InvalidXmlPreamble {},
        ))
    } else {
        Ok(())
    }

    // Additionally attributes format could be validated as they are well defined, as well as
    // comments presence inside of preable, but it is probably not worth it.
}

/// Validates XML logo
fn verify_xml_logo(logo: &[u8]) -> Result<(), ContractError> {
    verify_xml_preamble(logo)?;

    if logo.len() > LOGO_SIZE_CAP {
        Err(ContractError::Cw20(cw20_base::ContractError::LogoTooBig {}))
    } else {
        Ok(())
    }
}

/// Validates png logo
fn verify_png_logo(logo: &[u8]) -> Result<(), ContractError> {
    // PNG header format:
    // 0x89 - magic byte, out of ASCII table to fail on 7-bit systems
    // "PNG" ascii representation
    // [0x0d, 0x0a] - dos style line ending
    // 0x1a - dos control character, stop displaying rest of the file
    // 0x0a - unix style line ending
    const HEADER: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    if logo.len() > LOGO_SIZE_CAP {
        Err(ContractError::Cw20(cw20_base::ContractError::LogoTooBig {}))
    } else if !logo.starts_with(&HEADER) {
        Err(ContractError::Cw20(
            cw20_base::ContractError::InvalidPngHeader {},
        ))
    } else {
        Ok(())
    }
}

/// Checks if passed logo is correct, and if not, returns an error
fn verify_logo(logo: &Logo) -> Result<(), ContractError> {
    match logo {
        Logo::Embedded(EmbeddedLogo::Svg(logo)) => verify_xml_logo(logo),
        Logo::Embedded(EmbeddedLogo::Png(logo)) => verify_png_logo(logo),
        Logo::Url(_) => Ok(()), // Any reasonable url validation would be regex based, probably not worth it
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // NEW VARIANTS FOR CW20-HOOKS
        QueryMsg::Hooks {} => to_json_binary(&HOOKS.query_hooks(deps)?),
        QueryMsg::Ownership {} => to_json_binary(&cw_ownable::get_ownership(deps.storage)?),

        // COPIED FROM CW20-BASE
        QueryMsg::Balance { address } => {
            to_json_binary(&cw20_base::contract::query_balance(deps, address)?)
        }
        QueryMsg::TokenInfo {} => to_json_binary(&cw20_base::contract::query_token_info(deps)?),
        QueryMsg::Minter {} => to_json_binary(&cw20_base::contract::query_minter(deps)?),
        QueryMsg::Allowance { owner, spender } => to_json_binary(
            &cw20_base::allowances::query_allowance(deps, owner, spender)?,
        ),
        QueryMsg::AllAllowances {
            owner,
            start_after,
            limit,
        } => to_json_binary(&cw20_base::enumerable::query_owner_allowances(
            deps,
            owner,
            start_after,
            limit,
        )?),
        QueryMsg::AllSpenderAllowances {
            spender,
            start_after,
            limit,
        } => to_json_binary(&cw20_base::enumerable::query_spender_allowances(
            deps,
            spender,
            start_after,
            limit,
        )?),
        QueryMsg::AllAccounts { start_after, limit } => to_json_binary(
            &cw20_base::enumerable::query_all_accounts(deps, start_after, limit)?,
        ),
        QueryMsg::MarketingInfo {} => {
            to_json_binary(&cw20_base::contract::query_marketing_info(deps)?)
        }
        QueryMsg::DownloadLogo {} => {
            to_json_binary(&cw20_base::contract::query_download_logo(deps)?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        HOOK_REPLY_ID => {
            // Error if hook execution fails, rolling back previous changes.
            msg.result
                .into_result()
                .map_err(|error| ContractError::HookErrored { error })?;

            Ok(Response::default())
        }
        _ => Err(ContractError::UnknownReplyId { id: msg.id }),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    match msg {
        MigrateMsg::FromBase { owner } => {
            // Validate safe to migrate.
            let stored = get_contract_version(deps.storage)?;
            if stored.contract != "crates.io:cw20-base" {
                return Err(ContractError::InvalidMigration {
                    expected: "crates.io:cw20-base".to_string(),
                    actual: stored.contract,
                });
            }

            // Update contract version.
            set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

            // Initialize owner.
            cw_ownable::initialize_owner(deps.storage, deps.api, Some(&owner))?;

            // Copied from cw20-base v1.1.2.
            let original_version =
                ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
            if original_version < "0.14.0".parse::<semver::Version>().unwrap() {
                // Build reverse map of allowances per spender
                let data = ALLOWANCES
                    .range(deps.storage, None, None, Order::Ascending)
                    .collect::<StdResult<Vec<_>>>()?;
                for ((owner, spender), allowance) in data {
                    ALLOWANCES_SPENDER.save(deps.storage, (&spender, &owner), &allowance)?;
                }
            }

            Ok(Response::default())
        }
    }
}
