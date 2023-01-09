use cosmwasm_schema::cw_serde;
use cosmwasm_std::{StdResult, Storage, Uint128};
use cw_storage_plus::Item;

use crate::cell::Cell;

/// Singleton for loading and saving M matrixes.
pub(crate) struct M<'a> {
    lm: Item<'a, LM>,
}

/// (L)oaded M
///
/// A matrix for which M[x, y] == -M[y, x].
///
/// Indicies may be incremented or decremented. When index (x, y) is
/// incremented, index (y, x) is decremented with the reverse applying
/// when decrementing an index.
///
/// Invariant: indicies along the diagonal must never be incremented
/// or decremented.
///
/// The contents of the matrix are not avaliable, though consumers may
/// call `get_positive_row` to get the index, starting at zero,
/// of the first row with only positive, non-zero values. By
/// construction, there will only ever be one such row.
#[cw_serde]
pub(crate) struct LM {
    cells: Vec<Cell>,
    // we store `n` instead of re-computing on use as:
    //
    // cells.len = n * (n - 1) / 2
    //
    // which has a square root if you try and extract n from it.
    n: usize,
}

impl<'a> M<'a> {
    pub const fn new(key: &'a str) -> Self {
        Self { lm: Item::new(key) }
    }

    pub fn init(&self, storage: &mut dyn Storage, n: usize) -> StdResult<()> {
        // example 4x4 M:
        //
        //  \  1  2  3
        // -1  \  4  5
        // -2 -4  \  6
        // -3 -5 -6  \
        //
        // `cells` stores all the values in the upper diagonal of
        // M. there are `N-1 + N-2 .. + 1` or `N (N-1) / 2` cells.
        self.save(
            storage,
            LM {
                cells: vec![Cell::default(); n * (n - 1) / 2],
                n,
            },
        )
    }

    pub fn load(&self, storage: &dyn Storage) -> StdResult<LM> {
        self.lm.load(storage)
    }

    pub fn save(&self, storage: &mut dyn Storage, lm: LM) -> StdResult<()> {
        self.lm.save(storage, &lm)
    }
}

impl LM {
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
        // `self.cells` was not diagonalized. `(y + 1) * y / 2` is the
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

    pub fn get_positive_row(&self) -> Option<usize> {
        let n = self.n;
        'rows: for row in 0..n {
            for col in 0..n {
                if row != col && !self.get((col, row)).is_positive() {
                    continue 'rows;
                }
            }
            return Some(row);
        }
        None
    }
}

#[cfg(test)]
mod test_lm {
    use super::*;

    fn new_lm(n: usize) -> LM {
        LM {
            cells: vec![Cell::default(); n * (n - 1) / 2],
            n,
        }
    }

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
    fn debug_lm(lm: &LM) {
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

    #[test]
    fn test_internal_representation() {
        let mut lm = new_lm(4);
        lm.increment((1, 0), Uint128::new(1));
        lm.increment((2, 0), Uint128::new(2));
        lm.increment((3, 0), Uint128::new(3));
        lm.increment((2, 1), Uint128::new(4));
        lm.increment((3, 1), Uint128::new(5));
        lm.increment((3, 2), Uint128::new(6));

        debug_lm(&lm);

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
                    assert!(lm.get((x, y)).is_positive());
                    assert_eq!(lm.get((x, y)), Cell::Positive(Uint128::one()));
                    assert_eq!(lm.get((y, x)), Cell::Negative(Uint128::one()));
                }
            }
        }
    }

    #[test]
    fn test_first_positive() {
        let n = 8;
        let mut lm = new_lm(n);

        for x in 0..n {
            if x != 2 {
                lm.increment((x, 2), Uint128::one())
            }
        }

        let row = lm.get_positive_row();
        assert_eq!(row, Some(2));
    }
}

#[cfg(test)]
mod test_m {
    use cosmwasm_std::testing::mock_dependencies;

    use super::*;

    #[test]
    fn test_m() {
        let mut deps = mock_dependencies();
        let m = M::new("m");
        m.init(deps.as_mut().storage, 10).unwrap();
        let lm = m.load(deps.as_ref().storage).unwrap();
        m.save(deps.as_mut().storage, lm).unwrap();
    }
}
