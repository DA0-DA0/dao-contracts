import { PaymentInfo, Uint128 } from "./shared-types";

export type ExecuteMsg = ({
receive: Cw20ReceiveMsg
} | {
register_name: {
name: string
[k: string]: unknown
}
} | {
update_config: {
new_admin?: (string | null)
new_payment_info?: (PaymentInfo | null)
[k: string]: unknown
}
} | {
reserve: {
name: string
[k: string]: unknown
}
} | {
transfer_reservation: {
dao: string
name: string
[k: string]: unknown
}
} | {
revoke: {
name: string
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
