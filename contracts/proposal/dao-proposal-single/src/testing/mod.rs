mod adversarial_tests;
mod do_votes;
mod execute;
mod instantiate;
// v1 migration tests — gated off; the v1 contract stack pins
// cosmwasm-std 1.5.5 and cannot be hosted by cw-multi-test 2.x.
#[cfg(any())]
mod migration_tests;
mod queries;
mod tests;

pub(crate) const CREATOR_ADDR: &str = "cosmwasm1h34lmpywh4upnjdg90cjf4j70aee6z8qqfspugamjp42e4q28kqs8s7vcp";
