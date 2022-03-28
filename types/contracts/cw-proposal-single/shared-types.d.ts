/**
 * A thin wrapper around u128 that is using strings for JSON encoding/decoding, such that the full u128 range can be used for clients that convert JSON numbers to floats, like JavaScript and jq.
 *
 * # Examples
 *
 * Use `from` to create instances of this and `u128` to get the value out:
 *
 * ``` # use cosmwasm_std::Uint128; let a = Uint128::from(123u128); assert_eq!(a.u128(), 123);
 *
 * let b = Uint128::from(42u64); assert_eq!(b.u128(), 42);
 *
 * let c = Uint128::from(70u32); assert_eq!(c.u128(), 70); ```
 */
export type Uint128 = string;
/**
 * Information about the token to use for proposal deposits.
 */
export type DepositToken = ({
    token: {
    address: string
    [k: string]: unknown
    }
    } | {
    voting_module_token: {
    [k: string]: unknown
    }
    });
/**
 * Duration is a delta of time. You can add it to a BlockInfo or Expiration to move that further in the future. Note that an height-based Duration and a time-based Expiration cannot be combined
 */
export type Duration = ({
    height: number
    } | {
    time: number
    });
/**
 * The ways a proposal may reach its passing / failing threshold.
 */
export type Threshold = ({
    absolute_percentage: {
    percentage: PercentageThreshold
    [k: string]: unknown
    }
    } | {
    threshold_quorum: {
    quorum: PercentageThreshold
    threshold: PercentageThreshold
    [k: string]: unknown
    }
    });
/**
 * A percentage of voting power that must vote yes for a proposal to pass. An example of why this is needed:
 *
 * If a user specifies a 60% passing threshold, and there are 10 voters they likely expect that proposal to pass when there are 6 yes votes. This implies that the condition for passing should be `yes_votes >= total_votes * threshold`.
 *
 * With this in mind, how should a user specify that they would like proposals to pass if the majority of voters choose yes? Selecting a 50% passing threshold with those rules doesn't properly cover that case as 5 voters voting yes out of 10 would pass the proposal. Selecting 50.0001% or or some variation of that also does not work as a very small yes vote which technically makes the majority yes may not reach that threshold.
 *
 * To handle these cases we provide both a majority and percent option for all percentages. If majority is selected passing will be determined by `yes > total_votes * 0.5`. If percent is selected passing is determined by `yes >= total_votes * percent`.
 *
 * In both of these cases a proposal with only abstain votes must fail. This requires a special case passing logic.
 */
export type PercentageThreshold = ({
    majority: {
    [k: string]: unknown
    }
    } | {
    percent: Decimal
    });
/**
 * A fixed-point decimal value with 18 fractional digits, i.e. Decimal(1_000_000_000_000_000_000) == 1.0
 *
 * The greatest possible value that can be represented is 340282366920938463463.374607431768211455 (which is (2^128 - 1) / 10^18)
 */
export type Decimal = string;
/**
 * Information about the deposit required to create a proposal.
 */
export interface DepositInfo {
    [k: string]: unknown;
    /**
     * The number of tokens that must be deposited to create a proposal.
     */
    deposit: Uint128;
    /**
     * If failed proposals should have their deposits refunded.
     */
    refund_failed_proposals: boolean;
    /**
     * The address of the cw20 token to be used for proposal deposits.
     */
    token: DepositToken;
}
