#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Reply, Response, StdResult,
    SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use cw4::{MemberResponse, TotalWeightResponse};
use cw721_base::{
    ExecuteMsg as Cw721ExecuteMsg, InstantiateMsg as Cw721InstantiateMsg, QueryMsg as Cw721QueryMsg,
};
use cw_ownable::Action;
use cw_utils::parse_reply_instantiate_data;
use dao_cw721_extensions::roles::{ExecuteExt, MetadataExt, QueryExt};

use crate::msg::{ExecuteMsg, InstantiateMsg, NftContract, QueryMsg};
use crate::state::{Config, CONFIG, DAO, INITIAL_NFTS};
use crate::ContractError;

pub(crate) const CONTRACT_NAME: &str = "crates.io:dao-voting-cw721-roles";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_NFT_CONTRACT_REPLY_ID: u64 = 0;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<Empty>, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    DAO.save(deps.storage, &info.sender)?;

    match msg.nft_contract {
        NftContract::Existing { address } => {
            let config = Config {
                nft_address: deps.api.addr_validate(&address)?,
            };
            CONFIG.save(deps.storage, &config)?;

            Ok(Response::default()
                .add_attribute("method", "instantiate")
                .add_attribute("nft_contract", address))
        }
        NftContract::New {
            code_id,
            label,
            name,
            symbol,
            initial_nfts,
        } => {
            // Check there is at least one NFT to initialize
            if initial_nfts.is_empty() {
                return Err(ContractError::NoInitialNfts {});
            }

            // Save initial NFTs for use in reply
            INITIAL_NFTS.save(deps.storage, &initial_nfts)?;

            // Create instantiate submessage for NFT roles contract
            let msg = SubMsg::reply_on_success(
                WasmMsg::Instantiate {
                    code_id,
                    funds: vec![],
                    admin: Some(info.sender.to_string()),
                    label,
                    msg: to_json_binary(&Cw721InstantiateMsg {
                        name,
                        symbol,
                        // Admin must be set to contract to mint initial NFTs
                        minter: env.contract.address.to_string(),
                    })?,
                },
                INSTANTIATE_NFT_CONTRACT_REPLY_ID,
            );

            Ok(Response::default().add_submessage(msg))
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response<Empty>, ContractError> {
    Err(ContractError::NoExecute {})
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => query_config(deps),
        QueryMsg::Dao {} => query_dao(deps),
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
    at_height: Option<u64>,
) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    let member: MemberResponse = deps.querier.query_wasm_smart(
        config.nft_address,
        &Cw721QueryMsg::<QueryExt>::Extension {
            msg: QueryExt::Member {
                addr: address,
                at_height,
            },
        },
    )?;

    to_json_binary(&dao_interface::voting::VotingPowerAtHeightResponse {
        power: member.weight.unwrap_or(0).into(),
        height: at_height.unwrap_or(env.block.height),
    })
}

pub fn query_total_power_at_height(
    deps: Deps,
    env: Env,
    at_height: Option<u64>,
) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    let total: TotalWeightResponse = deps.querier.query_wasm_smart(
        config.nft_address,
        &Cw721QueryMsg::<QueryExt>::Extension {
            msg: QueryExt::TotalWeight { at_height },
        },
    )?;

    to_json_binary(&dao_interface::voting::TotalPowerAtHeightResponse {
        power: total.weight.into(),
        height: at_height.unwrap_or(env.block.height),
    })
}

pub fn query_config(deps: Deps) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    to_json_binary(&config)
}

pub fn query_dao(deps: Deps) -> StdResult<Binary> {
    let dao = DAO.load(deps.storage)?;
    to_json_binary(&dao)
}

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_json_binary(&dao_interface::voting::InfoResponse { info })
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

                    // Save config
                    let config = Config {
                        nft_address: deps.api.addr_validate(&nft_contract)?,
                    };
                    CONFIG.save(deps.storage, &config)?;

                    let initial_nfts = INITIAL_NFTS.load(deps.storage)?;

                    // Add mint submessages
                    let mint_submessages: Vec<SubMsg> = initial_nfts
                        .iter()
                        .flat_map(|nft| -> Result<SubMsg, ContractError> {
                            Ok(SubMsg::new(WasmMsg::Execute {
                                contract_addr: nft_contract.clone(),
                                funds: vec![],
                                msg: to_json_binary(
                                    &Cw721ExecuteMsg::<MetadataExt, ExecuteExt>::Mint {
                                        token_id: nft.token_id.clone(),
                                        owner: nft.owner.clone(),
                                        token_uri: nft.token_uri.clone(),
                                        extension: MetadataExt {
                                            role: nft.clone().extension.role,
                                            weight: nft.extension.weight,
                                        },
                                    },
                                )?,
                            }))
                        })
                        .collect::<Vec<SubMsg>>();

                    // Clear space
                    INITIAL_NFTS.remove(deps.storage);

                    // Update minter message
                    let update_minter_msg = WasmMsg::Execute {
                        contract_addr: nft_contract.clone(),
                        msg: to_json_binary(
                            &Cw721ExecuteMsg::<MetadataExt, ExecuteExt>::UpdateOwnership(
                                Action::TransferOwnership {
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
