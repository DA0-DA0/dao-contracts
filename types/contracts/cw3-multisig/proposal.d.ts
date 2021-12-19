import { CosmosMsgFor_Empty, Expiration, Status, Threshold, Votes } from "./shared-types";

export interface Proposal {
description: string
expires: Expiration
msgs: CosmosMsgFor_Empty[]
start_height: number
status: Status
/**
 * pass requirements
 */
threshold: Threshold
title: string
total_weight: number
votes: Votes
[k: string]: unknown
}
