use cw_orch::{anyhow, prelude::*};

use crate::{
    external::TokenFactorySuite,
    tests::{ADMIN, PREFIX},
    VestingSuite,
};

#[test]
fn test_vesting() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let admin = mock.addr_make(ADMIN);
    let app = VestingSuite::deploy_on(mock.clone(), admin.clone())?;
    mock.next_block().unwrap();
    Ok(())
}
