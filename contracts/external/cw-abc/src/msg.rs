use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Uint128};

use crate::abc::{CurveType, HatchConfig, ReserveToken, SupplyToken};
use crate::ContractError;

#[cw_serde]
pub struct InstantiateMsg {
    // Supply token information
    pub supply: SupplyToken,

    // Reserve token information
    pub reserve: ReserveToken,

    // Curve type for this contract
    pub curve_type: CurveType,

    // Hatch configuration information
    pub hatch_config: HatchConfig,
}

impl InstantiateMsg {
    /// Validate the instantiate message
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.supply.subdenom.is_empty() {
            return Err(ContractError::SupplyTokenError("Token subdenom must not be empty.".to_string()));
        }

        self.hatch_config.validate()
    }
}


#[cw_serde]
pub enum ExecuteMsg {
    /// Buy will attempt to purchase as many supply tokens as possible.
    /// You must send only reserve tokens in that message
    Buy {},
    /// Implements CW20. Burn is a base message to destroy tokens forever
    Burn { amount: Uint128 },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the reserve and supply quantities, as well as the spot price to buy 1 token
    #[returns(CurveInfoResponse)]
    CurveInfo {},
}

#[cw_serde]
pub struct CurveInfoResponse {
    // how many reserve tokens have been received
    pub reserve: Uint128,
    // how many supply tokens have been issued
    pub supply: Uint128,
    pub spot_price: Decimal,
    pub reserve_denom: String,
}
