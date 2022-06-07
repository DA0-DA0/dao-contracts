export interface ConfigResponse {
/**
 * If true the contract will automatically add received cw20 tokens to its treasury.
 */
automatically_add_cw20s: boolean
/**
 * If true the contract will automatically add received cw721 tokens to its treasury.
 */
automatically_add_cw721s: boolean
/**
 * A description of the contract.
 */
description: string
/**
 * An optional image URL for displaying alongside the contract.
 */
image_url?: (string | null)
/**
 * The name of the contract.
 */
name: string
[k: string]: unknown
}
