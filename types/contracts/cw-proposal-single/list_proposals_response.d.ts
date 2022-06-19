import { ProposalResponse } from "./shared-types";

/**
 * A list of proposals returned by `ListProposals` and `ReverseProposals`.
 */
export interface ListProposalsResponse {
proposals: ProposalResponse[]
[k: string]: unknown
}
