import { ContractVersion } from "./shared-types";

export interface InfoResponse {
info: ContractVersion
[k: string]: unknown
}
