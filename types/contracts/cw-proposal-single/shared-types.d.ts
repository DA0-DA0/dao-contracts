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
    } | {
    absolute_count: {
    threshold: Uint128
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
 * Counterpart to the `DepositInfo` struct which has been processed.
 */
export interface CheckedDepositInfo {
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
    token: Addr;
}
export type CosmosMsgFor_Empty = ({
    bank: BankMsg
    } | {
    custom: Empty
    } | {
    staking: StakingMsg
    } | {
    distribution: DistributionMsg
    } | {
    stargate: {
    type_url: string
    value: Binary
    [k: string]: unknown
    }
    } | {
    ibc: IbcMsg
    } | {
    wasm: WasmMsg
    } | {
    gov: GovMsg
    });
/**
 * The message types of the bank module.
 *
 * See https://github.com/cosmos/cosmos-sdk/blob/v0.40.0/proto/cosmos/bank/v1beta1/tx.proto
 */
export type BankMsg = ({
    send: {
    amount: Coin[]
    to_address: string
    [k: string]: unknown
    }
    } | {
    burn: {
    amount: Coin[]
    [k: string]: unknown
    }
    });
/**
 * The message types of the staking module.
 *
 * See https://github.com/cosmos/cosmos-sdk/blob/v0.40.0/proto/cosmos/staking/v1beta1/tx.proto
 */
export type StakingMsg = ({
    delegate: {
    amount: Coin
    validator: string
    [k: string]: unknown
    }
    } | {
    undelegate: {
    amount: Coin
    validator: string
    [k: string]: unknown
    }
    } | {
    redelegate: {
    amount: Coin
    dst_validator: string
    src_validator: string
    [k: string]: unknown
    }
    });
/**
 * The message types of the distribution module.
 *
 * See https://github.com/cosmos/cosmos-sdk/blob/v0.42.4/proto/cosmos/distribution/v1beta1/tx.proto
 */
export type DistributionMsg = ({
    set_withdraw_address: {
    /**
     * The `withdraw_address`
     */
    address: string
    [k: string]: unknown
    }
    } | {
    withdraw_delegator_reward: {
    /**
     * The `validator_address`
     */
    validator: string
    [k: string]: unknown
    }
    });
/**
 * Binary is a wrapper around Vec<u8> to add base64 de/serialization with serde. It also adds some helper methods to help encode inline.
 *
 * This is only needed as serde-json-{core,wasm} has a horrible encoding for Vec<u8>
 */
export type Binary = string;
/**
 * These are messages in the IBC lifecycle. Only usable by IBC-enabled contracts (contracts that directly speak the IBC protocol via 6 entry points)
 */
export type IbcMsg = ({
    transfer: {
    /**
     * packet data only supports one coin https://github.com/cosmos/cosmos-sdk/blob/v0.40.0/proto/ibc/applications/transfer/v1/transfer.proto#L11-L20
     */
    amount: Coin
    /**
     * exisiting channel to send the tokens over
     */
    channel_id: string
    /**
     * when packet times out, measured on remote chain
     */
    timeout: IbcTimeout
    /**
     * address on the remote chain to receive these tokens
     */
    to_address: string
    [k: string]: unknown
    }
    } | {
    send_packet: {
    channel_id: string
    data: Binary
    /**
     * when packet times out, measured on remote chain
     */
    timeout: IbcTimeout
    [k: string]: unknown
    }
    } | {
    close_channel: {
    channel_id: string
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
/**
 * The message types of the wasm module.
 *
 * See https://github.com/CosmWasm/wasmd/blob/v0.14.0/x/wasm/internal/types/tx.proto
 */
export type WasmMsg = ({
    execute: {
    contract_addr: string
    funds: Coin[]
    /**
     * msg is the json-encoded ExecuteMsg struct (as raw Binary)
     */
    msg: Binary
    [k: string]: unknown
    }
    } | {
    instantiate: {
    admin?: (string | null)
    code_id: number
    funds: Coin[]
    /**
     * A human-readbale label for the contract
     */
    label: string
    /**
     * msg is the JSON-encoded InstantiateMsg struct (as raw Binary)
     */
    msg: Binary
    [k: string]: unknown
    }
    } | {
    migrate: {
    contract_addr: string
    /**
     * msg is the json-encoded MigrateMsg struct that will be passed to the new code
     */
    msg: Binary
    /**
     * the code_id of the new logic to place in the given contract
     */
    new_code_id: number
    [k: string]: unknown
    }
    } | {
    update_admin: {
    admin: string
    contract_addr: string
    [k: string]: unknown
    }
    } | {
    clear_admin: {
    contract_addr: string
    [k: string]: unknown
    }
    });
export type GovMsg = {
    vote: {
    proposal_id: number
    vote: VoteOption
    [k: string]: unknown
    }
    };
export type VoteOption = ("yes" | "no" | "abstain" | "no_with_veto");
export type Vote = ("yes" | "no" | "abstain");
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
export interface Coin {
    [k: string]: unknown;
    amount: Uint128;
    denom: string;
}
/**
 * An empty struct that serves as a placeholder in different places, such as contracts that don't set a custom message.
 *
 * It is designed to be expressable in correct JSON and JSON Schema but contains no meaningful data. Previously we used enums without cases, but those cannot represented as valid JSON Schema (https://github.com/CosmWasm/cosmwasm/issues/451)
 */
export interface Empty {
    [k: string]: unknown;
}
/**
 * In IBC each package must set at least one type of timeout: the timestamp or the block height. Using this rather complex enum instead of two timeout fields we ensure that at least one timeout is set.
 */
export interface IbcTimeout {
    [k: string]: unknown;
    block?: (IbcTimeoutBlock | null);
    timestamp?: (Timestamp | null);
}
/**
 * IBCTimeoutHeight Height is a monotonically increasing data type that can be compared against another Height for the purposes of updating and freezing clients. Ordering is (revision_number, timeout_height)
 */
export interface IbcTimeoutBlock {
    [k: string]: unknown;
    /**
     * block height after which the packet times out. the height within the given revision
     */
    height: number;
    /**
     * the version that the client is currently on (eg. after reseting the chain this could increment 1 as height drops to 0)
     */
    revision: number;
}
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
export type Status = ("open" | "rejected" | "passed" | "executed" | "closed");
/**
 * Information about a proposal returned by proposal queries.
 */
export interface ProposalResponse {
    [k: string]: unknown;
    id: number;
    proposal: Proposal;
}
export interface Proposal {
    [k: string]: unknown;
    allow_revoting: boolean;
    /**
     * Information about the deposit that was sent as part of this proposal. None if no deposit.
     */
    deposit_info?: (CheckedDepositInfo | null);
    description: string;
    /**
     * The the time at which this proposal will expire and close for additional votes.
     */
    expiration: Expiration;
    /**
     * The minimum amount of time this proposal must remain open for voting. The proposal may not pass unless this is expired or None.
     */
    min_voting_period?: (Expiration | null);
    /**
     * The messages that will be executed should this proposal pass.
     */
    msgs: CosmosMsgFor_Empty[];
    /**
     * The address that created this proposal.
     */
    proposer: Addr;
    /**
     * The block height at which this proposal was created. Voting power queries should query for voting power at this block height.
     */
    start_height: number;
    status: Status;
    /**
     * The threshold at which this proposal will pass.
     */
    threshold: Threshold;
    title: string;
    /**
     * The total amount of voting power at the time of this proposal's creation.
     */
    total_power: Uint128;
    votes: Votes;
}
export interface Votes {
    [k: string]: unknown;
    abstain: Uint128;
    no: Uint128;
    yes: Uint128;
}
/**
 * Information about a vote that was cast.
 */
export interface VoteInfo {
    [k: string]: unknown;
    /**
     * The voting power behind the vote.
     */
    power: Uint128;
    /**
     * Position on the vote.
     */
    vote: Vote;
    /**
     * The address that voted.
     */
    voter: Addr;
}
