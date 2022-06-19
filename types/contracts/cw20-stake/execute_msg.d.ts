import { Duration, Uint128 } from "./shared-types";

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
update_config: {
duration?: (Duration | null)
manager?: (string | null)
owner?: (string | null)
[k: string]: unknown
}
} | {
add_hook: {
addr: string
[k: string]: unknown
}
} | {
remove_hook: {
addr: string
[k: string]: unknown
}
})
/**
 * Binary is a wrapper around Vec<u8> to add base64 de/serialization with serde. It also adds some helper methods to help encode inline.
 * 
 * This is only needed as serde-json-{core,wasm} has a horrible encoding for Vec<u8>
 */
export type Binary = string

/**
 * Cw20ReceiveMsg should be de/serialized under `Receive()` variant in a ExecuteMsg
 */
export interface Cw20ReceiveMsg {
amount: Uint128
msg: Binary
sender: string
[k: string]: unknown
}
