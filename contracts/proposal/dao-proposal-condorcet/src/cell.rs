use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

#[cw_serde]
#[derive(Copy)]
pub(crate) enum Cell {
    Positive(Uint128),
    Zero,
    Negative(Uint128),
}

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

    pub fn is_positive(&self) -> bool {
        matches!(self, Cell::Positive(_))
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::Zero
    }
}
