use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, OverflowError, OverflowOperation::Sub, StdError, Uint128};
use cw20::{Balance, Cw20CoinVerified};
use cw_utils::NativeBalance;
use std::convert::{TryFrom,TryInto};

use crate::error::GenericError;

#[cw_serde]
#[derive(Default)]
pub struct GenericBalance {
    pub native: Vec<Coin>,
    pub cw20: Vec<Cw20CoinVerified>,
}
impl GenericBalance {
    pub fn has_native(&self) -> bool {
        !self.native.is_empty()
    }
    pub fn has_cw20(&self) -> bool {
        !self.cw20.is_empty()
    }
    pub fn total_amount(&self) -> u128 {
        let native:Uint128=self.native.iter().map(|el|{
            el.amount
        }).sum();
        let cw20:Uint128=self.cw20.iter().map(|el|{
            el.amount
        }).sum();

        cw20.u128()+native.u128()
    }
}
impl From<Balance> for GenericBalance {
    fn from(balance: Balance) -> GenericBalance {
        match balance {
            Balance::Native(balance) => GenericBalance {
                native: balance.0,
                cw20: vec![],
            },
            Balance::Cw20(token) => GenericBalance {
                native: vec![],
                cw20: vec![token],
            },
        }
    }
}
impl Into<Option<Balance>> for GenericBalance {
    fn into(self) -> Option<Balance> {
        let res = if self.has_native() {
            Some(Balance::Native(NativeBalance(self.native)))
        } else if !self.cw20.is_empty() {
            Some(Balance::Cw20(self.cw20[0].clone()))
        } else {
            None
        };
        res
    }
}

impl GenericBalance {
    pub fn add_tokens(&mut self, add: Balance) {
        match add {
            Balance::Native(balance) => {
                for token in balance.0 {
                    let index = self.native.iter().enumerate().find_map(|(i, exist)| {
                        if exist.denom == token.denom {
                            Some(i)
                        } else {
                            None
                        }
                    });
                    match index {
                        Some(idx) => self.native[idx].amount += token.amount,
                        None => self.native.push(token),
                    }
                }
            }
            Balance::Cw20(token) => {
                let index = self.cw20.iter().enumerate().find_map(|(i, exist)| {
                    if exist.address == token.address {
                        Some(i)
                    } else {
                        None
                    }
                });
                match index {
                    Some(idx) => self.cw20[idx].amount += token.amount,
                    None => self.cw20.push(token),
                }
            }
        };
    }
    pub fn remove_tokens(&mut self, remove: Balance) {
        match remove {
            Balance::Native(balance) => {
                for token in balance.0 {
                    let index = self.native.iter().enumerate().find_map(|(i, exist)| {
                        if exist.denom == token.denom {
                            Some(i)
                        } else {
                            None
                        }
                    });
                    match index {
                        Some(idx) => self.native[idx].amount -= token.amount,
                        None => {
                            if let Some(ind) = index {
                                self.native.remove(ind);
                            }
                            ()
                        }
                    }
                }
            }
            Balance::Cw20(token) => {
                let index = self.cw20.iter().enumerate().find_map(|(i, exist)| {
                    if exist.address == token.address {
                        Some(i)
                    } else {
                        None
                    }
                });
                match index {
                    Some(idx) => self.cw20[idx].amount -= token.amount,
                    None => {
                        if let Some(ind) = index {
                            self.cw20.remove(ind);
                        }
                        ()
                    }
                }
            }
        };
    }
}
pub trait FindAndMutate<'a, T, Rhs = &'a T>
where
    Self: IntoIterator<Item = T>,
{
    /// Safely adding and adding amount
    fn find_checked_add(&mut self, add: Rhs) -> Result<(), GenericError>;
    /// Safely finding and subtracting amount and remove it if it's zero
    fn find_checked_sub(&mut self, sub: Rhs) -> Result<(), GenericError>;
}
pub trait BalancesOperations<'a, T, Rhs> {
    fn checked_add_coins(&mut self, add: Rhs) -> Result<(), GenericError>;
    fn checked_sub_coins(&mut self, sub: Rhs) -> Result<(), GenericError>;
}
pub trait GenericBalances {
    fn add_tokens(&mut self, add: Balance);
    fn minus_tokens(&mut self, minus: Balance);
}
impl<'a, T, Rhs> BalancesOperations<'a, T, Rhs> for Vec<T>
where
    Rhs: IntoIterator<Item = &'a T>,
    Self: FindAndMutate<'a, T>,
    T: 'a,
{
    fn checked_add_coins(&mut self, add: Rhs) -> Result<(), GenericError> {
        for add_token in add {
            self.find_checked_add(add_token)?;
        }
        Ok(())
    }

    fn checked_sub_coins(&mut self, sub: Rhs) -> Result<(), GenericError> {
        for sub_token in sub {
            self.find_checked_sub(sub_token)?;
        }
        Ok(())
    }
}
impl FindAndMutate<'_, Coin> for Vec<Coin> {
    fn find_checked_add(&mut self, add: &Coin) -> Result<(), GenericError> {
        let token = self.iter_mut().find(|exist| exist.denom == add.denom);
        match token {
            Some(exist) => {
                exist.amount = exist
                    .amount
                    .checked_add(add.amount)
                    .map_err(StdError::overflow)?
            }
            None => self.push(add.clone()),
        }
        Ok(())
    }

    fn find_checked_sub(&mut self, sub: &Coin) -> Result<(), GenericError> {
        let coin = self.iter().position(|exist| exist.denom == sub.denom);
        match coin {
            Some(exist) => {
                match self[exist].amount.cmp(&sub.amount) {
                    std::cmp::Ordering::Less => {
                        return Err(GenericError::Std(StdError::overflow(OverflowError::new(
                            Sub,
                            self[exist].amount,
                            sub.amount,
                        ))))
                    }
                    std::cmp::Ordering::Equal => {
                        self.swap_remove(exist);
                    }
                    std::cmp::Ordering::Greater => self[exist].amount -= sub.amount,
                };
                Ok(())
            }
            None => Err(GenericError::EmptyBalance {}),
        }
    }
}

