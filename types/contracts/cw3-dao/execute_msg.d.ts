import { Addr, Config, CosmosMsgFor_Empty, Expiration, Vote } from "./shared-types";

export type ExecuteMsg = ({
propose: ProposeMsg
} | {
vote: VoteMsg
} | {
execute: {
proposal_id: number
[k: string]: unknown
}
} | {
close: {
proposal_id: number
[k: string]: unknown
}
} | {
update_config: Config
} | {
update_cw20_token_list: {
to_add: Addr[]
to_remove: Addr[]
[k: string]: unknown
}
})

export interface ProposeMsg {
description: string
latest?: (Expiration | null)
msgs: CosmosMsgFor_Empty[]
title: string
[k: string]: unknown
}

export interface VoteMsg {
proposal_id: number
vote: Vote
[k: string]: unknown
}
