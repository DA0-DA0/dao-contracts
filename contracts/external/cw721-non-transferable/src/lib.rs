pub use crate::msg::{InstantiateMsg, QueryMsg};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Empty;
pub use cw721_base::{
    entry::{execute as _execute, query as _query},
    ContractError, Cw721Contract, ExecuteMsg, InstantiateMsg as Cw721BaseInstantiateMsg,
    MinterResponse,
};

pub mod msg;
pub mod state;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw721-non-transferable-with-weight";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cw_serde]
pub struct MetadataExtension {
    pub weight: u32,
}

#[cw_serde]
pub enum ExecuteExt {
    /// Update a given token ID with a new token URI or weight
    UpdateToken { id: String, weight: Option<u64>, token_uri: Option<String> }
    /// Add a new hook to be informed of all membership changes. Must be called by Admin
    AddHook { addr: String },
    /// Remove a hook. Must be called by Admin
    RemoveHook { addr: String },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryExt {
 /// /// TODO maybe make these an exetension?
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
    Cw721Contract<'a, MetadataExtension, Empty, Empty, Empty>;

// TODO add weight extension to token
// TODO hooks for minting, burning, updates?
// TODO updatable? (yes, by minter)

#[cfg(not(feature = "library"))]
pub mod entry {
    use super::*;
    use crate::state::{Config, CONFIG, TOTAL};
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
        // TODO use cw-ownable
        // let admin_addr: Option<Addr> = msg
        //     .admin
        //     .as_deref()
        //     .map(|s| deps.api.addr_validate(s))
        //     .transpose()?;

        // let config = Config { admin: admin_addr };

        // CONFIG.save(deps.storage, &config)?;

        let cw721_base_instantiate_msg = Cw721BaseInstantiateMsg {
            name: msg.name,
            symbol: msg.symbol,
            minter: msg.minter,
        };

        Cw721NonTransferableContract::default().instantiate(
            deps.branch(),
            env,
            info,
            cw721_base_instantiate_msg,
        )?;

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
        msg: ExecuteMsg<MetadataExtension, Empty>,
    ) -> Result<Response, cw721_base::ContractError> {
        // TODO use cw-ownable
        let owner = cw_ownable::assert_owner(deps.storage, &info.sender)?;
        match owner {
            Some(admin) => {
                if admin == info.sender {
                    match msg {
                        // TODO on burn / mint / update, update member weights and total
                        ExecuteMsg::Mint(msg) => unimplemented!(),
                        ExecuteMsg::Burn { id } => unimplemented!(),
                        // TODO implement hooks and update token
                        ExecuteMsg::Extension { msg } => match msg {
                            ExecuteExt::AddHook { addr } => unimplemented!(),
                            ExecuteExt::RemoveHook { addr } => unimplemented!(),
                            ExecuteExt::UpdateToken { id, token_uri, weight } => unimplemented!(),
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

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        // TODO query weight by owner at height?
        // TODO query total weight at height?
        match msg {
            _ => _query(deps, env, msg.into()),
        }
    }
}
