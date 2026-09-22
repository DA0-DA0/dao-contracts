use std::{collections::HashMap, ffi::OsStr, panic::Location, path::Path, time::Duration};

use cosmrs::{
    proto::{
        cosmos::{
            bank::v1beta1::{QueryBalanceRequest, QueryBalanceResponse},
            staking::v1beta1::{QueryValidatorsRequest, QueryValidatorsResponse},
        },
        cosmwasm::wasm::v1::{
            AccessConfig, MsgExecuteContract, MsgInstantiateContract,
            MsgInstantiateContractResponse, MsgStoreCode, MsgStoreCodeResponse,
            QuerySmartContractStateRequest, QuerySmartContractStateResponse,
        },
        traits::Message,
    },
    Any,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::runtime::{Builder, Runtime};

use crate::{
    error::{protocol, ProcessError},
    gas::{self, GasReport},
    registry::ContractMap,
    rpc::{Receipt, Rpc},
    Address, Coin, Config, Result, SigningKey,
};

pub struct ChainClient {
    pub contract_map: ContractMap,
    rpc: Rpc,
    runtime: Runtime,
    gas_report: Option<GasReport>,
}

pub struct StoreResponse {
    pub code_id: u64,
    pub res: Receipt,
}
pub struct InstantiateResponse {
    pub address: String,
    pub res: Receipt,
}
pub struct QueryResponse {
    bytes: Vec<u8>,
}

impl QueryResponse {
    pub fn data<'a, T: Deserialize<'a>>(&'a self) -> Result<T> {
        Ok(serde_json::from_slice(&self.bytes)?)
    }
}

impl ChainClient {
    pub fn new(cfg: Config, profile_gas: bool) -> Result<Self> {
        let runtime = Builder::new_current_thread().enable_all().build()?;
        let rpc = {
            let _entered = runtime.enter();
            Rpc::new(cfg.chain_cfg)?
        };
        Ok(Self {
            contract_map: ContractMap::new(cfg.contract_deploy_info),
            rpc,
            runtime,
            gas_report: profile_gas.then(HashMap::new),
        })
    }

    #[track_caller]
    pub fn store_contracts(
        &mut self,
        directory: &str,
        key: &SigningKey,
        instantiate_permission: Option<AccessConfig>,
    ) -> Result<Vec<StoreResponse>> {
        let mut paths = std::fs::read_dir(directory)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<Vec<_>>>()?;
        paths.retain(|path| path.extension() == Some(OsStr::new("wasm")));
        paths.sort();
        let mut stored = Vec::with_capacity(paths.len());
        for path in paths {
            let name = artifact_name(&path)?;
            let bytes = std::fs::read(&path)?;
            let checksum = Sha256::digest(&bytes).to_vec();
            let receipt = self.runtime.block_on(self.rpc.transact(key, |sender| {
                Ok(Any {
                    type_url: "/cosmwasm.wasm.v1.MsgStoreCode".into(),
                    value: MsgStoreCode {
                        sender: sender.to_string(),
                        wasm_byte_code: bytes,
                        instantiate_permission: instantiate_permission.clone(),
                    }
                    .encode_to_vec(),
                })
            }))?;
            let response: MsgStoreCodeResponse =
                receipt.message_response("/cosmwasm.wasm.v1.MsgStoreCodeResponse")?;
            if response.code_id == 0 || response.checksum != checksum {
                return Err(protocol(
                    "store response has invalid code ID or a different Wasm checksum",
                ));
            }
            self.contract_map.register(&name, response.code_id);
            gas::record(
                &mut self.gas_report,
                &name,
                "Store",
                "Store",
                &receipt,
                Location::caller(),
            );
            println!(
                "Stored {}: code_id={} tx={} height={} gas_wanted={} gas_used={}",
                path.file_name().unwrap_or_default().to_string_lossy(),
                response.code_id,
                receipt.hash,
                receipt.height,
                receipt.gas_wanted,
                receipt.gas_used
            );
            stored.push(StoreResponse {
                code_id: response.code_id,
                res: receipt,
            });
        }
        Ok(stored)
    }

    #[track_caller]
    pub fn instantiate<T: Serialize>(
        &mut self,
        name: &str,
        operation: &str,
        msg: &T,
        key: &SigningKey,
        admin: Option<Address>,
        funds: Vec<Coin>,
    ) -> Result<InstantiateResponse> {
        let code_id = self.contract_map.code_id(name)?;
        let receipt = self.runtime.block_on(self.rpc.transact(key, |sender| {
            Ok(Any {
                type_url: "/cosmwasm.wasm.v1.MsgInstantiateContract".into(),
                value: MsgInstantiateContract {
                    sender: sender.to_string(),
                    admin: admin.map(|address| address.to_string()).unwrap_or_default(),
                    code_id,
                    // Preserve the original harness label and transaction size.
                    label: "cosm-orc".into(),
                    msg: serde_json::to_vec(msg)?,
                    funds: funds.into_iter().map(Into::into).collect(),
                }
                .encode_to_vec(),
            })
        }))?;
        let response: MsgInstantiateContractResponse =
            receipt.message_response("/cosmwasm.wasm.v1.MsgInstantiateContractResponse")?;
        response.address.parse::<Address>().map_err(protocol)?;
        self.contract_map
            .add_address(name, response.address.clone())?;
        gas::record(
            &mut self.gas_report,
            name,
            operation,
            "Instantiate",
            &receipt,
            Location::caller(),
        );
        Ok(InstantiateResponse {
            address: response.address,
            res: receipt,
        })
    }

    #[track_caller]
    pub fn execute<T: Serialize>(
        &mut self,
        name: &str,
        operation: &str,
        msg: &T,
        key: &SigningKey,
        funds: Vec<Coin>,
    ) -> Result<Receipt> {
        let contract = self.contract_map.address(name)?;
        let receipt = self.runtime.block_on(self.rpc.transact(key, |sender| {
            Ok(Any {
                type_url: "/cosmwasm.wasm.v1.MsgExecuteContract".into(),
                value: MsgExecuteContract {
                    sender: sender.to_string(),
                    contract,
                    msg: serde_json::to_vec(msg)?,
                    funds: funds.into_iter().map(Into::into).collect(),
                }
                .encode_to_vec(),
            })
        }))?;
        gas::record(
            &mut self.gas_report,
            name,
            operation,
            "Execute",
            &receipt,
            Location::caller(),
        );
        Ok(receipt)
    }

    pub fn query<T: Serialize>(&self, name: &str, msg: &T) -> Result<QueryResponse> {
        let response: QuerySmartContractStateResponse = self.runtime.block_on(self.rpc.query(
            "/cosmwasm.wasm.v1.Query/SmartContractState",
            QuerySmartContractStateRequest {
                address: self.contract_map.address(name)?,
                query_data: serde_json::to_vec(msg)?,
            },
        ))?;
        Ok(QueryResponse {
            bytes: response.data,
        })
    }

    pub fn balance(&self, address: &str, denom: &str) -> Result<u128> {
        let response: QueryBalanceResponse = self.runtime.block_on(self.rpc.query(
            "/cosmos.bank.v1beta1.Query/Balance",
            QueryBalanceRequest {
                address: address.into(),
                denom: denom.into(),
            },
        ))?;
        let coin = response
            .balance
            .ok_or_else(|| protocol("bank query omitted balance"))?;
        if coin.denom != denom {
            return Err(protocol("bank query returned a different denomination"));
        }
        coin.amount.parse().map_err(protocol)
    }

    pub fn bonded_validators(&self) -> Result<Vec<String>> {
        let response: QueryValidatorsResponse = self.runtime.block_on(self.rpc.query(
            "/cosmos.staking.v1beta1.Query/Validators",
            QueryValidatorsRequest {
                status: "BOND_STATUS_BONDED".into(),
                pagination: None,
            },
        ))?;
        Ok(response
            .validators
            .into_iter()
            .map(|validator| validator.operator_address)
            .collect())
    }

    pub fn poll_for_n_blocks(&self, n: u64, budget: Duration, first_block: bool) -> Result<()> {
        self.runtime
            .block_on(self.rpc.poll_for_n_blocks(n, budget, first_block))
    }

    pub fn poll_for_n_secs(&self, n: u64, budget: Duration) -> Result<()> {
        self.runtime.block_on(self.rpc.poll_for_n_secs(n, budget))
    }

    pub fn gas_profiler_report(&self) -> Result<&GasReport> {
        self.gas_report
            .as_ref()
            .ok_or_else(|| ProcessError::Config("gas profiling is disabled".into()))
    }
}

fn artifact_name(path: &Path) -> Result<String> {
    let stem = path
        .file_stem()
        .and_then(OsStr::to_str)
        .ok_or_else(|| protocol("invalid Wasm filename"))?;
    // Preserve named variants. Only strip the legacy current-architecture suffix,
    // never turn default/Thorchain builds into an ambiguous unsuffixed alias.
    Ok(stem
        .strip_suffix(&format!("-{}", std::env::consts::ARCH))
        .unwrap_or(stem)
        .into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_data_can_borrow_a_string() {
        let response = QueryResponse {
            bytes: br#""juno-address""#.to_vec(),
        };
        let address: &str = response.data().unwrap();
        assert_eq!(address, "juno-address");
    }

    #[test]
    fn named_artifacts_and_legacy_architecture_suffix_are_preserved() {
        assert_eq!(
            artifact_name(Path::new("token-default.wasm")).unwrap(),
            "token-default"
        );
        assert_eq!(
            artifact_name(Path::new("token-thorchain.wasm")).unwrap(),
            "token-thorchain"
        );
        let legacy = format!("token-default-{}.wasm", std::env::consts::ARCH);
        assert_eq!(artifact_name(Path::new(&legacy)).unwrap(), "token-default");
    }
}
