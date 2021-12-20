import { Addr, Config } from "./shared-types";

/**
 * Cw4Contract is a wrapper around Addr that provides a lot of helpers for working with cw4 contracts
 * 
 * If you wish to persist this, convert to Cw4CanonicalContract via .canonical()
 */
export type Cw4Contract = Addr

export interface ConfigResponse {
config: Config
group_address: Cw4Contract
[k: string]: unknown
}
