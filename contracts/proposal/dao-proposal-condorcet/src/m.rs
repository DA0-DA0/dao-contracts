use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Uint128, Uint256};

use crate::cell::Cell;

/// M
///
/// A NxN matrix for which M[x, y] == -M[y, x].
///
/// Indicies may be incremented or decremented. When index (x, y) is
/// incremented, index (y, x) is decremented with the reverse applying
/// when decrementing an index.
///
/// Invariant: indicies along the diagonal must never be incremented
/// or decremented.
///
/// The contents of the matrix are not avaliable, though consumers may
/// call the `stats` method which returns information about the first
/// positive column, or the column closest to containing all positive
/// values.
#[cw_serde]
pub(crate) struct M {
    cells: Vec<Cell>,
    // the n in NxN. stored instead of re-computing on use as:
    //
    // cells.len = n * (n - 1) / 2
    //
    // which has a square root if you try and extract n from it.
    pub n: u32,
}

pub(crate) enum Stats {
    PositiveColumn {
        /// Index of the column that is positive.
        col: u32,
        /// Smallest value in the column's distance from zero.
        min_margin: Uint128,
    },
    NoPositiveColumn {
        /// False if there exists a column where:
        ///
        /// distance_from_positivity(col) <= power_outstanding * (N-1)
        /// && max_negative_magnitude(col) < power_outstanding
        no_winnable_columns: bool,
    },
}

impl M {
    pub fn new(n: u32) -> Self {
        M {
            // example 4x4 M:
            //
            //  \  1  2  3
            // -1  \  4  5
            // -2 -4  \  6
            // -3 -5 -6  \
            //
            // `cells` stores all the values in the upper diagonal of
            // M. there are `N-1 + N-2 .. + 1` or `N (N-1) / 2` cells.
            cells: vec![Cell::default(); (n * (n - 1) / 2) as usize],
            n,
        }
    }

    /// Gets the index in `self.cells` that corresponds to the index (x, y) in M.
    ///
    /// Invariant: x > y as otherwise the upper diagonal
    /// (`self.cells`) will not contain an entry for the index.
    fn index(&self, (x, y): (u32, u32)) -> u32 {
        let n = self.n;
        // the start of the row in `self.cells`.
        //
        // the easiest way to conceptualize this is
        // geometrically. `y*n` is the area of the whole matrix up to
        // row `y`, and thus the start index of the row if
        // `self.cells` was not diagonalized [1]. `(y + 1) * y / 2` is the
        // area of the space that is not in the upper diagonal.
        //
        // whole_area - area_of_non_diagonal = area_of_diagonal
        //
        // because we're in the land of discrete math and things are
        // zero-indexed, area_of_diagonal == start_of_row.
        let row = y * n - (y + 1) * y / 2;
        // we know that x > y, so to get the index in `self.cells` we
        // offset x by the distance of x from the line x = y (the
        // diagonal), as `self.cells`' first index corresponds to the
        // first item in that row of the upper diagonal.
        let offset = x - (y + 1);
        row + offset
    }

    pub(crate) fn get(&self, (x, y): (u32, u32)) -> Cell {
        if x < y {
            self.get((y, x)).invert()
        } else {
            let i = self.index((x, y)) as usize;
            self.cells[i]
        }
    }

    pub fn increment(&mut self, (x, y): (u32, u32), amount: Uint128) {
        debug_assert!(x != y);
        if x < y {
            self.decrement((y, x), amount)
        } else {
            let i = self.index((x, y)) as usize;
            self.cells[i] = self.cells[i].increment(amount)
        }
    }

    pub fn decrement(&mut self, (x, y): (u32, u32), amount: Uint128) {
        debug_assert!(x != y);
        if x < y {
            self.increment((y, x), amount)
        } else {
            let i = self.index((x, y)) as usize;
            self.cells[i] = self.cells[i].decrement(amount)
        }
    }

    /// Computes statistics about M which are used to determine if a
    /// proposal has passed or may be rejected early.
    ///
    /// Code comments refer to this proof of conditions for early
    /// rejection:
    ///
    /// https://github.com/DA0-DA0/dao-contracts/wiki/Proofs-of-early-rejection-cases-for-Condorcet-proposals
    pub fn stats(&self, power_outstanding: Uint128) -> Stats {
        let n = self.n;
        let mut no_winnable_columns = true;
        for col in 0..n {
            let mut distance_from_positivity = Uint256::zero();
            let mut min_margin = Uint128::MAX;
            let mut max_negative = Uint128::zero();
            for row in 0..n {
                if row != col {
                    match self.get((col, row)) {
                        Cell::Positive(p) => {
                            if p < min_margin {
                                min_margin = p
                            }
                        }
                        Cell::Negative(v) => {
                            if v > max_negative {
                                max_negative = v;
                            }
                            distance_from_positivity += Uint256::from(v) + Uint256::one();
                        }
                        Cell::Zero => distance_from_positivity += Uint256::one(),
                    }
                }
            }
            if distance_from_positivity.is_zero() {
                // there is only ever one positive column, as the
                // symmetry of this matrix means that if there is a
                // positive column there is also a row with negative
                // values in every column except that one. so, we can
                // return early here.
                return Stats::PositiveColumn { col, min_margin };
            }

            // a column is winnable if both claim A and B are false (see proof)
            if distance_from_positivity <= power_outstanding.full_mul((self.n - 1) as u64) {
                // ^ claim_a = false
                if max_negative < power_outstanding {
                    // ^ claim_b = false
                    no_winnable_columns = false
                }
            }
        }
        Stats::NoPositiveColumn {
            no_winnable_columns,
        }
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;

    // prints out the LM in it's full matrix form. looks something
    // like this:
    //
    // ```
    //   \ -1  0  0  0  0  0  0
    //   1  \  1  1  1  1  1  1
    //   0 -1  \  0  0  0  0  0
    //   0 -1  0  \  0  0  0  0
    //   0 -1  0  0  \  0  0  0
    //   0 -1  0  0  0  \  0  0
    //   0 -1  0  0  0  0  \  0
    //   0 -1  0  0  0  0  0  \
    // ```
    #[allow(dead_code)]
    pub(crate) fn debug_lm(lm: &M) {
        for row in 0..lm.n {
            for col in 0..lm.n {
                if row == col {
                    eprint!("  \\");
                } else {
                    let c = lm.get((col, row));
                    match c {
                        Cell::Positive(p) => eprint!("  {p}"),
                        Cell::Zero => eprint!("  0"),
                        Cell::Negative(p) => eprint!(" -{p}"),
                    }
                }
            }
            eprintln!()
        }
    }
}

#[cfg(test)]
mod test_lm {
    use super::*;

