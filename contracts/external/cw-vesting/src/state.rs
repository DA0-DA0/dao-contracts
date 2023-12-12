use cosmwasm_schema::cw_serde;
use cw_storage_plus::Item;

use crate::vesting::Payment;

#[cw_serde]
pub struct DaoStakingLimits {
    // TODO max limits need to be kept track of
    // The maximum amount of tokens that can be staked by this contract.
    // pub max: Option<Uint128>,
    // Staking contracts this contract is allowed to stake with
    pub staking_contract_allowlist: Vec<String>,
}

pub const DAO_STAKING_LIMITS: Item<DaoStakingLimits> = Item::new("dao_staking_limits");
pub const PAYMENT: Payment = Payment::new("vesting", "staked", "validator", "cardinality");
pub const UNBONDING_DURATION_SECONDS: Item<u64> = Item::new("ubs");
