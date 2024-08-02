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
cw-msg-filter = "0.1.0"

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

let messages: Vec<CosmosMsg> = vec![
    // Your messages here
];

match filter.check_messages(&messages) {
    Ok(()) => println!("All messages are valid!"),
    Err(e) => println!("Error: {}", e),
}

```

## API

### `MsgFilter`

The main struct for configuring message filtering rules.

#### Fields:
- `allowed_msgs`: Optional list of `AllowedMsg` specifying which messages are permitted.
- `max_msg_count`: Optional maximum number of messages allowed.
- `spending_limits`: Optional list of `Coin` specifying spending limits per denomination.

### `AllowedMsg`

An enum specifying the types of allowed messages:

- `Exact(CosmosMsg)`: Allows an exact message.
- `GenericWasmExecuteMsg`: Allows Wasm execute messages matching specified criteria.
- `Type(MsgType)`: Allows messages of a specific type.

### `MsgType`

An enum representing different types of CosmWasm messages (e.g., `BankSend`, `WasmExecute`, etc.).
