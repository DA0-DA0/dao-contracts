import { Addr, CosmosMsgFor_Empty, Expiration, Status, Threshold, Uint128, Votes } from "./shared-types";

export interface Proposal {
/**
 * Amount of the native governance token required for voting
 */
deposit: Uint128
description: string
expires: Expiration
msgs: CosmosMsgFor_Empty[]
proposer: Addr
start_height: number
status: Status
/**
 * Pass requirements
 */
threshold: Threshold
title: string
/**
 * The total weight when the proposal started (used to calculate percentages)
 */
total_weight: Uint128
/**
 * summary of existing votes
 */
votes: Votes
[k: string]: unknown
}
