/**
 * Returned by the `GetItem` query.
 */
export interface GetItemResponse {
/**
 * `None` if no item with the provided key was found, `Some` otherwise.
 */
item?: (string | null)
[k: string]: unknown
}
