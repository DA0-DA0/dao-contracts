import { DepositInfo, Duration, Threshold } from "./shared-types";

export interface InstantiateMsg {
/**
 * Allows changing votes before the proposal expires. If this is enabled proposals will not be able to complete early as final vote information is not known until the time of proposal expiration.
 */
allow_revoting: boolean
/**
 * Information about the deposit required to create a proposal. None if there is no deposit requirement, Some otherwise.
 */
deposit_info?: (DepositInfo | null)
/**
 * The default maximum amount of time a proposal may be voted on before expiring.
 */
max_voting_period: Duration
/**
 * The minimum amount of time a proposal must be open before passing. A proposal may fail before this amount of time has elapsed, but it will not pass. This can be useful for preventing governance attacks wherein an attacker aquires a large number of tokens and forces a proposal through.
 */
min_voting_period?: (Duration | null)
/**
 * If set to true only members may execute passed proposals. Otherwise, any address may execute a passed proposal.
 */
only_members_execute: boolean
/**
 * The threshold a proposal must reach to complete.
 */
threshold: Threshold
[k: string]: unknown
}
