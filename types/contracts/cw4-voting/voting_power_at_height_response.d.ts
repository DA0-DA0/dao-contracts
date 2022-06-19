import { Uint128 } from "./shared-types";

export interface VotingPowerAtHeightResponse {
height: number
power: Uint128
[k: string]: unknown
}
