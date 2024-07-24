use cw_orch::{anyhow, prelude::*};

use crate::{
    gauges::GaugeSuite,
    tests::{ADMIN, DAO1, DENOM, PREFIX},
};

#[test]
fn test_gauge_suite() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let admin = mock.addr_make(ADMIN);
    let _app = GaugeSuite::deploy_on(mock.clone(), admin.clone())?;
    mock.next_block().unwrap();
    Ok(())
}
