// Tests for the crate, using cw-multi-test
// Most coverage lives here
mod multitest;

// Integration tests using an actual chain binary, requires
// the "test-tube" feature to be enabled
// cargo test --features test-tube
#[cfg(test)]
#[cfg(feature = "test-tube")]
mod test_tube;
