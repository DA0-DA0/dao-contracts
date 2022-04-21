import { DepositInfo, Duration, Threshold } from "./shared-types";

export interface InstantiateMsg {
/**
 * Information about the deposit required to create a proposal. None if there is no deposit requirement, Some otherwise.
 */
deposit_info?: (DepositInfo | null)
/**
 * The default maximum amount of time a proposal may be voted on before expiring.
 */
max_voting_period: Duration
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
