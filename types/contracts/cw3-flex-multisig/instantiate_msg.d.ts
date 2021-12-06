import { Duration, Threshold } from "./shared-types";

export type GroupMsg = ({
instantiate_new_group: {
code_id: number
label: string
voters: Member[]
[k: string]: unknown
}
} | {
use_existing_group: {
addr: string
[k: string]: unknown
}
})

export interface InstantiateMsg {
/**
 * A description of the multisig.
 */
description: string
group: GroupMsg
max_voting_period: Duration
/**
 * The name of the multisig.
 */
name: string
threshold: Threshold
[k: string]: unknown
}
/**
 * A group member has a weight associated with them. This may all be equal, or may have meaning in the app that makes use of the group (eg. voting power)
 */
export interface Member {
addr: string
weight: number
[k: string]: unknown
}
