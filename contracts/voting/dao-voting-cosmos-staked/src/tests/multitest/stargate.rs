use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    from_json, storage_keys::to_length_prefixed, to_json_binary, Addr, Api, Binary, BlockInfo,
    Decimal, Querier, Storage,
};
use cw_multi_test::{error::AnyResult, AppResponse, CosmosRouter, Stargate};
use cw_storage_plus::Map;
use osmosis_std::types::cosmos::staking::v1beta1::{Pool, QueryPoolResponse};

use super::tests::{DELEGATOR, VALIDATOR};

// from StakingKeeper
#[cw_serde]
struct Shares {
    stake: Decimal,
    rewards: Decimal,
}
const NAMESPACE_STAKING: &[u8] = b"staking";
const STAKES: Map<(&Addr, &Addr), Shares> = Map::new("stakes");
pub struct StargateKeeper {}

impl StargateKeeper {}

impl Stargate for StargateKeeper {
    fn execute<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _sender: Addr,
        _type_url: String,
        _value: Binary,
    ) -> AnyResult<AppResponse> {
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
        if path == *"/cosmos.staking.v1beta1.Query/Pool" {
            // since we can't access the app or staking module, let's just raw
            // access the storage key we know is set for staking in the test :D
            let mut key = to_length_prefixed(NAMESPACE_STAKING);
            let map_key = STAKES.key((&Addr::unchecked(DELEGATOR), &Addr::unchecked(VALIDATOR)));
            key.extend_from_slice(&map_key);
            let data: Shares = from_json(storage.get(&key).unwrap())?;

            return Ok(to_json_binary(&QueryPoolResponse {
                pool: Some(Pool {
                    not_bonded_tokens: "0".to_string(),
                    bonded_tokens: data.stake.to_string(),
                }),
            })?);
        }
        Ok(data)
    }
}
