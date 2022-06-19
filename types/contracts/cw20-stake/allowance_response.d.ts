import { Expiration, Uint128 } from "./shared-types";

export interface AllowanceResponse {
allowance: Uint128
expires: Expiration
[k: string]: unknown
}
