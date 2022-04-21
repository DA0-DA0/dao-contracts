import { Addr, CosmosMsgFor_Empty, Expiration, Status, ThresholdResponse, Uint128 } from "./shared-types";

/**
 * Note, if you are storing custom messages in the proposal, the querier needs to know what possible custom message types those are in order to parse the response
 */
export interface ProposalResponse {
deposit_amount: Uint128
description: string
expires: Expiration
id: number
msgs: CosmosMsgFor_Empty[]
proposer: Addr
/**
 * The block height the proposal was created at. This can be cross referenced with staked_balance_at_height queries to determine an addresses's voting power for this proposal.
 */
start_height: number
status: Status
/**
 * This is the threshold that is applied to this proposal. Both the rules of the voting contract, as well as the total_weight of the voting group may have changed since this time. That means that the generic `Threshold{}` query does not provide valid information for existing proposals.
 */
threshold: ThresholdResponse
title: string
[k: string]: unknown
}
