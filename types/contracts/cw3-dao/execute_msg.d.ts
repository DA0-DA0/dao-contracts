import { Binary, Duration, Threshold, Uint128 } from "./shared-types";

export type ExecuteMsg = ({
propose: Propose
} | {
vote: {
proposal_id: number
vote: Vote
[k: string]: unknown
}
} | {
execute: {
proposal_id: number
[k: string]: unknown
}
} | {
close: {
proposal_id: number
[k: string]: unknown
}
} | {
update_config: {
description: string
max_voting_period: Duration
name: string
proposal_deposit_amount: Uint128
proposal_deposit_token_address: string
refund_failed_proposals?: (boolean | null)
threshold: Threshold
[k: string]: unknown
}
} | {
update_cw20_token_list: {
to_add: Addr[]
to_remove: Addr[]
[k: string]: unknown
}
})
/**
 * Expiration represents a point in time when some event happens. It can compare with a BlockInfo and will return is_expired() == true once the condition is hit (and for every block in the future)
 */
export type Expiration = ({
at_height: number
} | {
at_time: Timestamp
} | {
never: {
[k: string]: unknown
}
})
/**
 * A point in time in nanosecond precision.
 * 
 * This type can represent times from 1970-01-01T00:00:00Z to 2554-07-21T23:34:33Z.
 * 
 * ## Examples
 * 
 * ``` # use cosmwasm_std::Timestamp; let ts = Timestamp::from_nanos(1_000_000_202); assert_eq!(ts.nanos(), 1_000_000_202); assert_eq!(ts.seconds(), 1); assert_eq!(ts.subsec_nanos(), 202);
 * 
 * let ts = ts.plus_seconds(2); assert_eq!(ts.nanos(), 3_000_000_202); assert_eq!(ts.seconds(), 3); assert_eq!(ts.subsec_nanos(), 202); ```
 */
export type Timestamp = Uint64
/**
 * A thin wrapper around u64 that is using strings for JSON encoding/decoding, such that the full u64 range can be used for clients that convert JSON numbers to floats, like JavaScript and jq.
 * 
 * # Examples
 * 
 * Use `from` to create instances of this and `u64` to get the value out:
 * 
 * ``` # use cosmwasm_std::Uint64; let a = Uint64::from(42u64); assert_eq!(a.u64(), 42);
 * 
 * let b = Uint64::from(70u32); assert_eq!(b.u64(), 70); ```
 */
export type Uint64 = string
export type CosmosMsgFor_Empty = ({
bank: BankMsg
} | {
custom: Empty
} | {
staking: StakingMsg
} | {
distribution: DistributionMsg
} | {
wasm: WasmMsg
})
/**
 * The message types of the bank module.
 * 
 * See https://github.com/cosmos/cosmos-sdk/blob/v0.40.0/proto/cosmos/bank/v1beta1/tx.proto
 */
export type BankMsg = ({
send: {
amount: Coin[]
to_address: string
[k: string]: unknown
}
} | {
burn: {
amount: Coin[]
[k: string]: unknown
}
})
/**
 * The message types of the staking module.
 * 
 * See https://github.com/cosmos/cosmos-sdk/blob/v0.40.0/proto/cosmos/staking/v1beta1/tx.proto
 */
export type StakingMsg = ({
delegate: {
amount: Coin
validator: string
[k: string]: unknown
}
} | {
undelegate: {
amount: Coin
validator: string
[k: string]: unknown
}
} | {
redelegate: {
amount: Coin
dst_validator: string
src_validator: string
[k: string]: unknown
}
})
/**
 * The message types of the distribution module.
 * 
 * See https://github.com/cosmos/cosmos-sdk/blob/v0.42.4/proto/cosmos/distribution/v1beta1/tx.proto
 */
export type DistributionMsg = ({
set_withdraw_address: {
/**
 * The `withdraw_address`
 */
address: string
[k: string]: unknown
}
} | {
withdraw_delegator_reward: {
/**
 * The `validator_address`
 */
validator: string
[k: string]: unknown
}
})
/**
 * The message types of the wasm module.
 * 
 * See https://github.com/CosmWasm/wasmd/blob/v0.14.0/x/wasm/internal/types/tx.proto
 */
export type WasmMsg = ({
execute: {
contract_addr: string
funds: Coin[]
/**
 * msg is the json-encoded ExecuteMsg struct (as raw Binary)
 */
msg: Binary
[k: string]: unknown
}
} | {
instantiate: {
admin?: (string | null)
code_id: number
funds: Coin[]
/**
 * A human-readbale label for the contract
 */
label: string
/**
 * msg is the JSON-encoded InstantiateMsg struct (as raw Binary)
 */
msg: Binary
[k: string]: unknown
}
} | {
migrate: {
contract_addr: string
/**
 * msg is the json-encoded MigrateMsg struct that will be passed to the new code
 */
msg: Binary
/**
 * the code_id of the new logic to place in the given contract
 */
new_code_id: number
[k: string]: unknown
}
} | {
update_admin: {
admin: string
contract_addr: string
[k: string]: unknown
}
} | {
clear_admin: {
contract_addr: string
[k: string]: unknown
}
})
export type Vote = ("yes" | "no" | "abstain" | "veto")
/**
 * A human readable address.
 * 
 * In Cosmos, this is typically bech32 encoded. But for multi-chain smart contracts no assumptions should be made other than being UTF-8 encoded and of reasonable length.
 * 
 * This type represents a validated address. It can be created in the following ways 1. Use `Addr::unchecked(input)` 2. Use `let checked: Addr = deps.api.addr_validate(input)?` 3. Use `let checked: Addr = deps.api.addr_humanize(canonical_addr)?` 4. Deserialize from JSON. This must only be done from JSON that was validated before such as a contract's state. `Addr` must not be used in messages sent by the user because this would result in unvalidated instances.
 * 
 * This type is immutable. If you really need to mutate it (Really? Are you sure?), create a mutable copy using `let mut mutable = Addr::to_string()` and operate on that `String` instance.
 */
export type Addr = string

export interface Propose {
description: string
latest?: (Expiration | null)
msgs: CosmosMsgFor_Empty[]
title: string
[k: string]: unknown
}
export interface Coin {
amount: Uint128
denom: string
[k: string]: unknown
}
/**
 * An empty struct that serves as a placeholder in different places, such as contracts that don't set a custom message.
 * 
 * It is designed to be expressable in correct JSON and JSON Schema but contains no meaningful data. Previously we used enums without cases, but those cannot represented as valid JSON Schema (https://github.com/CosmWasm/cosmwasm/issues/451)
 */
export interface Empty {
[k: string]: unknown
}
