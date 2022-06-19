import { Expiration, Uint128 } from "./shared-types";

export interface ClaimsResponse {
claims: Claim[]
[k: string]: unknown
}
export interface Claim {
amount: Uint128
release_at: Expiration
[k: string]: unknown
}
