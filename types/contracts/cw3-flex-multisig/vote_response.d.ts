import { VoteInfo } from "./shared-types";

export interface VoteResponse {
vote?: (VoteInfo | null)
[k: string]: unknown
}
