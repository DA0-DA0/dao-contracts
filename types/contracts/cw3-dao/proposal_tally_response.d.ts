import { Decimal, Status, ThresholdResponse, Uint128, Votes } from './shared-types';

/** Tally information for a proposal. */
export interface ProposalTallyResponse {
    /** The current status. */
    status: Status,
    /** The threshold requirements for the proposal to pass. */
    threshold: ThresholdResponse,
    /** The current percentage turnout. */
    quorum: Decimal,
    /** The total number of votes. */
    total_votes: Uint128,
    /** The total number of tokens avaliable for voting (the number of
     * governace tokens avaliable). */
    total_weight: Uint128,
    /** Vote tally information for the proposal. */
    votes: Votes,
}
