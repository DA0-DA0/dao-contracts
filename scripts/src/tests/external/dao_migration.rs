use cw_orch::{anyhow, prelude::*};

use crate::{
    tests::{ADMIN, PREFIX},
    DaoMigrationSuite,
};

#[test]
fn test_dao_migration() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let admin = mock.addr_make(ADMIN);
    let app = DaoMigrationSuite::deploy_on(mock.clone(), admin.clone())?;
    mock.next_block().unwrap();
    Ok(())
}
