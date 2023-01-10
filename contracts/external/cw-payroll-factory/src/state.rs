use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};

/// Temporarily holds recipient and instantiator
pub const TMP_CONTRACT_INFO: Item<(Addr, Addr)> = Item::new("tmp_contract_info");

#[cw_serde]
pub struct VestingContract {
    pub contract: String,
    pub owner: String,
    pub recipient: String,
}

pub struct TokenIndexes<'a> {
    pub owner: MultiIndex<'a, String, VestingContract, String>,
    pub recipient: MultiIndex<'a, String, VestingContract, String>,
}

impl<'a> IndexList<VestingContract> for TokenIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<VestingContract>> + '_> {
        let v: Vec<&dyn Index<VestingContract>> = vec![&self.owner, &self.recipient];
        Box::new(v.into_iter())
    }
}

pub fn vesting_contracts<'a>() -> IndexedMap<'a, &'a str, VestingContract, TokenIndexes<'a>> {
    let indexes = TokenIndexes {
        owner: MultiIndex::new(
            |_pk: &[u8], d: &VestingContract| d.owner.clone(),
            "vesting_contracts",
            "vesting_contracts__owner",
        ),
        recipient: MultiIndex::new(
            |_pk: &[u8], d: &VestingContract| d.recipient.clone(),
            "vesting_contracts",
            "vesting_contracts__recipient",
        ),
    };
    IndexedMap::new("vesting_contracts", indexes)
}
