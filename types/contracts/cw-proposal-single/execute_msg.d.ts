import { CosmosMsgFor_Empty, DepositInfo, Duration, Threshold, Vote } from "./shared-types";

export type ExecuteMsg = ({
propose: {
/**
 * A description of the proposal.
 */
description: string
/**
 * The messages that should be executed in response to this proposal passing.
 */
msgs: CosmosMsgFor_Empty[]
/**
 * The title of the proposal.
 */
title: string
[k: string]: unknown
}
} | {
vote: {
/**
 * The ID of the proposal to vote on.
 */
proposal_id: number
/**
 * The senders position on the proposal.
 */
vote: Vote
[k: string]: unknown
}
} | {
execute: {
/**
 * The ID of the proposal to execute.
 */
proposal_id: number
[k: string]: unknown
}
} | {
close: {
/**
 * The ID of the proposal to close.
 */
proposal_id: number
[k: string]: unknown
}
} | {
update_config: {
/**
 * Allows changing votes before the proposal expires. If this is enabled proposals will not be able to complete early as final vote information is not known until the time of proposal expiration.
 */
allow_revoting: boolean
/**
 * The address if tge DAO that this governance module is associated with.
 */
dao: string
/**
 * Information about the deposit required to make a proposal. None if no deposit, Some otherwise.
 */
deposit_info?: (DepositInfo | null)
/**
 * The default maximum amount of time a proposal may be voted on before expiring. This will only apply to proposals created after the config update.
 */
max_voting_period: Duration
/**
 * The minimum amount of time a proposal must be open before passing. A proposal may fail before this amount of time has elapsed, but it will not pass. This can be useful for preventing governance attacks wherein an attacker aquires a large number of tokens and forces a proposal through.
 */
min_voting_period?: (Duration | null)
/**
 * If set to true only members may execute passed proposals. Otherwise, any address may execute a passed proposal. Applies to all outstanding and future proposals.
 */
only_members_execute: boolean
/**
 * The new proposal passing threshold. This will only apply to proposals created after the config update.
 */
threshold: Threshold
[k: string]: unknown
}
} | {
add_proposal_hook: {
address: string
[k: string]: unknown
}
} | {
remove_proposal_hook: {
address: string
[k: string]: unknown
}
} | {
add_vote_hook: {
address: string
[k: string]: unknown
}
} | {
remove_vote_hook: {
address: string
[k: string]: unknown
}
})
