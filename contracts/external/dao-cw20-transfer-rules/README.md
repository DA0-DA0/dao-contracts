# dao-cw20-transfer-rules

[![dao-cw20-transfer-rules on crates.io](https://img.shields.io/crates/v/dao-cw20-transfer-rules.svg?logo=rust)](https://crates.io/crates/dao-cw20-transfer-rules)
[![docs.rs](https://img.shields.io/docsrs/dao-cw20-transfer-rules?logo=docsdotrs)](https://docs.rs/dao-cw20-transfer-rules/latest/cw_admin_factory/)

Enforces granular transfer rules on cw20 token transfer that can take into
account DAO membership. Addresses can optionally be allowed to send tokens,
receive tokens, or both, and a default can be set for DAO members with no
allowance specified. By default, no one can transfer.
