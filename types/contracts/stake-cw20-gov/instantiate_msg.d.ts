import { Binary, Uint128 } from "./shared-types";

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

export interface InstantiateMsg {
decimals: number
initial_balances: Cw20Coin[]
marketing?: (InstantiateMarketingInfo | null)
mint?: (MinterResponse | null)
name: string
symbol: string
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
export interface MinterResponse {
/**
 * cap is a hard cap on total supply that can be achieved by minting. Note that this refers to total_supply. If None, there is unlimited cap.
 */
cap?: (Uint128 | null)
minter: string
[k: string]: unknown
}
