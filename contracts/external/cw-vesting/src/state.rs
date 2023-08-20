use cw_storage_plus::Item;

use crate::vesting::Payment;

pub const PAYMENT: Payment = Payment::new("vesting", "staked", "validator", "cardinality");
pub const UNBONDING_DURATION_SECONDS: Item<u64> = Item::new("ubs");
