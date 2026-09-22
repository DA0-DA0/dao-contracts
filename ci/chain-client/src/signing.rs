use cosmrs::proto::traits::Message;
use cosmrs::{crypto::secp256k1, proto::cosmos::tx::v1beta1::TxRaw, tx, AccountId, Any, Coin};

use crate::{
    config::ChainConfig,
    error::{protocol, ProcessError, Result},
};

// Preserve the previous envelopes, including their legacy memo strings: changing
// these alters transaction size and gas, even if the contract message is equal.
pub(crate) const SIMULATION_MEMO: &str = "cosm-client memo";
pub(crate) const TRANSACTION_MEMO: &str = "Made with cosm-tome client";

#[derive(Clone)]
pub enum Key {
    Mnemonic(String),
}

impl std::fmt::Debug for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Mnemonic([REDACTED])")
    }
}

#[derive(Clone, Debug)]
pub struct SigningKey {
    pub name: String,
    pub key: Key,
    pub derivation_path: String,
}

impl SigningKey {
    pub fn address(&self, prefix: &str) -> Result<AccountId> {
        Ok(self.derive(prefix)?.1)
    }

    pub(crate) fn derive(&self, prefix: &str) -> Result<(secp256k1::SigningKey, AccountId)> {
        let Key::Mnemonic(phrase) = &self.key;
        let mnemonic = bip32::Mnemonic::new(phrase, bip32::Language::English)
            .map_err(|_| ProcessError::Signing("invalid BIP39 mnemonic".into()))?;
        let path = self
            .derivation_path
            .parse::<bip32::DerivationPath>()
            .map_err(|_| ProcessError::Signing("invalid derivation path".into()))?;
        let key = secp256k1::SigningKey::derive_from_path(mnemonic.to_seed(""), &path)
            .map_err(|_| ProcessError::Signing("key derivation failed".into()))?;
        let address = key
            .public_key()
            .account_id(prefix)
            .map_err(|_| ProcessError::Signing("invalid account prefix".into()))?;
        Ok((key, address))
    }
}

pub(crate) fn simulation_bytes(msgs: Vec<Any>, sequence: u64, denom: &str) -> Result<Vec<u8>> {
    let body = tx::Body::new(msgs, SIMULATION_MEMO, 0u32);
    let fee = tx::Fee::from_amount_and_gas(
        Coin {
            denom: denom.parse().map_err(protocol)?,
            amount: 0,
        },
        0u64,
    );
    let auth = tx::SignerInfo::single_direct(None, sequence).auth_info(fee);
    Ok(TxRaw {
        body_bytes: body.into_bytes().map_err(protocol)?,
        auth_info_bytes: auth.into_bytes().map_err(protocol)?,
        signatures: vec![vec![]],
    }
    .encode_to_vec())
}

pub(crate) fn fee(gas_used: u64, cfg: &ChainConfig) -> Result<tx::Fee> {
    let gas = (gas_used as f64 * cfg.gas_adjustment).ceil();
    let amount = (gas * cfg.gas_price).ceil();
    // Avoid saturating float-to-integer casts silently changing fees.
    if !gas.is_finite()
        || !amount.is_finite()
        || gas < 0.0
        || amount < 0.0
        || gas >= u64::MAX as f64
        || amount >= u64::MAX as f64
    {
        return Err(protocol("simulated fee is outside the supported u64 range"));
    }
    Ok(tx::Fee::from_amount_and_gas(
        Coin {
            denom: cfg.denom.parse().map_err(protocol)?,
            amount: (amount as u64).into(),
        },
        gas as u64,
    ))
}

pub(crate) fn sign(
    key: &secp256k1::SigningKey,
    msgs: Vec<Any>,
    account_number: u64,
    sequence: u64,
    fee: tx::Fee,
    chain_id: &str,
) -> Result<Vec<u8>> {
    let body = tx::Body::new(msgs, TRANSACTION_MEMO, 0u32);
    let auth = tx::SignerInfo::single_direct(Some(key.public_key()), sequence).auth_info(fee);
    let doc = tx::SignDoc::new(
        &body,
        &auth,
        &chain_id.parse().map_err(protocol)?,
        account_number,
    )
    .map_err(protocol)?;
    doc.sign(key)
        .map_err(protocol)?
        .to_bytes()
        .map_err(protocol)
}
