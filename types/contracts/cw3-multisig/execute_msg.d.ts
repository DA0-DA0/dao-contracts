import { Config, CosmosMsgFor_Empty, Expiration, Vote } from "./shared-types";

export type ExecuteMsg = ({
propose: {
description: string
latest?: (Expiration | null)
msgs: CosmosMsgFor_Empty[]
title: string
[k: string]: unknown
}
} | {
vote: {
proposal_id: number
vote: Vote
[k: string]: unknown
}
} | {
execute: {
proposal_id: number
[k: string]: unknown
}
} | {
close: {
proposal_id: number
[k: string]: unknown
}
} | {
member_changed_hook: MemberChangedHookMsg
} | {
update_config: Config
})

/**
 * MemberChangedHookMsg should be de/serialized under `MemberChangedHook()` variant in a ExecuteMsg. This contains a list of all diffs on the given transaction.
 */
export interface MemberChangedHookMsg {
diffs: MemberDiff[]
[k: string]: unknown
}
/**
 * MemberDiff shows the old and new states for a given cw4 member They cannot both be None. old = None, new = Some -> Insert old = Some, new = Some -> Update old = Some, new = None -> Delete
 */
export interface MemberDiff {
key: string
new?: (number | null)
old?: (number | null)
[k: string]: unknown
}
