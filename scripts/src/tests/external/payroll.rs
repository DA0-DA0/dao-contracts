use cw_orch::{anyhow, prelude::*};

use crate::{
    external::PayrollSuite,
    tests::{ADMIN, PREFIX},
};

#[test]
fn test_payroll() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let admin = mock.addr_make(ADMIN);
    let app = PayrollSuite::deploy_on(mock.clone(), admin.clone())?;
    mock.next_block().unwrap();
    Ok(())
}
