import { Addr, Config } from "./shared-types";

export interface ConfigResponse {
config: Config
gov_token: Addr
[k: string]: unknown
}
