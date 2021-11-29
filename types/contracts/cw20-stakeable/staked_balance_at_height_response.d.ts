import { Uint128 } from "./shared-types";

export interface StakedBalanceAtHeightResponse {
balance: Uint128
height: number
[k: string]: unknown
}
