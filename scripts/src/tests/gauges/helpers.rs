use cosmwasm_std::Decimal;

pub const EPOCH: u64 = 60 * 60 * 24 * 7;
pub const RESET_EPOCH: u64 = 30 * 86_400;

pub fn simple_vote(
    voter: &str,
    option: &str,
    percentage: u64,
    cast: impl Into<Option<u64>>,
) -> gauge_orchestrator::msg::VoteInfo {
    gauge_orchestrator::msg::VoteInfo {
        voter: voter.to_string(),
        votes: vec![gauge_orchestrator::state::Vote {
            option: option.to_string(),
            weight: Decimal::percent(percentage),
        }],
        cast: cast.into(),
    }
}

pub fn multi_vote(
    voter: &str,
    votes: &[(&str, u64)],
    cast: impl Into<Option<u64>>,
) -> gauge_orchestrator::msg::VoteInfo {
    let votes = votes
        .iter()
        .map(|(opt, percentage)| gauge_orchestrator::state::Vote {
            option: opt.to_string(),
            weight: Decimal::percent(*percentage),
        })
        .collect();
    gauge_orchestrator::msg::VoteInfo {
        voter: voter.to_string(),
        votes,
        cast: cast.into(),
    }
}
