import { Addr, CheckedDepositInfo, Duration, Threshold } from "./shared-types";

/**
 * The governance module's configuration.
 */
export interface ConfigResponse {
/**
 * Allows changing votes before the proposal expires. If this is enabled proposals will not be able to complete early as final vote information is not known until the time of proposal expiration.
 */
allow_revoting: boolean
/**
 * The address of the DAO that this governance module is associated with.
 */
dao: Addr
/**
 * Information about the depost required to create a proposal. None if no deposit is required, Some otherwise.
 */
deposit_info?: (CheckedDepositInfo | null)
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
