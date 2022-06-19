import { Addr, Uint128 } from "./shared-types";

export type ExecuteMsg = ({
stake_change_hook: StakeChangedHookMsg
} | {
claim: {
[k: string]: unknown
}
} | {
receive: Cw20ReceiveMsg
} | {
fund: {
[k: string]: unknown
}
} | {
update_reward_duration: {
new_duration: number
[k: string]: unknown
}
} | {
update_owner: {
new_owner?: (string | null)
[k: string]: unknown
}
} | {
update_manager: {
new_manager?: (string | null)
[k: string]: unknown
}
})
export type StakeChangedHookMsg = ({
stake: {
addr: Addr
amount: Uint128
[k: string]: unknown
}
} | {
unstake: {
addr: Addr
amount: Uint128
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
