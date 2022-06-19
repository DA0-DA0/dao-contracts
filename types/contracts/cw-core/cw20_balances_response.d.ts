import { Addr, Uint128 } from "./shared-types";

/**
 * Returned by the `Cw20Balances` query.
 */
export interface Cw20BalancesResponse {
/**
 * The address of the token.
 */
addr: Addr
/**
 * The contract's balance.
 */
balance: Uint128
[k: string]: unknown
}
