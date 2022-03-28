export interface InfoResponse {
info: ContractVersion
[k: string]: unknown
}
export interface ContractVersion {
/**
 * contract is the crate name of the implementing contract, eg. `crate:cw20-base` we will use other prefixes for other languages, and their standard global namespacing
 */
contract: string
/**
 * version is any string that this implementation knows. It may be simple counter "1", "2". or semantic version on release tags "v0.7.0", or some custom feature flag list. the only code that needs to understand the version parsing is code that knows how to migrate from the given contract (and is tied to it's implementation somehow)
 */
version: string
[k: string]: unknown
}
