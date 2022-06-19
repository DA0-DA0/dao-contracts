import { Addr } from "./shared-types";

/**
 * Returned by the `AdminNomination` query.
 */
export interface AdminNominationResponse {
/**
 * The currently nominated admin or None if no nomination is pending.
 */
nomination?: (Addr | null)
[k: string]: unknown
}
