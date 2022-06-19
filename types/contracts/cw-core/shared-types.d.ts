/**
 * A human readable address.
 *
 * In Cosmos, this is typically bech32 encoded. But for multi-chain smart contracts no assumptions should be made other than being UTF-8 encoded and of reasonable length.
 *
 * This type represents a validated address. It can be created in the following ways 1. Use `Addr::unchecked(input)` 2. Use `let checked: Addr = deps.api.addr_validate(input)?` 3. Use `let checked: Addr = deps.api.addr_humanize(canonical_addr)?` 4. Deserialize from JSON. This must only be done from JSON that was validated before such as a contract's state. `Addr` must not be used in messages sent by the user because this would result in unvalidated instances.
 *
 * This type is immutable. If you really need to mutate it (Really? Are you sure?), create a mutable copy using `let mut mutable = Addr::to_string()` and operate on that `String` instance.
 */
export type Addr = string;
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
 * Information about if the contract is currently paused.
 */
export type PauseInfoResponse = ({
    Paused: {
    expiration: Expiration
    [k: string]: unknown
    }
    } | {
    Unpaused: {
    [k: string]: unknown
    }
    });
/**
 * Expiration represents a point in time when some event happens. It can compare with a BlockInfo and will return is_expired() == true once the condition is hit (and for every block in the future)
 */
export type Expiration = ({
    at_height: number
    } | {
    at_time: Timestamp
    } | {
    never: {
    [k: string]: unknown
    }
    });
/**
 * A point in time in nanosecond precision.
 *
 * This type can represent times from 1970-01-01T00:00:00Z to 2554-07-21T23:34:33Z.
 *
 * ## Examples
 *
 * ``` # use cosmwasm_std::Timestamp; let ts = Timestamp::from_nanos(1_000_000_202); assert_eq!(ts.nanos(), 1_000_000_202); assert_eq!(ts.seconds(), 1); assert_eq!(ts.subsec_nanos(), 202);
 *
 * let ts = ts.plus_seconds(2); assert_eq!(ts.nanos(), 3_000_000_202); assert_eq!(ts.seconds(), 3); assert_eq!(ts.subsec_nanos(), 202); ```
 */
export type Timestamp = Uint64;
/**
 * A thin wrapper around u64 that is using strings for JSON encoding/decoding, such that the full u64 range can be used for clients that convert JSON numbers to floats, like JavaScript and jq.
 *
 * # Examples
 *
 * Use `from` to create instances of this and `u64` to get the value out:
 *
 * ``` # use cosmwasm_std::Uint64; let a = Uint64::from(42u64); assert_eq!(a.u64(), 42);
 *
 * let b = Uint64::from(70u32); assert_eq!(b.u64(), 70); ```
 */
export type Uint64 = string;
export interface Config {
    [k: string]: unknown;
    /**
     * If true the contract will automatically add received cw20 tokens to its treasury.
     */
    automatically_add_cw20s: boolean;
    /**
     * If true the contract will automatically add received cw721 tokens to its treasury.
     */
    automatically_add_cw721s: boolean;
    /**
     * A description of the contract.
     */
    description: string;
    /**
     * An optional image URL for displaying alongside the contract.
     */
    image_url?: (string | null);
    /**
     * The name of the contract.
     */
    name: string;
}
export interface ContractVersion {
    [k: string]: unknown;
    /**
     * contract is the crate name of the implementing contract, eg. `crate:cw20-base` we will use other prefixes for other languages, and their standard global namespacing
     */
    contract: string;
    /**
     * version is any string that this implementation knows. It may be simple counter "1", "2". or semantic version on release tags "v0.7.0", or some custom feature flag list. the only code that needs to understand the version parsing is code that knows how to migrate from the given contract (and is tied to it's implementation somehow)
     */
    version: string;
}
/**
 * Binary is a wrapper around Vec<u8> to add base64 de/serialization with serde. It also adds some helper methods to help encode inline.
 *
 * This is only needed as serde-json-{core,wasm} has a horrible encoding for Vec<u8>
 */
export type Binary = string;
/**
 * Information about the admin of a contract.
 */
export type Admin = ({
    address: {
    addr: string
    [k: string]: unknown
    }
    } | {
    core_contract: {
    [k: string]: unknown
    }
    } | {
    none: {
    [k: string]: unknown
    }
    });
/**
 * Information needed to instantiate a proposal or voting module.
 */
export interface ModuleInstantiateInfo {
    [k: string]: unknown;
    /**
     * Admin of the instantiated contract.
     */
    admin: Admin;
    /**
     * Code ID of the contract to be instantiated.
     */
    code_id: number;
    /**
     * Label for the instantiated contract.
     */
    label: string;
    /**
     * Instantiate message to be used to create the contract.
     */
    msg: Binary;
}