    fn new_m(n: u32) -> M {
        M {
            cells: vec![Cell::default(); (n * (n - 1) / 2) as usize],
            n,
        }
    }

    #[test]
    fn test_internal_representation() {
        let mut m = new_m(4);
        m.increment((1, 0), Uint128::new(1));
        m.increment((2, 0), Uint128::new(2));
        m.increment((3, 0), Uint128::new(3));
        m.increment((2, 1), Uint128::new(4));
        m.increment((3, 1), Uint128::new(5));
        m.increment((3, 2), Uint128::new(6));

        assert_eq!(
            m.cells,
            (1..7)
                .map(|i| Cell::Positive(Uint128::new(i)))
                .collect::<Vec<Cell>>()
        )
    }

    #[test]
    fn test_index() {
        let n = 3;
        let m = new_m(n);

        let i = m.index((1, 0));
        assert_eq!(i, 0);
    }

    #[test]
    fn test_create() {
        let n = 10;
        let m = new_m(n);

        // we now expect this to be a 10 / 10 square.
        for x in 0..n {
            for y in 0..n {
                if x != y {
                    let c = m.get((x, y));
                    assert!(matches!(c, Cell::Zero))
                }
            }
        }
    }

    #[test]
    fn test_incrementation() {
        // decrement all values for which y < x. all values for which
        // y > x should become positive.
        let n = 11;
        let mut m = new_m(n);

        for x in 0..n {
            for y in 0..n {
                if y < x {
                    m.increment((y, x), Uint128::one())
                }
            }
        }

        for x in 0..n {
            for y in 0..n {
                if y > x {
                    assert_eq!(m.get((x, y)), Cell::Positive(Uint128::one()));
                    assert_eq!(m.get((y, x)), Cell::Negative(Uint128::one()));
                }
            }
        }
    }

    #[test]
    fn test_stats_positive_column() {
        let n = 8;
        let mut m = new_m(n);

        for y in 0..n {
            if y != 2 {
                m.increment((2, y), Uint128::one())
            }
        }

        match m.stats(Uint128::zero()) {
            Stats::PositiveColumn { col, min_margin } => {
                assert_eq!((col, min_margin), (2, Uint128::one()))
            }
            Stats::NoPositiveColumn { .. } => panic!("expected a positive column"),
        }
    }

    #[test]
    fn test_stats_no_positive_column() {
        let n = 8;
        let mut m = new_m(n);

        match m.stats(Uint128::new(n as u128)) {
            Stats::PositiveColumn { .. } => panic!("expected no positive columns"),
            Stats::NoPositiveColumn {
                no_winnable_columns,
            } => {
                // false because there exists a row that may be
                // flipped with N voting power remaining, and the
                // largest negative in that row is less than the power
                // outstanding.
                assert!(!no_winnable_columns)
            }
        }

        for i in 1..n {
            m.decrement((i - 1, i), Uint128::new(2));
        }

        // last row here has no negative value and a distance from
        // positivity of n - 2.
        //
        //  \  2  0  0  0  0  0  0
        // -2  \  2  0  0  0  0  0
        //  0 -2  \  2  0  0  0  0
        //  0  0 -2  \  2  0  0  0
        //  0  0  0 -2  \  2  0  0
        //  0  0  0  0 -2  \  2  0
        //  0  0  0  0  0 -2  \  2
        //  0  0  0  0  0  0 -2  \

        match m.stats(Uint128::new((n - 3) as u128)) {
            Stats::PositiveColumn { .. } => panic!("expected no positive columns"),
            Stats::NoPositiveColumn {
                no_winnable_columns,
            } => {
                // last column can be flipped.
                assert!(!no_winnable_columns)
            }
        }

        m.decrement((n - 1, n - 3), Uint128::new(1));

        //  \  2  0  0  0  0  0  0
        // -2  \  2  0  0  0  0  0
        //  0 -2  \  2  0  0  0  0
        //  0  0 -2  \  2  0  0  0
        //  0  0  0 -2  \  2  0  0
        //  0  0  0  0 -2  \  2 -1
        //  0  0  0  0  0 -2  \  2
        //  0  0  0  0  0  1 -2  \

        match m.stats(Uint128::new(7)) {
            Stats::PositiveColumn { .. } => panic!("expected no positive columns"),
            Stats::NoPositiveColumn {
                no_winnable_columns,
            } => {
                // there is enough voting power to flip columns n-1 and n-2.
                assert!(!no_winnable_columns)
            }
        }

        match m.stats(Uint128::new(1)) {
            Stats::PositiveColumn { .. } => panic!("expected no positive columns"),
            Stats::NoPositiveColumn {
                no_winnable_columns,
            } => {
                // there is not enough voting power to flip any columns.
                assert!(no_winnable_columns)
            }
        }
    }
}
