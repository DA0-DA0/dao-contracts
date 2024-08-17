# CosmWasm Message Filter

## Overview

The CosmWasm Message Filter is a Rust library designed to filter and validate CosmWasm messages based on configurable criteria. It provides a flexible and robust way to ensure that only allowed messages are processed, enforcing spending limits and message count restrictions.

## Features

- Filter messages based on exact match, generic Wasm execute criteria, or message type
- Enforce maximum message count
- Apply spending limits across multiple denominations
- Detailed error reporting for invalid messages or exceeded limits

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
cw-msg-filter = "2.5.0"

```

## Usage

Here's a basic example of how to use the `MsgFilter`:

```rust
use cw_msg_filter::{MsgFilter, AllowedMsg, MsgType};
use cosmwasm_std::{CosmosMsg, coins};

let filter = MsgFilter {
    allowed_msgs: Some(vec![
        AllowedMsg::Type(MsgType::BankSend),
        AllowedMsg::GenericWasmExecuteMsg {
            contract: Some("allowed_contract".to_string()),
            key: Some("action".to_string()),
            funds: Some(coins(100, "utoken")),
        },
    ]),
    max_msg_count: Some(5),
    spending_limits: Some(coins(1000, "utoken")),
};

// Validate the configuration
match filter.validate_config() {
    Ok(()) => println!("Configuration is valid"),
    Err(e) => println!("Invalid configuration: {}", e),
}

let messages: Vec<CosmosMsg> = vec![
    // Your messages here
];

// Check if messages are allowed
match filter.check_messages(&messages) {
    Ok(()) => println!("All messages are valid!"),
    Err(e) => println!("Error: {}", e),
}

// Filter messages
match filter.filter_messages(&messages) {
    Ok(filtered) => println!("Filtered messages: {:?}", filtered),
    Err(e) => println!("Error while filtering: {}", e),
}
```

## API

### `MsgFilter`

The main struct for configuring message filtering rules.

#### Fields:

- `allowed_msgs`: Optional `Vec<AllowedMsg>` specifying which messages are permitted.
- `max_msg_count`: Optional `u8` setting the maximum number of messages allowed.
- `spending_limits`: Optional `Vec<Coin>` specifying spending limits per denomination.

#### Methods:

- `validate_config(&self) -> Result<(), ConfigValidationError>`: Validates the configuration of the `MsgFilter`.
- `check_messages(&self, messages: &[CosmosMsg]) -> Result<(), MsgFilterError>`: Checks if all messages are allowed according to the filter rules.
- `filter_messages<'a>(&self, messages: &'a [CosmosMsg]) -> Result<Vec<&'a CosmosMsg>, MsgFilterError>`: Returns a list of messages that meet the filter criteria.

### `AllowedMsg`

An enum specifying the types of allowed messages:

- `Exact(CosmosMsg)`: Allows an exact message.
- `GenericWasmExecuteMsg { contract: Option<String>, key: Option<String>, funds: Option<Vec<Coin>> }`: Allows Wasm execute messages matching specified criteria.
- `Type(MsgType)`: Allows messages of a specific type.

### `MsgType`

An enum representing different types of CosmWasm messages (e.g., `BankSend`, `WasmExecute`, etc.).

### `MsgFilterError`

An enum representing different types of errors that can occur during message filtering:

- `Std(StdError)`: Standard error from cosmwasm_std.
- `InvalidConfiguration`: Invalid filter configuration.
- `InvalidMsg`: Message not allowed by the filter.
- `SpendingLimitExceeded { denom: String, limit: Uint128, actual: Uint128 }`: Spending limit exceeded for a specific denomination.
- `TooManyMsgs { count: u8, max: u8 }`: Maximum message count exceeded.

### `ConfigValidationError`

An enum representing different types of configuration validation errors:

- `NoFilteringCriteria`: No filtering criteria specified.
- `InvalidMaxMsgCount(u8)`: Invalid maximum message count.
- `InvalidSpendingLimit { denom: String }`: Invalid spending limit for a specific denomination.
- `EmptyGenericWasmExecuteMsg`: Generic Wasm Execute Message has no criteria specified.
- `InvalidCoinAmount { denom: String }`: Invalid coin amount in funds for a specific denomination.
