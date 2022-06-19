import { Expiration, Uint128 } from "./shared-types";

export interface AllAllowancesResponse {
allowances: AllowanceInfo[]
[k: string]: unknown
}
export interface AllowanceInfo {
allowance: Uint128
expires: Expiration
spender: string
[k: string]: unknown
}
