/**
 * Duration is a delta of time. You can add it to a BlockInfo or Expiration to move that further in the future. Note that an height-based Duration and a time-based Expiration cannot be combined
 */
export type Duration = ({
height: number
} | {
time: number
})
/**
 * This defines the different ways tallies can happen.
 * 
 * The total_weight used for calculating success as well as the weights of each individual voter used in tallying should be snapshotted at the beginning of the block at which the proposal starts (this is likely the responsibility of a correct cw4 implementation). See also `ThresholdResponse` in the cw3 spec.
 */
export type Threshold = ({
absolute_count: {
weight: number
[k: string]: unknown
}
} | {
absolute_percentage: {
percentage: Decimal
[k: string]: unknown
}
} | {
threshold_quorum: {
quorum: Decimal
threshold: Decimal
[k: string]: unknown
}
})
/**
 * A fixed-point decimal value with 18 fractional digits, i.e. Decimal(1_000_000_000_000_000_000) == 1.0
 * 
 * The greatest possible value that can be represented is 340282366920938463463.374607431768211455 (which is (2^128 - 1) / 10^18)
 */
export type Decimal = string

export interface InstantiateMsg {
group_addr: string
max_voting_period: Duration
threshold: Threshold
[k: string]: unknown
}
