use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::{ProcessError, Result};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub chain_cfg: ChainConfig,
    #[serde(default)]
    pub contract_deploy_info: HashMap<String, DeployInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChainConfig {
    pub denom: String,
    pub prefix: String,
    pub chain_id: String,
    pub derivation_path: String,
    pub gas_price: f64,
    pub gas_adjustment: f64,
    #[serde(default)]
    pub rpc_endpoint: Option<String>,
    // Retain old YAML configuration on a round trip, even though this client
    // does not use gRPC. Never invent an RPC endpoint from this address.
    #[serde(default)]
    pub grpc_endpoint: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeployInfo {
    pub code_id: Option<u64>,
    pub address: Option<String>,
}

impl Config {
    pub fn from_yaml(file: &str) -> Result<Self> {
        let bytes = std::fs::read(file)?;
        serde_yaml::from_slice(&bytes).map_err(|e| ProcessError::Config(e.to_string()))
    }
}

impl ChainConfig {
    pub(crate) fn validate(&self) -> Result<&str> {
        let endpoint = self.rpc_endpoint.as_deref().filter(|s| !s.is_empty()).ok_or_else(|| {
            ProcessError::Config("rpc_endpoint is required; supply the RPC URL for this chain (a grpc_endpoint cannot be substituted)".into())
        })?;
        if !self.gas_price.is_finite()
            || self.gas_price < 0.0
            || !self.gas_adjustment.is_finite()
            || self.gas_adjustment <= 0.0
        {
            return Err(ProcessError::Config(
                "gas price/adjustment must be finite and nonnegative/positive".into(),
            ));
        }
        self.chain_id
            .parse::<cosmrs::tendermint::chain::Id>()
            .map_err(|e| ProcessError::Config(e.to_string()))?;
        self.denom
            .parse::<cosmrs::Denom>()
            .map_err(|e| ProcessError::Config(e.to_string()))?;
        Ok(endpoint)
    }
}
