use anyhow::Result;
use cosmwasm_std::{from_json, to_json_binary, Addr, Api, Binary, BlockInfo, Querier, Storage};
use cw_multi_test::{error::AnyResult, AppResponse, CosmosRouter, Stargate};
use omniflix_std::types::omniflix::onft::v1beta1::{
    Collection, Denom, MsgCreateDenom, MsgCreateDenomResponse, MsgMintOnft, MsgMintOnftResponse,
    MsgTransferOnft, MsgTransferOnftResponse, QuerySupplyRequest, QuerySupplyResponse,
};
use omniflix_std::types::omniflix::onft::v1beta1::{Onft, QueryOnftRequest, QueryOnftResponse};
use prost::{DecodeError, Message};

const COLLECTION_PREFIX: &str = "collection";

pub struct StargateKeeper {}

impl StargateKeeper {}

impl Stargate for StargateKeeper {
    fn execute<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        sender: Addr,
        type_url: String,
        value: Binary,
    ) -> AnyResult<AppResponse> {
        if type_url == *"/OmniFlix.onft.v1beta1.MsgCreateDenom" {
            let msg: MsgCreateDenom = Message::decode(value.as_slice()).unwrap();
            let collection = Collection {
                denom: Some(Denom {
                    creator: sender.to_string(),
                    data: msg.data,
                    name: msg.name,
                    id: msg.id.clone(),
                    preview_uri: msg.preview_uri,
                    description: msg.description,
                    schema: msg.schema,
                    symbol: msg.symbol,
                    uri: msg.uri,
                    uri_hash: msg.uri_hash,
                    royalty_receivers: msg.royalty_receivers,
                }),
                onfts: vec![],
            };
            let key = format!("collections:{}:{}", COLLECTION_PREFIX, msg.id);
            let serialized_collection =
                to_json_binary(&collection).expect("Failed to serialize Collection");
            storage.set(key.as_bytes(), &serialized_collection);

            return Ok(AppResponse {
                events: vec![],
                data: Some(Binary::from(MsgCreateDenomResponse {})),
            });
        }
        if type_url == *"/OmniFlix.onft.v1beta1.MsgMintONFT" {
            let msg: MsgMintOnft = Message::decode(value.as_slice()).unwrap();
            let key = format!("collections:{}:{}", COLLECTION_PREFIX, msg.denom_id.clone());
            let serialized_collection = storage.get(key.as_bytes());
            let mut collection: Collection = from_json(serialized_collection.unwrap())
                .expect("Failed to deserialize Collection");
            let onft = Onft {
                id: msg.id,
                created_at: None,
                nsfw: msg.nsfw,
                owner: msg.recipient,
                data: msg.data,
                transferable: msg.transferable,
                extensible: msg.extensible,
                metadata: msg.metadata,
                royalty_share: msg.royalty_share,
            };
            collection.onfts.push(onft);
            let serialized_collection =
                to_json_binary(&collection).expect("Failed to serialize Collection");
            storage.set(key.as_bytes(), &serialized_collection);

            return Ok(AppResponse {
                events: vec![],
                data: Some(Binary::from(MsgMintOnftResponse {})),
            });
        }
        if type_url == *"/OmniFlix.onft.v1beta1.MsgTransferONFT" {
            let parsed_msg: Result<MsgTransferOnft, DecodeError> =
                Message::decode(value.as_slice());
            if let Ok(msg) = parsed_msg {
                let key = format!("collections:{}:{}", COLLECTION_PREFIX, msg.denom_id.clone());
                let serialized_collection = storage.get(key.as_bytes());
                let mut collection: Collection = from_json(serialized_collection.unwrap())
                    .expect("Failed to deserialize Collection");
                let onft = collection.onfts.iter_mut().find(|onft| onft.id == msg.id);
                let onft = onft.unwrap();
                onft.owner = msg.recipient;
                let serialized_collection =
                    to_json_binary(&collection).expect("Failed to serialize Collection");
                storage.set(key.as_bytes(), &serialized_collection);

                return Ok(AppResponse {
                    events: vec![],
                    data: Some(Binary::from(MsgTransferOnftResponse {})),
                });
            };
        }
        Ok(AppResponse::default())
    }

    fn query(
        &self,
        _api: &dyn Api,
        storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        path: String,
        data: Binary,
    ) -> AnyResult<Binary> {
        if path == *"/OmniFlix.onft.v1beta1.Query/ONFT" {
            let request: QueryOnftRequest = Message::decode(data.as_slice()).unwrap();

            let key = format!("collections:{}:{}", COLLECTION_PREFIX, request.denom_id);
            let serialized_collection = storage.get(key.as_bytes());
            let collection: Collection = from_json(serialized_collection.unwrap())
                .expect("Failed to deserialize Collection");
            let onft = collection
                .onfts
                .into_iter()
                .find(|onft| onft.id == request.id);

            return Ok(to_json_binary(&QueryOnftResponse { onft })?);
        }
        if path == *"/OmniFlix.onft.v1beta1.Query/Supply" {
            let request: QuerySupplyRequest = Message::decode(data.as_slice()).unwrap();

            let key = format!("collections:{}:{}", COLLECTION_PREFIX, request.denom_id);
            let serialized_collection = storage.get(key.as_bytes());
            let collection: Collection = from_json(serialized_collection.unwrap())
                .expect("Failed to deserialize Collection");

            return Ok(to_json_binary(&QuerySupplyResponse {
                amount: collection.onfts.len() as u64,
            })?);
        }
        Ok(data)
    }
}
