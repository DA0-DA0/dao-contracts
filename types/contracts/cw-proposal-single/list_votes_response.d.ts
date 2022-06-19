import { VoteInfo } from "./shared-types";

/**
 * Information about the votes for a proposal.
 */
export interface ListVotesResponse {
votes: VoteInfo[]
[k: string]: unknown
}
