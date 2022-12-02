use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, OverflowError, OverflowOperation::Sub, StdError};
use cw20::{Balance, Cw20CoinVerified, Cw20ReceiveMsg};
use cw_utils::NativeBalance;

use crate::error::GenericError;
#[cw_serde]
#[derive(Default)]
pub struct WrapedBalance(Balance);

impl WrapedBalance {
    pub fn is_native(&self) -> bool {
        matches!(self.0, Balance::Native(_))
    }
    pub fn is_cw20(&self) -> bool {
        matches!(self.0, Balance::Cw20(_))
    }
    pub fn native(&self) -> Option<&Coin> {
        match &self.0 {
            Balance::Native(nb) => nb.0.get(0),
            Balance::Cw20(_) => None,
        }
    }
    pub fn cw20(&self) -> Option<&Cw20CoinVerified> {
        match &self.0 {
            Balance::Native(_) => None,
            Balance::Cw20(cw20) => Some(&cw20),
        }
    }
    pub fn new_native(native: Coin) -> Self {
        WrapedBalance(Balance::Native(NativeBalance(vec![native])))
    }
    pub fn new_cw20(cw20: Cw20CoinVerified) -> Self {
        WrapedBalance(Balance::Cw20(cw20))
    }
    pub fn amount(&self) -> u128 {
        match &self.0 {
            Balance::Native(nb) => {
                if let Some(it) = nb.0.get(0) {
                    it.amount.u128()
                } else {
                    0
                }
            }
            Balance::Cw20(cw20) => cw20.amount.u128(),
        }
    }
}
impl From<Balance> for WrapedBalance {
    fn from(balance: Balance) -> WrapedBalance {
        WrapedBalance(balance)
    }
}
impl From<Cw20ReceiveMsg> for WrapedBalance {
    fn from(msg: Cw20ReceiveMsg) -> WrapedBalance {
        WrapedBalance::new_cw20(Cw20CoinVerified {
            address: Addr::unchecked(msg.sender),
            amount: msg.amount,
        })
    }
}
impl Into<Balance> for WrapedBalance {
    fn into(self) -> Balance {
        self.0
    }
}
impl Into<Option<Balance>> for WrapedBalance {
    fn into(self) -> Option<Balance> {
        Some(self.0)
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
impl WrapedBalance {
    pub fn checked_add_native(&mut self, add: &[Coin]) -> Result<(), GenericError> {
        if let Balance::Native(mut nb) = self.0.clone() {
            return nb.0.checked_add_coins(add);
        }
        return Err(GenericError::EmptyBalance {  });
    }

    pub fn checked_add_cw20(&mut self, add: &[Cw20CoinVerified]) -> Result<(), GenericError> {
        if let Balance::Cw20(cw20) = self.0.clone() {
            return vec![cw20].checked_add_coins(add);
        }
        return Err(GenericError::EmptyBalance {  });
    }

    pub fn checked_sub_native(&mut self, sub: &[Coin]) -> Result<(), GenericError> {
        if let Balance::Native(mut nb) = self.0.clone() {
            return nb.0.checked_sub_coins(sub);
        }
        return Err(GenericError::EmptyBalance {  });
    }

    pub fn checked_sub_cw20(&mut self, sub: &[Cw20CoinVerified]) -> Result<(), GenericError> {
        if let Balance::Cw20(cw20) = self.0.clone() {
            return vec![cw20].checked_sub_coins(sub);
        }
        return Err(GenericError::EmptyBalance {  });
    }
}
