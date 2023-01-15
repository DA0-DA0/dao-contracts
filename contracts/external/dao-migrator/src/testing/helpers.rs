use cosmwasm_std::Empty;
use cw_multi_test::Contract;
use dao_testing::contracts::{
    cw20_base_contract, cw20_stake_contract, cw20_staked_balances_voting_contract,
    v1_dao_core_contract, v1_proposal_single_contract,
};

pub(crate) const SENDER_ADDR: &str = "creator";

pub struct InitDaoDataV1 {
    pub proposal_code: Box<dyn Contract<Empty>>,
    pub core_code: Box<dyn Contract<Empty>>,
    pub cw20_code: Box<dyn Contract<Empty>>,
    pub cw20_stake_code: Box<dyn Contract<Empty>>,
    pub voting_code: Box<dyn Contract<Empty>>,
}

impl Default for InitDaoDataV1 {
    fn default() -> Self {
        InitDaoDataV1 {
            proposal_code: v1_proposal_single_contract(),
            core_code: v1_dao_core_contract(),
            cw20_code: cw20_base_contract(),
            cw20_stake_code: cw20_stake_contract(),
            voting_code: cw20_staked_balances_voting_contract(),
        }
    }
}
