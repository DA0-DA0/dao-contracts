use cosmwasm_std::{
    testing::{mock_env, mock_info},
    Decimal, DepsMut, Response, Uint128,
};
use dao_interface::token::NewDenomMetadata;

use crate::contract;
use crate::msg::InstantiateMsg;
use crate::{
    abc::{
        ClosedConfig, CommonsPhaseConfig, CurveType, HatchConfig, MinMax, OpenConfig, ReserveToken,
        SupplyToken,
    },
    ContractError,
};

pub(crate) mod prelude {
    pub use super::{
        default_instantiate_msg, default_supply_metadata, TEST_CREATOR, TEST_RESERVE_DENOM,
        TEST_SUPPLY_DENOM, _TEST_BUYER, _TEST_INVESTOR,
    };
    pub use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
    pub use speculoos::prelude::*;
}

pub const TEST_RESERVE_DENOM: &str = "satoshi";
pub const TEST_CREATOR: &str = "creator";
pub const _TEST_INVESTOR: &str = "investor";
pub const _TEST_BUYER: &str = "buyer";

pub const TEST_SUPPLY_DENOM: &str = "subdenom";

pub fn default_supply_metadata() -> NewDenomMetadata {
    NewDenomMetadata {
        name: "Bonded".to_string(),
        symbol: "EPOXY".to_string(),
        description: "Forever".to_string(),
        display: "EPOXY".to_string(),
        additional_denom_units: None,
    }
}

pub fn default_instantiate_msg(
    decimals: u8,
    reserve_decimals: u8,
    curve_type: CurveType,
) -> InstantiateMsg {
    InstantiateMsg {
        fees_recipient: TEST_CREATOR.to_string(),
        token_issuer_code_id: 1,
        supply: SupplyToken {
            subdenom: TEST_SUPPLY_DENOM.to_string(),
            metadata: Some(default_supply_metadata()),
            decimals,
            max_supply: None,
        },
        reserve: ReserveToken {
            denom: TEST_RESERVE_DENOM.to_string(),
            decimals: reserve_decimals,
        },
        phase_config: CommonsPhaseConfig {
            hatch: HatchConfig {
                contribution_limits: MinMax {
                    min: Uint128::one(),
                    max: Uint128::from(1000000u128),
                },
                initial_raise: MinMax {
                    min: Uint128::one(),
                    max: Uint128::from(1000000u128),
                },
                entry_fee: Decimal::percent(10u64),
                exit_fee: Decimal::zero(),
            },
            open: OpenConfig {
                entry_fee: Decimal::percent(10u64),
                exit_fee: Decimal::percent(10u64),
            },
            closed: ClosedConfig {},
        },
        hatcher_allowlist: None,
        curve_type,
    }
}

pub fn mock_init(deps: DepsMut, init_msg: InstantiateMsg) -> Result<Response, ContractError> {
    let info = mock_info(TEST_CREATOR, &[]);
    let env = mock_env();
    contract::instantiate(deps, env, info, init_msg)
}
