import { Denom, Uint128 } from "./shared-types";

export interface PendingRewardsResponse {
address: string
denom: Denom
last_update_block: number
pending_rewards: Uint128
[k: string]: unknown
}
