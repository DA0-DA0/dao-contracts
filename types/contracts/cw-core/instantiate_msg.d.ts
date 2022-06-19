import { ModuleInstantiateInfo } from "./shared-types";

export interface InstantiateMsg {
/**
 * Optional Admin with the ability to execute DAO messages directly. Useful for building SubDAOs controlled by a parent DAO. If no admin is specified the contract is set as its own admin so that the admin may be updated later by governance.
 */
admin?: (string | null)
/**
 * If true the contract will automatically add received cw20 tokens to its treasury.
 */
automatically_add_cw20s: boolean
/**
 * If true the contract will automatically add received cw721 tokens to its treasury.
 */
automatically_add_cw721s: boolean
/**
 * A description of the core contract.
 */
description: string
/**
 * An image URL to describe the core module contract.
 */
image_url?: (string | null)
/**
 * Initial information for arbitrary contract addresses to be added to the items map. The key is the name of the item in the items map. The value is an enum that either uses an existing address or instantiates a new contract.
 */
initial_items?: (InitialItem[] | null)
/**
 * The name of the core contract.
 */
name: string
/**
 * Instantiate information for the core contract's proposal modules.
 */
proposal_modules_instantiate_info: ModuleInstantiateInfo[]
/**
 * Instantiate information for the core contract's voting power module.
 */
voting_module_instantiate_info: ModuleInstantiateInfo
[k: string]: unknown
}
export interface InitialItem {
/**
 * The name of the item.
 */
key: string
/**
 * The value the item will have at instantiation time.
 */
value: string
[k: string]: unknown
}
