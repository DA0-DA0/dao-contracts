import { MemberDiff } from "./shared-types";

export type ExecuteMsg = {
member_changed_hook: {
diffs: MemberDiff[]
[k: string]: unknown
}
}
