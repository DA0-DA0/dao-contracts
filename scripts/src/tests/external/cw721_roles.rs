use cw_orch::{anyhow, prelude::*};

use crate::{
    tests::{ADMIN, PREFIX},
    Cw721RolesSuite,
};

#[test]
fn test_cw721_roles() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let admin = mock.addr_make(ADMIN);
    let _app = Cw721RolesSuite::deploy_on(mock.clone(), admin.clone())?;
    mock.next_block().unwrap();
    Ok(())
}
