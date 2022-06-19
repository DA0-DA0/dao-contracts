import { Uint128 } from "./shared-types";

export interface TokenInfoResponse {
decimals: number
name: string
symbol: string
total_supply: Uint128
[k: string]: unknown
}
