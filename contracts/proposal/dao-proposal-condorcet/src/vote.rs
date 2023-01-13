use std::ops::Index;

use cosmwasm_schema::cw_serde;
use thiserror::Error;

#[cw_serde]
pub struct Vote(Vec<u32>);

impl Vote {
    pub(crate) fn new(vote: Vec<u32>, candidates: usize) -> Result<Self, VoteError> {
        if vote.len() != candidates {
            return Err(VoteError::LenMissmatch {
                got: vote.len() as u32,
                expected: candidates as u32,
            });
        }
        let mut seen = vec![];
        for v in vote {
            if v as usize >= candidates {
                return Err(VoteError::InvalidCandidate { candidate: v });
            }
            if seen.contains(&v) {
                return Err(VoteError::DuplicateCandidate { candidate: v });
            }
            seen.push(v);
        }
        Ok(Vote(seen))
    }

    pub fn iter(&self) -> std::slice::Iter<'_, u32> {
        self.0.iter()
    }
}

impl Index<usize> for Vote {
    type Output = u32;

    fn index(&self, index: usize) -> &Self::Output {
        self.0.index(index)
    }
}

#[derive(Error, Debug)]
pub enum VoteError {
    #[error("candidate ({candidate}) appears in ballot more than once")]
    DuplicateCandidate { candidate: u32 },

    #[error("no such candidate ({candidate})")]
    InvalidCandidate { candidate: u32 },

    #[error("ballot has wrong number of candidates. got ({got}) expected ({expected})")]
    LenMissmatch { got: u32, expected: u32 },
}
