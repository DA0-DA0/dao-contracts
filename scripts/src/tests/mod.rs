use cw_orch::{
    anyhow::{self},
    prelude::*,
};
use dao_voting_cw4::msg::InstantiateMsg as Cw4VotingInitMsg;

use crate::DaoDao;

mod daos;
// mod deploy;
mod distribution;
mod external;
mod gauges;
mod propose;
mod staking;
mod voting;

pub(crate) const PREFIX: &str = "mock";
pub(crate) const ADMIN: &str = "admin";
// pub(crate) const DENOM: &str = "juno";
// pub(crate) const DAO1: &str = "dao1";

#[test]
fn test_dao_suite() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let admin = mock.addr_make(ADMIN);
    let _app = DaoDao::deploy_on(mock.clone(), admin.clone())?;
    mock.next_block().unwrap();
    Ok(())
}
