/**
 * MemberDiff shows the old and new states for a given cw4 member They cannot both be None. old = None, new = Some -> Insert old = Some, new = Some -> Update old = Some, new = None -> Delete
 */
export interface MemberDiff {
    [k: string]: unknown;
    key: string;
    new?: (number | null);
    old?: (number | null);
}
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
