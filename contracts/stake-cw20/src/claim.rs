use cosmwasm_std::{Addr, BlockInfo, Deps, Order, StdResult, Storage, Uint128};
use cw_controllers::{Claim, ClaimsResponse};
use cw_storage_plus::{Item, Map, Prefix, PrefixBound};
use cw_utils::{Expiration, Scheduled};
use crate::ContractError;

// TODO: revisit design (split each claim on own key?)
pub struct Claims<'a>();

pub struct NewClaims<'a> {
    pub claims: Map<'a, (&'a Addr, Scheduled, u128), bool>,
    pub total_claims: Map<'a, &'a Addr, Uint128>,
    pub max_index: Option<u64>,
    pub current_index_size: Item<'a, u64>,
    pub total_claim_index: Map<'a, (u128, &'a Addr), bool>
}

impl<'a> NewClaims<'a> {
    pub const fn new(storage_key: &'a str, max_index: Option<u64>) -> Self {
        NewClaims{
            claims: Map::new(storage_key),
            total_claim_index: Map::new(storage_key + "_balance_index"),
            total_claims: Map::new(storage_key + "_total_claims"),
            current_index_size: Item::new(storage_key + "_current_index_size"),
            max_index,
        }
    }

    /// This creates a claim, such that the given address can claim an amount of tokens after
    /// the release date.
    pub fn create_claim(
        &self,
        storage: &mut dyn Storage,
        addr: &Addr,
        amount: Uint128,
        release_at: Scheduled,
    ) -> StdResult<()> {
        // add a claim to this user to get their tokens after the unbonding period
        self.claims.save(storage, (addr, release_at, amount.u128()), &true)?;

        // if max index is setup, index stake
        if let Some(max_index) = self.max_index {
            let total_claim = self.total_claims.update(storage, addr, |old| {
                match old {
                    None => Ok(amount),
                    Some(o) => Ok(amount + o)
                }
            })?;

            // get last indexed total_claim after current claim
            let last_claim  = self.total_claim_index
                .prefix_range(storage, Some(PrefixBound::exclusive(total_claim)), None, Order::Ascending)
                .map(|r| r.map(|((amount, addr), v)| (amount, addr)))
                .take(1 as usize)
                .collect::<StdResult<Vec<_>>>()?
                .first();

            match last_claim {
                // if not found means first claim
                None => {
                    self.total_claims.save(storage, addr, &amount)?;
                    self.total_claim_index.save(storage, (amount.u128(), addr), &true)?;
                }
                // if found, update total claim and index
                Some(_) => {
                    self.total_claims.save(storage, addr, &amount)?;
                    self.total_claim_index.save(storage, (amount.u128(), addr), &true)?;

                    let current_index_size = self.current_index_size.load(storage)?;
                    // remove last element if current_index_size > max_size
                    if current_index_size >= max_index {
                        let (amount, addr) = self.total_claim_index
                            .prefix_range(storage, None, None, Order::Ascending)
                            .map(|r| r.map(|((amount, addr), v)| (amount, addr)))
                            .take(1 as usize)
                            .collect::<StdResult<Vec<_>>>()?
                            .first()
                            .unwrap(); // we know there is item inside we can unwrap.

                        self.total_claim_index.remove(store, (*amount, addr))
                    }
                }
            }
        }

        Ok(())
    }

    /// This iterates over all mature claims for the address, and removes them, up to an optional cap.
    /// it removes the finished claims and returns the total amount of tokens to be released.
    pub fn claim_tokens(
        &self,
        storage: &mut dyn Storage,
        addr: &Addr,
        block: &BlockInfo,
        cap: Option<Uint128>,
    ) -> StdResult<Uint128> {
        /*
        self.0.update(storage, addr, |claim| -> StdResult<_> {
            let (_send, waiting): (Vec<_>, _) =
                claim.unwrap_or_default().iter().cloned().partition(|c| {
                    // if mature and we can pay fully, then include in _send
                    if c.release_at.is_expired(block) {
                        if let Some(limit) = cap {
                            if to_send + c.amount > limit {
                                return false;
                            }
                        }
                        // TODO: handle partial paying claims?
                        to_send += c.amount;
                        true
                    } else {
                        // not to send, leave in waiting and save again
                        false
                    }
                });
            Ok(waiting)
        })?;
         */

        self.claims.prefix_range(storage, None, None, Order::Descending)
            .filter(|&x| x.is_err() || x.unwrap())

        Ok(_)
    }
}
/*

impl<'a> Claims<'a> {
    pub const fn new(storage_key: &'a str) -> Self {
        Claims(Map::new(storage_key))
    }

    /// This creates a claim, such that the given address can claim an amount of tokens after
    /// the release date.
    pub fn create_claim(
        &self,
        storage: &mut dyn Storage,
        addr: &Addr,
        amount: Uint128,
        release_at: Expiration,
    ) -> StdResult<()> {
        // add a claim to this user to get their tokens after the unbonding period
        self.0.update(storage, addr, |old| -> StdResult<_> {
            let mut claims = old.unwrap_or_default();
            claims.push(Claim { amount, release_at });
            Ok(claims)
        })?;
        Ok(())
    }

    /// This iterates over all mature claims for the address, and removes them, up to an optional cap.
    /// it removes the finished claims and returns the total amount of tokens to be released.
    pub fn claim_tokens(
        &self,
        storage: &mut dyn Storage,
        addr: &Addr,
        block: &BlockInfo,
        cap: Option<Uint128>,
    ) -> StdResult<Uint128> {
        let mut to_send = Uint128::zero();
        self.0.update(storage, addr, |claim| -> StdResult<_> {
            let (_send, waiting): (Vec<_>, _) =
                claim.unwrap_or_default().iter().cloned().partition(|c| {
                    // if mature and we can pay fully, then include in _send
                    if c.release_at.is_expired(block) {
                        if let Some(limit) = cap {
                            if to_send + c.amount > limit {
                                return false;
                            }
                        }
                        // TODO: handle partial paying claims?
                        to_send += c.amount;
                        true
                    } else {
                        // not to send, leave in waiting and save again
                        false
                    }
                });
            Ok(waiting)
        })?;
        Ok(to_send)
    }

    pub fn query_claims(&self, deps: Deps, address: &Addr) -> StdResult<ClaimsResponse> {
        let claims = self.0.may_load(deps.storage, address)?.unwrap_or_default();
        Ok(ClaimsResponse { claims })
    }
}

 */