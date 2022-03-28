import { Addr, Uint128 } from "./shared-types";

export interface Cw20BalancesResponse {
cw20_balances: Cw20CoinVerified[]
[k: string]: unknown
}
export interface Cw20CoinVerified {
address: Addr
amount: Uint128
[k: string]: unknown
}
