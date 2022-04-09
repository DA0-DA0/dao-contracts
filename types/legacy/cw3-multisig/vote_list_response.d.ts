import { VoteInfo } from "./shared-types";

export interface VoteListResponse {
votes: VoteInfo[]
[k: string]: unknown
}
