import { Addr, Config, ContractVersion, PauseInfoResponse } from "./shared-types";

/**
 * Relevant state for the governance module. Returned by the `DumpState` query.
 */
export interface DumpStateResponse {
/**
 * Optional DAO Admin
 */
admin: Addr
/**
 * The governance contract's config.
 */
config: Config
pause_info: PauseInfoResponse
/**
 * The governance modules associated with the governance contract.
 */
proposal_modules: Addr[]
/**
 * The governance contract's version.
 */
version: ContractVersion
/**
 * The voting module associated with the governance contract.
 */
voting_module: Addr
[k: string]: unknown
}
