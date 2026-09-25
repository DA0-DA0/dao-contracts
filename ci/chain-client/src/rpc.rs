use std::{future::Future, time::Duration};

use base64::{engine::general_purpose::STANDARD, Engine};
use cosmrs::{
    proto::{
        cosmos::{
            auth::v1beta1::{BaseAccount, QueryAccountRequest, QueryAccountResponse},
            base::abci::v1beta1::TxMsgData,
            tx::v1beta1::{SimulateRequest, SimulateResponse},
        },
        traits::Message,
    },
    tendermint::{abci::Event, Hash},
    AccountId, Any,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tendermint_rpc::{client::CompatMode, endpoint, Client, HttpClient};
use tokio::time::{sleep, timeout};

use crate::{
    config::ChainConfig,
    error::{protocol, ProcessError, Result, TxError},
    signing::{self, SigningKey},
};

pub const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_millis(500);

#[cfg(test)]
mod pinned_juno;
#[cfg(test)]
mod tests;

/// Committed execution data; CheckTx is never converted into a receipt.
#[derive(Clone, Debug, Serialize)]
pub struct Receipt {
    pub hash: String,
    pub height: u64,
    pub code: u32,
    pub codespace: String,
    pub log: String,
    pub gas_wanted: u64,
    pub gas_used: u64,
    pub data: Vec<u8>,
    pub events: Vec<Event>,
}

impl Receipt {
    pub fn message_response<T: Message + Default>(&self, type_url: &str) -> Result<T> {
        let data = TxMsgData::decode(self.data.as_slice()).map_err(protocol)?;
        // The facade submits exactly one top-level message. Nested instantiate
        // events can name other contracts, so they are not an address oracle.
        if data.msg_responses.len() != 1 || data.msg_responses[0].type_url != type_url {
            return Err(protocol(format!(
                "expected one {type_url} message response"
            )));
        }
        T::decode(data.msg_responses[0].value.as_slice()).map_err(protocol)
    }
}

pub struct Rpc {
    client: HttpClient,
    cfg: ChainConfig,
    operation_timeout: Duration,
}

impl Rpc {
    pub fn new(cfg: ChainConfig) -> Result<Self> {
        let endpoint: tendermint_rpc::HttpClientUrl = cfg.validate()?.try_into()?;
        let transport = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(OPERATION_TIMEOUT)
            .connect_timeout(OPERATION_TIMEOUT)
            .build()
            .map_err(|e| ProcessError::Rpc(e.to_string()))?;
        // broadcast_tx_commit's request is identical for CometBFT 0.37 and
        // 0.38, and the response type aliases both field spellings, so the
        // 0.37 dialect also decodes 0.38 responses for the endpoints used
        // here. status() still rejects unsupported node versions.
        let client = HttpClient::builder(endpoint)
            .compat_mode(CompatMode::V0_37)
            .client(transport)
            .build()?;
        Ok(Self {
            client,
            cfg,
            operation_timeout: OPERATION_TIMEOUT,
        })
    }

    async fn bounded<T>(&self, future: impl Future<Output = Result<T>>) -> Result<T> {
        timeout(self.operation_timeout, future)
            .await
            .map_err(|_| ProcessError::Timeout { hash: None })?
    }

    async fn status_inner(&self) -> Result<endpoint::status::Response> {
        let status = self.client.status().await?;
        if status.node_info.network.as_str() != self.cfg.chain_id {
            return Err(protocol("node chain ID does not match configured chain_id"));
        }
        if !status.node_info.version.to_string().starts_with("0.37.")
            && !status.node_info.version.to_string().starts_with("0.38.")
        {
            return Err(protocol(
                "this fixture client requires CometBFT 0.37 or 0.38",
            ));
        }
        Ok(status)
    }

    pub async fn status(&self) -> Result<endpoint::status::Response> {
        self.bounded(self.status_inner()).await
    }

    async fn query_inner<Q: Message, R: Message + Default>(
        &self,
        path: &str,
        request: Q,
    ) -> Result<R> {
        let res = self
            .client
            .abci_query(Some(path.to_string()), request.encode_to_vec(), None, false)
            .await?;
        if res.code.is_err() {
            return Err(ProcessError::Query {
                code: res.code.value(),
                codespace: res.codespace,
                log: res.log,
            });
        }
        R::decode(res.value.as_slice()).map_err(protocol)
    }

    pub async fn query<Q: Message, R: Message + Default>(
        &self,
        path: &str,
        request: Q,
    ) -> Result<R> {
        self.bounded(self.query_inner(path, request)).await
    }

    async fn account_inner(&self, address: &AccountId) -> Result<BaseAccount> {
        let res: QueryAccountResponse = self
            .query_inner(
                "/cosmos.auth.v1beta1.Query/Account",
                QueryAccountRequest {
                    address: address.to_string(),
                },
            )
            .await?;
        let any = res.account.ok_or_else(|| protocol("missing account"))?;
        if any.type_url != "/cosmos.auth.v1beta1.BaseAccount" {
            return Err(protocol(
                "unsupported signing account type (expected BaseAccount)",
            ));
        }
        let account = BaseAccount::decode(any.value.as_slice()).map_err(protocol)?;
        if account.address != address.as_ref() {
            return Err(protocol(
                "account query returned a different signer address",
            ));
        }
        Ok(account)
    }

    pub async fn account(&self, address: &AccountId) -> Result<BaseAccount> {
        self.bounded(self.account_inner(address)).await
    }

    /// Single submission. One enclosing timeout covers status, key derivation,
    /// account lookup, simulation, signing, connection and committed response.
    /// An ambiguous failure is returned to the caller, never retried.
    pub async fn transact<F>(&self, key: &SigningKey, message: F) -> Result<Receipt>
    where
        F: FnOnce(&AccountId) -> Result<Any>,
    {
        let mut hash = None;
        let operation = async {
            self.status_inner().await?;
            let (signer, address) = key.derive(&self.cfg.prefix)?;
            let msgs = vec![message(&address)?];
            let account = self.account_inner(&address).await?;
            let tx_bytes =
                signing::simulation_bytes(msgs.clone(), account.sequence, &self.cfg.denom)?;
            #[allow(deprecated)]
            let simulation = SimulateRequest { tx: None, tx_bytes };
            let simulation: SimulateResponse = self
                .query_inner("/cosmos.tx.v1beta1.Service/Simulate", simulation)
                .await
                .map_err(|err| match err {
                    ProcessError::Query {
                        code,
                        codespace,
                        log,
                    } => TxError::Simulation {
                        code,
                        codespace,
                        log,
                    }
                    .into(),
                    other => other,
                })?;
            let gas = simulation
                .gas_info
                .ok_or_else(|| protocol("simulation omitted gas_info"))?;
            let fee = signing::fee(gas.gas_used, &self.cfg)?;
            let bytes = signing::sign(
                &signer,
                msgs,
                account.account_number,
                account.sequence,
                fee,
                &self.cfg.chain_id,
            )?;
            let expected = transaction_hash(&bytes);
            hash = Some(expected.to_string());
            let response = self.client.broadcast_tx_commit(bytes).await.map_err(|e| {
                ProcessError::Broadcast {
                    hash: expected.to_string(),
                    message: e.to_string(),
                }
            })?;
            committed(response, expected)
        };
        match timeout(self.operation_timeout, operation).await {
            Ok(result) => result,
            Err(_) => Err(ProcessError::Timeout { hash }),
        }
    }

    pub async fn poll_for_n_blocks(
        &self,
        n: u64,
        budget: Duration,
        first_block: bool,
    ) -> Result<()> {
        timeout(budget, async {
            let mut status = self.status_inner().await?;
            while first_block && status.sync_info.latest_block_height.value() == 0 {
                sleep(POLL_INTERVAL).await;
                status = self.status_inner().await?;
            }
            let target = status
                .sync_info
                .latest_block_height
                .value()
                .checked_add(n)
                .ok_or_else(|| protocol("block target overflow"))?;
            while status.sync_info.latest_block_height.value() < target {
                sleep(POLL_INTERVAL).await;
                status = self.status_inner().await?;
            }
            Ok(())
        })
        .await
        .map_err(|_| ProcessError::Timeout { hash: None })?
    }

    pub async fn poll_for_n_secs(&self, n: u64, budget: Duration) -> Result<()> {
        timeout(budget, async {
            let start = self
                .status_inner()
                .await?
                .sync_info
                .latest_block_time
                .unix_timestamp();
            let target = start
                .checked_add(n.try_into().map_err(protocol)?)
                .ok_or_else(|| protocol("block time target overflow"))?;
            while self
                .status_inner()
                .await?
                .sync_info
                .latest_block_time
                .unix_timestamp()
                < target
            {
                sleep(POLL_INTERVAL).await;
            }
            Ok(())
        })
        .await
        .map_err(|_| ProcessError::Timeout { hash: None })?
    }
}

pub(crate) fn transaction_hash(bytes: &[u8]) -> Hash {
    Hash::Sha256(Sha256::digest(bytes).into())
}

fn committed(res: endpoint::broadcast::tx_commit::Response, expected: Hash) -> Result<Receipt> {
    if res.hash != expected {
        return Err(protocol(
            "committed response hash differs from the signed transaction",
        ));
    }
    if res.check_tx.code.is_err() {
        return Err(TxError::CheckTx {
            code: res.check_tx.code.value(),
            codespace: res.check_tx.codespace,
            log: res.check_tx.log,
            hash: expected.to_string(),
        }
        .into());
    }
    if res.height.value() == 0 {
        return Err(protocol("response has no positive committed height"));
    }
    let result = res.tx_result;
    let receipt = Receipt {
        hash: expected.to_string(),
        height: res.height.value(),
        code: result.code.value(),
        codespace: result.codespace,
        log: result.log,
        gas_used: result.gas_used.try_into().map_err(protocol)?,
        gas_wanted: result.gas_wanted.try_into().map_err(protocol)?,
        // ExecTxResult's Bytes deserializer retains the RPC JSON base64 text.
        // Decode only the data field; 0.37 event attributes are already strings.
        data: STANDARD.decode(result.data).map_err(protocol)?,
        events: result.events,
    };
    if receipt.code != 0 {
        return Err(TxError::Execution {
            code: receipt.code,
            codespace: receipt.codespace.clone(),
            log: receipt.log.clone(),
            hash: receipt.hash.clone(),
            response: Box::new(receipt),
        }
        .into());
    }
    Ok(receipt)
}
