import { DepositInfo, Duration, Threshold, Uint128 } from "./shared-types";

export type ExecuteMsg = ({
propose: {
/**
 * A description of the proposal.
 */
description: string
/**
 * Optionally, a proposal may have a different expiration than the one that would be set by the `max_voting_period` in the governance module's config.
 */
latest?: (Expiration | null)
/**
 * The messages that should be executed in response to this proposal passing.
 */
msgs: CosmosMsgFor_Empty[]
/**
 * The title of the proposal.
 */
title: string
[k: string]: unknown
}
} | {
vote: {
/**
 * The ID of the proposal to vote on.
 */
proposal_id: number
/**
 * The senders position on the proposal.
 */
vote: Vote
[k: string]: unknown
}
} | {
execute: {
/**
 * The ID of the proposal to execute.
 */
proposal_id: number
[k: string]: unknown
}
} | {
close: {
/**
 * The ID of the proposal to close.
 */
proposal_id: number
[k: string]: unknown
}
} | {
update_config: {
/**
 * The address if tge DAO that this governance module is associated with.
 */
dao: string
/**
 * Information about the deposit required to make a proposal. None if no deposit, Some otherwise.
 */
deposit_info?: (DepositInfo | null)
/**
 * The default maximum amount of time a proposal may be voted on before expiring. This will only apply to proposals created after the config update.
 */
max_voting_period: Duration
/**
 * If set to true only members may execute passed proposals. Otherwise, any address may execute a passed proposal. Applies to all outstanding and future proposals.
 */
only_members_execute: boolean
/**
 * The new proposal passing threshold. This will only apply to proposals created after the config update.
 */
threshold: Threshold
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
/**
 * Binary is a wrapper around Vec<u8> to add base64 de/serialization with serde. It also adds some helper methods to help encode inline.
 * 
 * This is only needed as serde-json-{core,wasm} has a horrible encoding for Vec<u8>
 */
export type Binary = string
export type Vote = ("yes" | "no" | "abstain")

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
