import { VoteInfo } from "./shared-types";

/**
 * Information about a vote.
 */
export interface VoteResponse {
/**
 * None if no such vote, Some otherwise.
 */
vote?: (VoteInfo | null)
[k: string]: unknown
}
