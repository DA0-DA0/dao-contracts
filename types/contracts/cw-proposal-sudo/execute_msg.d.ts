export type ExecuteMsg = {
execute: {
msgs: CosmosMsgFor_Empty[]
[k: string]: unknown
}
}
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
 * A thin wrapper around u128 that is using strings for JSON encoding/decoding, such that the full u128 range can be used for clients that convert JSON numbers to floats, like JavaScript and jq.
 * 
 * # Examples
 * 
 * Use `from` to create instances of this and `u128` to get the value out:
 * 
 * ``` # use cosmwasm_std::Uint128; let a = Uint128::from(123u128); assert_eq!(a.u128(), 123);
 * 
 * let b = Uint128::from(42u64); assert_eq!(b.u128(), 42);
 * 
 * let c = Uint128::from(70u32); assert_eq!(c.u128(), 70); ```
 */
export type Uint128 = string
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
