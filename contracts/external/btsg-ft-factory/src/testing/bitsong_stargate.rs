use anyhow::Error;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    from_json, to_json_binary, Addr, Api, Binary, BlockInfo, Coin, Event, Querier, Storage, Uint64,
};
use cw_multi_test::{error::AnyResult, AppResponse, BankSudo, CosmosRouter, Stargate, SudoMsg};
use prost::Message;

use crate::bitsong::{
    MsgIssue, MsgIssueResponse, MsgMint, MsgMintResponse, MsgSetMinter, MsgSetMinterResponse,
};

const DENOMS_PREFIX: &str = "denoms";
const DENOMS_COUNT_KEY: &str = "denoms_count";

#[cw_serde]
struct FanToken {
    pub denom: String,
    pub name: String,
    pub symbol: String,
    pub max_supply: String,
    pub authority: String,
    pub minter: String,
    pub uri: String,
}

pub struct StargateKeeper {}

impl StargateKeeper {}

impl Stargate for StargateKeeper {
    fn execute<ExecC, QueryC: cosmwasm_std::CustomQuery>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        type_url: String,
        value: Binary,
    ) -> AnyResult<AppResponse> {
        if type_url == *"/bitsong.fantoken.MsgIssue" {
            let denoms_count: Uint64 = storage
                .get(DENOMS_COUNT_KEY.as_bytes())
                .map_or_else(Uint64::zero, |d| from_json(d).unwrap());
            let denom = format!("fantoken{}", denoms_count.u64() + 1);

            let msg: MsgIssue = Message::decode(value.as_slice()).unwrap();
            let ft = FanToken {
                denom: denom.clone(),
                name: msg.name,
                symbol: msg.symbol,
                max_supply: msg.max_supply,
                authority: msg.authority,
                minter: msg.minter,
                uri: msg.uri,
            };

            let key = format!("{}:{}", DENOMS_PREFIX, denom.clone());
            let serialized_ft = to_json_binary(&ft).expect("Failed to serialize FanToken");
            storage.set(key.as_bytes(), &serialized_ft);

            return Ok(AppResponse {
                events: vec![
                    Event::new("bitsong.fantoken.v1beta1.EventIssue").add_attribute("denom", denom)
                ],
                data: Some(Binary::from(MsgIssueResponse {})),
            });
        }
        if type_url == *"/bitsong.fantoken.MsgMint" {
            let msg: MsgMint = Message::decode(value.as_slice()).unwrap();

            let coin = msg.coin.unwrap();

            let key = format!("{}:{}", DENOMS_PREFIX, coin.denom.clone());
            let serialized_ft = storage.get(key.as_bytes());
            let fantoken: FanToken =
                from_json(serialized_ft.unwrap()).expect("Failed to deserialize FanToken");

            if fantoken.minter != msg.minter || sender != msg.minter {
                return Err(Error::msg("Minter unauthorized"));
            }

            router.sudo(
                api,
                storage,
                block,
                SudoMsg::Bank(BankSudo::Mint {
                    to_address: msg.recipient.clone(),
                    amount: vec![Coin::new(coin.amount.parse().unwrap(), coin.denom.clone())],
                }),
            )?;

            return Ok(AppResponse {
                events: vec![],
                data: Some(Binary::from(MsgMintResponse {})),
            });
        }
        if type_url == *"/bitsong.fantoken.MsgSetMinter" {
            let msg: MsgSetMinter = Message::decode(value.as_slice()).unwrap();

            let key = format!("{}:{}", DENOMS_PREFIX, msg.denom.clone());
            let serialized_ft = storage.get(key.as_bytes());
            let mut fantoken: FanToken =
                from_json(serialized_ft.unwrap()).expect("Failed to deserialize FanToken");

            if fantoken.minter != msg.old_minter {
                return Err(Error::msg("Old minter does not match"));
            }

            fantoken.minter = msg.new_minter;
            storage.set(key.as_bytes(), &to_json_binary(&fantoken).unwrap());

            return Ok(AppResponse {
                events: vec![],
                data: Some(Binary::from(MsgSetMinterResponse {})),
            });
        }
        Ok(AppResponse::default())
    }

    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        _path: String,
        data: Binary,
    ) -> AnyResult<Binary> {
        Ok(data)
    }
}
