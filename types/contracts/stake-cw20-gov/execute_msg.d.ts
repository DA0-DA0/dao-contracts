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
