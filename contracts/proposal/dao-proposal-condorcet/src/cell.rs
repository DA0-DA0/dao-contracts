use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

/// A small type for storing a integer that can store numbers in
/// [-2^128, 2^128]. `0` is considered neither positive, nor negative.
///
/// # Example
///
/// ```ignore
/// use cosmwasm_std::Uint128;
///
/// let c = Cell::Positive(Uint128::new(1));
/// let c = c.decrement(Uint128::new(2));
/// assert_eq!(c, Cell::Negative(Uint128::new(1)));
/// ```
#[cw_serde]
#[derive(Copy)]
pub(crate) enum Cell {
    Positive(Uint128),
    Zero,
    Negative(Uint128),
}

#[allow(clippy::comparison_chain)]
impl Cell {
    pub fn increment(self, amount: Uint128) -> Self {
        match self {
            Cell::Positive(n) => Cell::Positive(n + amount),
            Cell::Zero => Cell::Positive(amount),
            Cell::Negative(n) => {
                if amount == n {
                    Cell::Zero
                } else if amount > n {
                    Cell::Positive(amount - n)
                } else {
                    Cell::Negative(n - amount)
                }
            }
        }
    }

    pub fn decrement(self, amount: Uint128) -> Self {
        match self {
            Cell::Positive(n) => {
                if amount == n {
                    Cell::Zero
                } else if amount > n {
                    Cell::Negative(amount - n)
                } else {
                    Cell::Positive(n - amount)
                }
            }
            Cell::Zero => Cell::Negative(amount),
            Cell::Negative(n) => Cell::Negative(n + amount),
        }
    }

    pub fn invert(self) -> Self {
        match self {
            Cell::Positive(n) => Cell::Negative(n),
            Cell::Zero => Cell::Zero,
            Cell::Negative(n) => Cell::Positive(n),
        }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::Zero
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_increment_to_zero() {
        let mut cell = Cell::default();
        for i in 1..100 {
            cell = cell.increment(Uint128::new(i));
            cell = cell.decrement(Uint128::new(i));
        }
        assert_eq!(cell, Cell::Zero);
    }

    #[test]
    fn can_hold_max() {
        let cell = Cell::Positive(Uint128::MAX)
            .decrement(Uint128::MAX)
            .decrement(Uint128::MAX);
        assert_eq!(cell, Cell::Negative(Uint128::MAX))
    }
}
