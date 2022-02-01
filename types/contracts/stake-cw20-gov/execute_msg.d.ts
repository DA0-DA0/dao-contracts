import { Binary, Uint128 } from "./shared-types";

export type ExecuteMsg = ({
receive: Cw20ReceiveMsg
} | {
unstake: {
amount: Uint128
[k: string]: unknown
}
} | {
claim: {
[k: string]: unknown
}
} | {
delegate_votes: {
recipient: string
[k: string]: unknown
}
} | {
update_config: {
admin: Addr
duration?: (Duration | null)
[k: string]: unknown
}
})
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
/**
 * Duration is a delta of time. You can add it to a BlockInfo or Expiration to move that further in the future. Note that an height-based Duration and a time-based Expiration cannot be combined
 */
export type Duration = ({
height: number
} | {
time: number
})

/**
 * Cw20ReceiveMsg should be de/serialized under `Receive()` variant in a ExecuteMsg
 */
export interface Cw20ReceiveMsg {
amount: Uint128
msg: Binary
sender: string
[k: string]: unknown
}
