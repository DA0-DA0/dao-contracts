import { Group } from "./shared-types";

export interface DumpResponse {
groups: Group[]
[k: string]: unknown
}
