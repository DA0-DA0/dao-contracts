import { Denom } from "./shared-types";

export interface InstantiateMsg {
manager?: (string | null)
owner?: (string | null)
reward_duration: number
reward_token: Denom
staking_contract: string
[k: string]: unknown
}
