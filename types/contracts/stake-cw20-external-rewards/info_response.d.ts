import { Addr, Denom, Uint128 } from "./shared-types";

export interface InfoResponse {
config: Config
reward: RewardConfig
[k: string]: unknown
}
export interface Config {
manager?: (Addr | null)
owner?: (Addr | null)
reward_token: Denom
staking_contract: Addr
[k: string]: unknown
}
export interface RewardConfig {
period_finish: number
reward_duration: number
reward_rate: Uint128
[k: string]: unknown
}
