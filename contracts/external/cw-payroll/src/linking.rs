use cosmwasm_std::{to_binary, CosmosMsg, DepsMut, Env, MessageInfo, Response, WasmMsg};

use crate::{
    msg::{ExecuteMsg, StreamId},
    state::{save_stream, Stream, STREAMS},
    ContractError,
};

pub enum LinkSyncType {
    Paused,
    Resumed,
}
pub(crate) trait SupportsLinking {
    fn link(
        &self,
        initiator_id: StreamId,
        link_id: StreamId,
        env:&Env,
        deps: DepsMut,
        info: &MessageInfo,
    ) -> Result<Response, ContractError>;
    fn detach(
        &self,
        initiator_id: StreamId,
        link_id: StreamId,
        env:&Env,
        deps: DepsMut,
        info: &MessageInfo,
    ) -> Result<Response, ContractError>;
    fn create_link_delete_msg(
        &self,
        stream_id: StreamId,
        env:&Env,
        deps: DepsMut,
        info: &MessageInfo,
    ) -> Result<CosmosMsg, ContractError>;
    fn create_sync_msg(
        &self,
        stream_id: StreamId,
        env:&Env,
        deps: DepsMut,
        info: &MessageInfo,
        sync_type: LinkSyncType,
    ) -> Result<CosmosMsg, ContractError>;
}

impl SupportsLinking for Stream {
    fn link(
        &self,
        initiator_id: StreamId,
        link_id: StreamId,
        env:&Env,
        deps: DepsMut,
        info: &MessageInfo,
    ) -> Result<Response, ContractError> {
        let mut initiator = STREAMS.may_load(deps.storage, initiator_id)?.ok_or(
            ContractError::InitiatorStreamNotFound {
                stream_id: initiator_id,
            },
        )?;

        let mut link = STREAMS
            .may_load(deps.storage, initiator_id)?
            .ok_or(ContractError::StreamNotFound {})?;

        if initiator.admin != info.sender {
            return Err(ContractError::Unauthorized {});
        }
        initiator.link_id = Some(link_id);
        initiator.is_link_initiator = true;
        link.link_id = Some(initiator_id);
        link.is_link_initiator = false;

        save_stream(deps.storage, initiator_id, &initiator).unwrap();
        save_stream(deps.storage, link_id, &link).unwrap();

        let response = Response::new()
            .add_attribute("method", "link")
            .add_attribute("initiator_id", initiator_id.to_string())
            .add_attribute("link_id", link_id.to_string())
            .add_attribute("admin", info.sender.clone());

        Ok(response)
    }

    fn detach(
        &self,
        initiator_id: StreamId,
        link_id: StreamId,
        _env:&Env,
        deps: DepsMut,
        info: &MessageInfo,
    ) -> Result<Response, ContractError> {
        let mut initiator = STREAMS.may_load(deps.storage, initiator_id)?.ok_or(
            ContractError::InitiatorStreamNotFound {
                stream_id: initiator_id,
            },
        )?;

        let mut link = STREAMS
            .may_load(deps.storage, initiator_id)?
            .ok_or(ContractError::StreamNotFound {})?;

        if initiator.admin != info.sender {
            return Err(ContractError::Unauthorized {});
        }
        initiator.link_id = None;
        initiator.is_link_initiator = false;
        link.link_id = None;
        link.is_link_initiator = false;

        save_stream(deps.storage, initiator_id, &initiator).unwrap();
        save_stream(deps.storage, link_id, &link).unwrap();
        let response = Response::new()
            .add_attribute("method", "link")
            .add_attribute("initiator_id", initiator_id.to_string())
            .add_attribute("link_id", link_id.to_string())
            .add_attribute("admin", info.sender.clone());

        Ok(response)
    }

    fn create_link_delete_msg(
        &self,
        stream_id: StreamId,
        env:&Env,
        deps: DepsMut,
        info: &MessageInfo,
    ) -> Result<CosmosMsg, ContractError> {
        let stream = STREAMS
            .may_load(deps.storage, stream_id)?
            .ok_or(ContractError::StreamNotFound {})?;
        if stream.link_id.is_none() {
            return Err(ContractError::StreamNotLinked {});
        }
        if !stream.is_link_initiator {
            return Err(ContractError::StreamNotInitiator {});
        }
        let msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.clone().into_string(),
            msg: to_binary(&ExecuteMsg::RemoveStream {
                id: stream.link_id.unwrap(),
            })?,
            funds: vec![],
        });
        Ok(msg)
    }

    fn create_sync_msg(
        &self,
        stream_id: StreamId,
        env:&Env,
        deps: DepsMut,
        info: &MessageInfo,
        sync_type: LinkSyncType,
    ) -> Result<CosmosMsg, ContractError> {
        let stream = STREAMS
            .may_load(deps.storage, stream_id)?
            .ok_or(ContractError::StreamNotFound {})?;
        match sync_type {
            LinkSyncType::Paused => {
                let msg = CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: env.contract.address.clone().into_string(),
                    msg: to_binary(&ExecuteMsg::PauseStream {
                        id: stream.link_id.unwrap(),
                    })?,
                    funds: vec![],
                });
                return Ok(msg);
            }
            LinkSyncType::Resumed => {
                let msg = CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: env.contract.address.clone().into_string(),
                    msg: to_binary(&ExecuteMsg::ResumeStream {
                        id: stream.link_id.unwrap(),
                        start_time: None,
                        end_time: None,
                    })?,
                    funds: vec![],
                });
                return Ok(msg);
            }
        }
    }
}
