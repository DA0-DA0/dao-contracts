use crate::{abc::CurveType, boot::CwAbc};
use boot_core::{BootUpload, Mock};
use cosmwasm_std::{Addr, Uint128};

use crate::testing::prelude::*;

type AResult = anyhow::Result<()>; // alias for Result<(), anyhow::Error>

// TODO: we need to make a PR to token factory bindings for the CustomHandler so that messages will actually execute
#[test]
fn instantiate() -> AResult {
    let sender = Addr::unchecked(TEST_CREATOR);
    let chain = Mock::new(&sender)?;

    let abc = CwAbc::new("cw:abc", chain);
    abc.upload()?;

    let curve_type = CurveType::SquareRoot {
        slope: Uint128::new(1),
        scale: 1,
    };

    let _init_msg = default_instantiate_msg(5u8, 5u8, curve_type);
    // abc.instantiate(&init_msg, None, None)?;
    //
    // let expected_config = msg::CurveInfoResponse {
    //     reserve: Default::default(),
    //     supply: Default::default(),
    //     funding: Default::default(),
    //     spot_price: Default::default(),
    //     reserve_denom: "".to_string(),
    // };
    //
    // let actual_config = abc.curve_info()?;
    //
    // assert_that!(&actual_config).is_equal_to(&expected_config);
    Ok(())
}
