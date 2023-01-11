use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

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
/// call `get_positive_col` to get the index, starting at zero,
/// of the first column with only positive, non-zero values. By
/// construction, there will only ever be one such column.
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

    pub fn positive_col_and_margin(&self) -> Option<(usize, Uint128)> {
        let n = self.n;
        'cols: for col in 0..n {
            let mut smallest_margin = Uint128::MAX;
            for row in 0..n {
                if row != col {
                    if let Cell::Positive(p) = self.get((col, row)) {
                        if p < smallest_margin {
                            smallest_margin = p;
                        }
                    } else {
                        continue 'cols;
                    }
                }
            }
            return Some((col, smallest_margin));
        }
        None
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

    fn new_lm(n: usize) -> M {
        M {
            cells: vec![Cell::default(); n * (n - 1) / 2],
            n,
        }
    }

    #[test]
    fn test_internal_representation() {
        let mut lm = new_lm(4);
        lm.increment((1, 0), Uint128::new(1));
        lm.increment((2, 0), Uint128::new(2));
        lm.increment((3, 0), Uint128::new(3));
        lm.increment((2, 1), Uint128::new(4));
        lm.increment((3, 1), Uint128::new(5));
        lm.increment((3, 2), Uint128::new(6));

        assert_eq!(
            lm.cells,
            (1..7)
                .map(|i| Cell::Positive(Uint128::new(i)))
                .collect::<Vec<Cell>>()
        )
    }

    #[test]
    fn test_index() {
        let n = 3;
        let lm = new_lm(n);

        let i = lm.index((1, 0));
        assert_eq!(i, 0);
    }

    #[test]
    fn test_create() {
        let n = 10;
        let lm = new_lm(n);

        // we now expect this to be a 10 / 10 square.
        for x in 0..n {
            for y in 0..n {
                if x != y {
                    let c = lm.get((x, y));
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
        let mut lm = new_lm(n);

        for x in 0..n {
            for y in 0..n {
                if y < x {
                    lm.increment((y, x), Uint128::one())
                }
            }
        }

        for x in 0..n {
            for y in 0..n {
                if y > x {
                    assert_eq!(lm.get((x, y)), Cell::Positive(Uint128::one()));
                    assert_eq!(lm.get((y, x)), Cell::Negative(Uint128::one()));
                }
            }
        }
    }

    #[test]
    fn test_first_positive_col() {
        let n = 8;
        let mut lm = new_lm(n);

        for y in 0..n {
            if y != 2 {
                lm.increment((2, y), Uint128::one())
            }
        }

        let (row, margin) = lm.positive_col_and_margin().unwrap();
        assert_eq!(row, 2);
        assert_eq!(margin, Uint128::one());
    }
}
