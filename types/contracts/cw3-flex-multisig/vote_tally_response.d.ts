import { Decimal, Status, ThresholdResponse, Uint128, Votes } from "./shared-types";

/**
 * Information about the current status of a proposal.
 * 
 * NOTE: this response type is not defined in the cw3 spec so we define it ourselves. Information about the current status of a proposal.
 */
export interface VoteTallyResponse {
/**
 * Current percentage turnout
 */
quorum: Decimal
/**
 * Current proposal status
 */
status: Status
/**
 * Required passing criteria
 */
threshold: ThresholdResponse
/**
 * Total number of votes for the proposal
 */
total_votes: Uint128
/**
 * Total number of votes possible for the proposal
 */
total_weight: Uint128
/**
 * Tally of the different votes
 */
votes: Votes
[k: string]: unknown
}
