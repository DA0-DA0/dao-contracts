use cosmwasm_std::Deps;
use cw_denom::{DenomError, UncheckedDenom};

use crate::{msg::AssetUnchecked, state::Asset};

impl AssetUnchecked {
    pub fn into_checked(self, deps: Deps) -> Result<Asset, DenomError> {
        Ok(Asset {
            denom: self.denom.into_checked(deps)?,
            amount: self.amount,
        })
    }

    pub fn new_native(denom: &str, amount: u128) -> Self {
        Self {
            denom: UncheckedDenom::Native(denom.to_owned()),
            amount: amount.into(),
        }
    }

    pub fn new_cw20(denom: &str, amount: u128) -> Self {
        Self {
            denom: UncheckedDenom::Cw20(denom.to_owned()),
            amount: amount.into(),
        }
    }
}
