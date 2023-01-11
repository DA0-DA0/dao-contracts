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
    pub n: usize,
}

pub(crate) enum Stats {
    PositiveColumn {
        /// Index of the column that is positive.
        col: usize,
        /// Smallest value in the column's distance from zero.
        min_margin: Uint128,
    },
    NoPositiveColumn {
        /// Smallest number required to flip a column positive. For
        /// example, given a column with values `[-1, 0, 1]`, the
        /// distance from positivity would be `3` as to become
        /// positive one would need to add two to index zero and one
        /// to index one, yielding `[1, 1, 1]`.
        ///
        /// This type needs to be larger than a u128 because `-2^128`
        /// is `2^128 + 1` away from being positive.
        min_col_distance_from_positivity: Uint256,
        /// The most negative value in the least negative column.
        max_negative_in_min_col: Uint128,
    },
}

impl M {
    pub fn new(n: usize) -> Self {
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
            cells: vec![Cell::default(); n * (n - 1) / 2],
            n,
        }
    }

    /// Gets the index in `self.cells` that corresponds to the index (x, y) in M.
    ///
    /// Invariant: x > y as otherwise the upper diagonal
    /// (`self.cells`) will not contain an entry for the index.
    fn index(&self, (x, y): (usize, usize)) -> usize {
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

    fn get(&self, (x, y): (usize, usize)) -> Cell {
        if x < y {
            self.get((y, x)).invert()
        } else {
            let i = self.index((x, y));
            self.cells[i]
        }
    }

    pub fn increment(&mut self, (x, y): (usize, usize), amount: Uint128) {
        debug_assert!(x != y);
        if x < y {
            self.decrement((y, x), amount)
        } else {
            let i = self.index((x, y));
            self.cells[i] = self.cells[i].increment(amount)
        }
    }

    pub fn decrement(&mut self, (x, y): (usize, usize), amount: Uint128) {
        debug_assert!(x != y);
        if x < y {
            self.increment((y, x), amount)
        } else {
            let i = self.index((x, y));
            self.cells[i] = self.cells[i].decrement(amount)
        }
    }

    pub fn stats(&self) -> Stats {
        let n = self.n;
        let mut min_col_distance_from_positivity = Uint256::MAX;
        let mut max_negative_in_min_col = Uint128::MAX;
        for col in 0..n {
            let mut distance_from_positivity = Uint256::zero();
            let mut min_margin = Uint128::MAX;
            let mut max_negative = Uint128::MAX;
            for row in 0..n {
                if row != col {
                    match self.get((col, row)) {
                        Cell::Positive(p) => {
                            if p < min_margin {
                                min_margin = p
                            }
                        }
                        Cell::Negative(n) => {
                            if n > max_negative {
                                max_negative = n;
                            }
                            distance_from_positivity += Uint256::from(n) + Uint256::one();
                        }
                        Cell::Zero => distance_from_positivity += Uint256::one(),
                    }
                }
            }
            if distance_from_positivity.is_zero() {
                return Stats::PositiveColumn { col, min_margin };
            }
            if distance_from_positivity < min_col_distance_from_positivity {
                min_col_distance_from_positivity = distance_from_positivity;
                max_negative_in_min_col = max_negative;
            }
        }
        Stats::NoPositiveColumn {
            min_col_distance_from_positivity,
            max_negative_in_min_col,
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
            eprintln!("")
        }
    }
}

#[cfg(test)]
mod test_lm {
    use super::*;

    fn new_m(n: usize) -> M {
        M {
            cells: vec![Cell::default(); n * (n - 1) / 2],
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

        match m.stats() {
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

        match m.stats() {
            Stats::PositiveColumn { .. } => panic!("expected no positive columns"),
            Stats::NoPositiveColumn {
                min_col_distance_from_positivity: min_distance_from_positivity,
            } => assert_eq!(min_distance_from_positivity, Uint256::from((n - 1) as u32)),
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

        match m.stats() {
            Stats::PositiveColumn { .. } => panic!("expected no positive columns"),
            Stats::NoPositiveColumn {
                min_col_distance_from_positivity: min_distance_from_positivity,
            } => assert_eq!(min_distance_from_positivity, Uint256::from((n - 2) as u32)),
        }
    }
}
