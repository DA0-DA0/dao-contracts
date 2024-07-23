use cw_orch::{anyhow, prelude::*};

use crate::{
    dao::TokenFactorySuite,
    tests::{ADMIN, PREFIX},
};

#[test]
fn test_tokenfactory() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let admin = mock.addr_make(ADMIN);
    let app = TokenFactorySuite::deploy_on(mock.clone(), admin.clone())?;

    mock.next_block().unwrap();
    Ok(())
}