impl FindAndMutate<'_, Cw20CoinVerified> for Vec<Cw20CoinVerified> {
    fn find_checked_add(&mut self, add: &Cw20CoinVerified) -> Result<(), GenericError> {
        let token = self.iter_mut().find(|exist| exist.address == add.address);
        match token {
            Some(exist) => {
                exist.amount = exist
                    .amount
                    .checked_add(add.amount)
                    .map_err(StdError::overflow)?
            }
            None => self.push(add.clone()),
        }
        Ok(())
    }

    fn find_checked_sub(&mut self, sub: &Cw20CoinVerified) -> Result<(), GenericError> {
        let coin_p = self.iter().position(|exist| exist.address == sub.address);
        match coin_p {
            Some(exist) => {
                match self[exist].amount.cmp(&sub.amount) {
                    std::cmp::Ordering::Less => {
                        return Err(GenericError::Std(StdError::overflow(OverflowError::new(
                            Sub,
                            self[exist].amount,
                            sub.amount,
                        ))))
                    }
                    std::cmp::Ordering::Equal => {
                        self.swap_remove(exist);
                    }
                    std::cmp::Ordering::Greater => self[exist].amount -= sub.amount,
                };

                Ok(())
            }
            None => Err(GenericError::EmptyBalance {}),
        }
    }
}

impl GenericBalance {
    pub fn checked_add_native(&mut self, add: &[Coin]) -> Result<(), GenericError> {
        self.native.checked_add_coins(add)
    }

    pub fn checked_add_cw20(&mut self, add: &[Cw20CoinVerified]) -> Result<(), GenericError> {
        self.cw20.checked_add_coins(add)
    }

    pub fn checked_sub_native(&mut self, sub: &[Coin]) -> Result<(), GenericError> {
        self.native.checked_sub_coins(sub)
    }

    pub fn checked_sub_cw20(&mut self, sub: &[Cw20CoinVerified]) -> Result<(), GenericError> {
        self.cw20.checked_sub_coins(sub)
    }

    pub fn checked_sub_generic(&mut self, sub: &GenericBalance) -> Result<(), GenericError> {
        self.checked_sub_native(&sub.native)?;
        self.checked_sub_cw20(&sub.cw20)
    }
}
