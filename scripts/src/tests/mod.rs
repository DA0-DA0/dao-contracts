use cosmwasm_std::{to_json_binary, Event, Uint128};
use cw4::Member;
use cw_orch::{
    anyhow::{self},
    prelude::*,
};
use dao_interface::{msg::InstantiateMsg, state::ModuleInstantiateInfo};
use dao_voting::{pre_propose::PreProposeInfo, threshold::Threshold};
use dao_voting_cw4::msg::{GroupContract, InstantiateMsg as Cw4VotingInitMsg};
use gauges::suite::DaoDaoCw4Gauge;

use crate::DaoDao;

mod deploy;
mod distribution;
mod external;
mod daos;
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
