import { ModuleInstantiateInfo } from "./shared-types";

export type InitialItemInfo = ({
Existing: {
address: string
[k: string]: unknown
}
} | {
Instantiate: {
info: ModuleInstantiateInfo
[k: string]: unknown
}
})

export interface InstantiateMsg {
/**
 * A description of the governance contract.
 */
description: string
/**
 * Instantiate information for the governance contract's governance modules.
 */
governance_modules_instantiate_info: ModuleInstantiateInfo[]
/**
 * An image URL to describe the governance module contract.
 */
image_url?: (string | null)
/**
 * Initial information for arbitrary contract addresses to be added to the items map. The key is the name of the item in the items map. The value is an enum that either uses an existing address or instantiates a new contract.
 */
initial_items?: (InitialItem[] | null)
/**
 * The name of the governance contract.
 */
name: string
/**
 * Instantiate information for the governance contract's voting power module.
 */
voting_module_instantiate_info: ModuleInstantiateInfo
[k: string]: unknown
}

export interface InitialItem {
/**
 * The info from which to derive the address.
 */
info: InitialItemInfo
/**
 * The name of the item.
 */
name: string
[k: string]: unknown
}
