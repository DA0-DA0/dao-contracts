pub mod abc;
pub(crate) mod commands;
pub mod contract;
pub mod curves;
mod error;
#[cfg(test)]
mod integration;
pub mod msg;
mod queries;
pub mod state;

// Integrationg tests using an actual chain binary, requires
// the "test-tube" feature to be enabled
// cargo test --features test-tube
#[cfg(test)]
#[cfg(feature = "test-tube")]
mod testtube;

pub use crate::error::ContractError;

// TODO do we still want these?
#[cfg(test)]
pub(crate) mod testing {
    use crate::abc::{
        ClosedConfig, CommonsPhaseConfig, CurveType, HatchConfig, MinMax, OpenConfig, ReserveToken,
        SupplyToken,
    };
    use crate::msg::InstantiateMsg;
    use cosmwasm_std::{
        testing::{mock_env, mock_info, MockApi, MockQuerier, MockStorage},
        Decimal, OwnedDeps, Uint128,
    };

    use crate::contract;
    use crate::contract::CwAbcResult;
    use cosmwasm_std::DepsMut;
    use std::marker::PhantomData;
    use token_bindings::{Metadata, TokenFactoryQuery};

    pub(crate) mod prelude {
        pub use super::{
            default_instantiate_msg, default_supply_metadata, mock_tf_dependencies, TEST_CREATOR,
            TEST_RESERVE_DENOM, TEST_SUPPLY_DENOM, _TEST_BUYER, _TEST_INVESTOR,
        };
        pub use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
        pub use speculoos::prelude::*;
    }

    pub const TEST_RESERVE_DENOM: &str = "satoshi";
    pub const TEST_CREATOR: &str = "creator";
    pub const _TEST_INVESTOR: &str = "investor";
    pub const _TEST_BUYER: &str = "buyer";

    pub const TEST_SUPPLY_DENOM: &str = "subdenom";

    pub fn default_supply_metadata() -> Metadata {
        Metadata {
            name: Some("Bonded".to_string()),
            symbol: Some("EPOXY".to_string()),
            description: None,
            denom_units: vec![],
            base: None,
            display: None,
        }
    }

    pub fn default_instantiate_msg(
        decimals: u8,
        reserve_decimals: u8,
        curve_type: CurveType,
    ) -> InstantiateMsg {
        InstantiateMsg {
            supply: SupplyToken {
                subdenom: TEST_SUPPLY_DENOM.to_string(),
                metadata: default_supply_metadata(),
                decimals,
            },
            reserve: ReserveToken {
                denom: TEST_RESERVE_DENOM.to_string(),
                decimals: reserve_decimals,
            },
            phase_config: CommonsPhaseConfig {
                hatch: HatchConfig {
                    initial_raise: MinMax {
                        min: Uint128::one(),
                        max: Uint128::from(1000000u128),
                    },
                    initial_price: Uint128::one(),
                    initial_allocation_ratio: Decimal::percent(10u64),
                    exit_tax: Decimal::zero(),
                },
                open: OpenConfig {
                    allocation_percentage: Decimal::percent(10u64),
                    exit_tax: Decimal::percent(10u64),
                },
                closed: ClosedConfig {},
            },
            hatcher_allowlist: None,
            curve_type,
        }
    }

    pub fn mock_init(deps: DepsMut<TokenFactoryQuery>, init_msg: InstantiateMsg) -> CwAbcResult {
        let info = mock_info(TEST_CREATOR, &[]);
        let env = mock_env();
        contract::instantiate(deps, env, info, init_msg)
    }

    pub fn mock_tf_dependencies(
    ) -> OwnedDeps<MockStorage, MockApi, MockQuerier<TokenFactoryQuery>, TokenFactoryQuery> {
        OwnedDeps {
            storage: MockStorage::default(),
            api: MockApi::default(),
            querier: MockQuerier::<TokenFactoryQuery>::new(&[]),
            custom_query_type: PhantomData::<TokenFactoryQuery>,
        }
    }
}
