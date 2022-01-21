import { Binary, Duration, Threshold, Uint128 } from "./shared-types";

export type GovTokenMsg = ({
instantiate_new_cw20: {
cw20_code_id: number
initial_dao_balance?: (Uint128 | null)
label: string
msg: GovTokenInstantiateMsg
stake_contract_code_id: number
unstaking_duration?: (Duration | null)
[k: string]: unknown
}
} | {
use_existing_cw20: {
addr: string
label: string
stake_contract_code_id: number
unstaking_duration?: (Duration | null)
[k: string]: unknown
}
})
/**
 * This is used for uploading logo data, or setting it in InstantiateData
 */
export type Logo = ({
url: string
} | {
embedded: EmbeddedLogo
})
/**
 * This is used to store the logo on the blockchain in an accepted format. Enforce maximum size of 5KB on all variants.
 */
export type EmbeddedLogo = ({
svg: Binary
} | {
png: Binary
})

export interface InstantiateMsg {
description: string
/**
 * Set an existing governance token or launch a new one
 */
gov_token: GovTokenMsg
/**
 * Optional Image URL that is used by the contract
 */
image_url?: (string | null)
/**
 * The amount of time a proposal can be voted on before expiring
 */
max_voting_period: Duration
name: string
/**
 * Deposit required to make a proposal
 */
proposal_deposit_amount: Uint128
/**
 * Refund a proposal if it is rejected
 */
refund_failed_proposals?: (boolean | null)
/**
 * Voting params configuration
 */
threshold: Threshold
[k: string]: unknown
}
export interface GovTokenInstantiateMsg {
decimals: number
initial_balances: Cw20Coin[]
marketing?: (InstantiateMarketingInfo | null)
name: string
symbol: string
[k: string]: unknown
}
export interface Cw20Coin {
address: string
amount: Uint128
[k: string]: unknown
}
export interface InstantiateMarketingInfo {
description?: (string | null)
logo?: (Logo | null)
marketing?: (string | null)
project?: (string | null)
[k: string]: unknown
}
