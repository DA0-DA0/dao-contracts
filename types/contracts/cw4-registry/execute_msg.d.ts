export type ExecuteMsg = ({
register: {
group_addrs: string[]
[k: string]: unknown
}
} | {
member_changed_hook: MemberChangedHookMsg
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
