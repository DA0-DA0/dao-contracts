import { Uint128 } from "./shared-types";

export interface InstantiateMsg {
owner: string
reward_rate: Uint128
reward_token: string
staking_addr: string
[k: string]: unknown
}
