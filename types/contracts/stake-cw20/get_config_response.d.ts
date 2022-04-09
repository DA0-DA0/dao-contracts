import { Duration } from "./shared-types";

export interface GetConfigResponse {
manager?: (string | null)
owner?: (string | null)
token_address: string
unstaking_duration?: (Duration | null)
[k: string]: unknown
}
