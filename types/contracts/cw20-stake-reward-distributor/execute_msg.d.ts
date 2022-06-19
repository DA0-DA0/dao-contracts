import { Uint128 } from "./shared-types";

export type ExecuteMsg = ({
update_config: {
owner: string
reward_rate: Uint128
reward_token: string
staking_addr: string
[k: string]: unknown
}
} | {
distribute: {
[k: string]: unknown
}
} | {
withdraw: {
[k: string]: unknown
}
})
