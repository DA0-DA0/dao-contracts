import { ActiveThreshold, Uint128 } from "./shared-types";
export type TokenInfo = ({
existing: {
address: string
staking_contract: StakingInfo
[k: string]: unknown
}
} | {
new: {
code_id: number
decimals: number
initial_balances: Cw20Coin[]
initial_dao_balance?: (Uint128 | null)
label: string
marketing?: (InstantiateMarketingInfo | null)
name: string
staking_code_id: number
symbol: string
unstaking_duration?: (Duration | null)
[k: string]: unknown
}
})
export type StakingInfo = ({
existing: {
staking_contract_address: string
[k: string]: unknown
}
} | {
new: {
staking_code_id: number
unstaking_duration?: (Duration | null)
[k: string]: unknown
}
})
/**
 * Duration is a delta of time. You can add it to a BlockInfo or Expiration to move that further in the future. Note that an height-based Duration and a time-based Expiration cannot be combined
 */
export type Duration = ({
height: number
} | {
time: number
})
/**
 * This is used for uploading logo data, or setting it in InstantiateData
 */
export type Logo = ({
url: string
} | {
embedded: EmbeddedLogo
})
/**
 * This is used to store the logo on the blockchain in an accepted format. Enforce maximum size of 5KB on all variants.
 */
export type EmbeddedLogo = ({
svg: Binary
} | {
png: Binary
})
/**
 * Binary is a wrapper around Vec<u8> to add base64 de/serialization with serde. It also adds some helper methods to help encode inline.
 * 
 * This is only needed as serde-json-{core,wasm} has a horrible encoding for Vec<u8>
 */
export type Binary = string

export interface InstantiateMsg {
active_threshold?: (ActiveThreshold | null)
token_info: TokenInfo
[k: string]: unknown
}
export interface Cw20Coin {
address: string
amount: Uint128
[k: string]: unknown
}
export interface InstantiateMarketingInfo {
description?: (string | null)
logo?: (Logo | null)
marketing?: (string | null)
project?: (string | null)
[k: string]: unknown
}
