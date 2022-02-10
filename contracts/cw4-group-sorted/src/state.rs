use cosmwasm_std::{Addr, DepsMut, StdResult, Storage};
use cw4::MemberDiff;
use cw_controllers::{Admin, Hooks};
use cw_storage_plus::{Item, SnapshotMap, Strategy};

use crate::query::Member;

/// The admin of the contract. This field will be uninitialized in the
/// event that the contract has no admin.
pub const ADMIN: Admin = Admin::new("admin");

/// The hooks registered with the contract.
pub const HOOKS: Hooks = Hooks::new("cw4-hooks");

/// The total weight of members.
pub const TOTAL: Item<u64> = Item::new(cw4::TOTAL_KEY);

/// Map of member addresses to weight. Used for querying member
/// weights at a given height.
pub const MEMBERS: SnapshotMap<&Addr, u64> = SnapshotMap::new(
    cw4::MEMBERS_KEY,
    cw4::MEMBERS_CHECKPOINTS,
    cw4::MEMBERS_CHANGELOG,
    Strategy::EveryBlock,
);

pub fn initialize_members(deps: DepsMut, members: Vec<Member>, height: u64) -> StdResult<()> {
    TOTAL.save(deps.storage, &0)?;
    for member in members.into_iter() {
        let Member { addr, weight } = member;
        let addr = deps.api.addr_validate(addr.as_str())?;
        update_member(deps.storage, addr, weight, height)?;
    }
    Ok(())
}

/// Updates information about a member.
pub fn update_member(
    storage: &mut dyn Storage,
    addr: Addr,
    weight: u64,
    height: u64,
) -> StdResult<MemberDiff> {
    let old_weight = MEMBERS.may_load(storage, &addr)?;

    MEMBERS.save(storage, &addr, &weight, height)?;
    TOTAL.update(storage, |total_weight| -> StdResult<_> {
        // Order of addition / subtraction important here to avoid
        // overflow.
        Ok((total_weight + weight) - old_weight.unwrap_or_default())
    })?;

    Ok(MemberDiff {
        key: addr.into_string(),
        old: old_weight,
        new: Some(weight),
    })
}

pub fn remove_member(
    storage: &mut dyn Storage,
    member: Addr,
    height: u64,
) -> StdResult<MemberDiff> {
    let old = MEMBERS.may_load(storage, &member)?;
    let old_weight = if let Some(weight) = old {
        TOTAL.update(storage, |total_weight| -> StdResult<_> {
            Ok(total_weight - weight)
        })?;
        Some(weight)
    } else {
        None
    };

    MEMBERS.remove(storage, &member, height)?;
    Ok(MemberDiff {
        key: member.into_string(),
        old: old_weight,
        new: None,
    })
}

pub fn list_members_sorted(storage: &dyn Storage) -> StdResult<Vec<Member>> {
    let members: Result<Vec<_>, _> = MEMBERS
        .range(storage, None, None, cosmwasm_std::Order::Ascending)
        .collect();
    let mut members: Vec<Member> = members?
        .into_iter()
        .map(|(addr, weight)| Member { weight, addr })
        .collect();
    members.sort();
    Ok(members.into_iter().rev().collect())
}
