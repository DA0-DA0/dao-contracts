import { Duration, Logo, Uint128 } from "./shared-types";

export interface InstantiateMsg {
cw20_base: InstantiateMsg1
unstaking_duration?: (Duration | null)
[k: string]: unknown
}
export interface InstantiateMsg1 {
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
