import { Binary, Config, ModuleInstantiateInfo, Timestamp, Uint128 } from "./shared-types";

export type ExecuteMsg = ({
execute_admin_msgs: {
msgs: CosmosMsgFor_Empty[]
[k: string]: unknown
}
} | {
execute_proposal_hook: {
msgs: CosmosMsgFor_Empty[]
[k: string]: unknown
}
} | {
pause: {
duration: Duration
[k: string]: unknown
}
} | {
receive: Cw20ReceiveMsg
} | {
receive_nft: Cw721ReceiveMsg
} | {
remove_item: {
key: string
[k: string]: unknown
}
} | {
set_item: {
addr: string
key: string
[k: string]: unknown
}
} | {
nominate_admin: {
admin?: (string | null)
[k: string]: unknown
}
} | {
accept_admin_nomination: {
[k: string]: unknown
}
} | {
withdraw_admin_nomination: {
[k: string]: unknown
}
} | {
update_config: {
config: Config
[k: string]: unknown
}
} | {
update_cw20_list: {
to_add: string[]
to_remove: string[]
[k: string]: unknown
}
} | {
update_cw721_list: {
to_add: string[]
to_remove: string[]
[k: string]: unknown
}
} | {
update_proposal_modules: {
to_add: ModuleInstantiateInfo[]
to_remove: string[]
[k: string]: unknown
}
} | {
update_voting_module: {
module: ModuleInstantiateInfo
[k: string]: unknown
}
})
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
})
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
})
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
})
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
})
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
})
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
})
export type GovMsg = {
vote: {
proposal_id: number
vote: VoteOption
[k: string]: unknown
}
}
export type VoteOption = ("yes" | "no" | "abstain" | "no_with_veto")
/**
 * Duration is a delta of time. You can add it to a BlockInfo or Expiration to move that further in the future. Note that an height-based Duration and a time-based Expiration cannot be combined
 */
export type Duration = ({
height: number
} | {
time: number
})

export interface Coin {
amount: Uint128
denom: string
[k: string]: unknown
}
/**
 * An empty struct that serves as a placeholder in different places, such as contracts that don't set a custom message.
 * 
 * It is designed to be expressable in correct JSON and JSON Schema but contains no meaningful data. Previously we used enums without cases, but those cannot represented as valid JSON Schema (https://github.com/CosmWasm/cosmwasm/issues/451)
 */
export interface Empty {
[k: string]: unknown
}
/**
 * In IBC each package must set at least one type of timeout: the timestamp or the block height. Using this rather complex enum instead of two timeout fields we ensure that at least one timeout is set.
 */
export interface IbcTimeout {
block?: (IbcTimeoutBlock | null)
timestamp?: (Timestamp | null)
[k: string]: unknown
}
/**
 * IBCTimeoutHeight Height is a monotonically increasing data type that can be compared against another Height for the purposes of updating and freezing clients. Ordering is (revision_number, timeout_height)
 */
export interface IbcTimeoutBlock {
/**
 * block height after which the packet times out. the height within the given revision
 */
height: number
/**
 * the version that the client is currently on (eg. after reseting the chain this could increment 1 as height drops to 0)
 */
revision: number
[k: string]: unknown
}
/**
 * Cw20ReceiveMsg should be de/serialized under `Receive()` variant in a ExecuteMsg
 */
export interface Cw20ReceiveMsg {
amount: Uint128
msg: Binary
sender: string
[k: string]: unknown
}
/**
 * Cw721ReceiveMsg should be de/serialized under `Receive()` variant in a ExecuteMsg
 */
export interface Cw721ReceiveMsg {
msg: Binary
sender: string
token_id: string
[k: string]: unknown
}
