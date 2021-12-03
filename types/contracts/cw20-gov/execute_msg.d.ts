import { Binary, Expiration, Logo, Uint128 } from "./shared-types";

export type ExecuteMsg = ({
transfer: {
amount: Uint128
recipient: string
[k: string]: unknown
}
} | {
burn: {
amount: Uint128
[k: string]: unknown
}
} | {
send: {
amount: Uint128
contract: string
msg: Binary
[k: string]: unknown
}
} | {
increase_allowance: {
amount: Uint128
expires?: (Expiration | null)
spender: string
[k: string]: unknown
}
} | {
decrease_allowance: {
amount: Uint128
expires?: (Expiration | null)
spender: string
[k: string]: unknown
}
} | {
transfer_from: {
amount: Uint128
owner: string
recipient: string
[k: string]: unknown
}
} | {
send_from: {
amount: Uint128
contract: string
msg: Binary
owner: string
[k: string]: unknown
}
} | {
burn_from: {
amount: Uint128
owner: string
[k: string]: unknown
}
} | {
mint: {
amount: Uint128
recipient: string
[k: string]: unknown
}
} | {
update_marketing: {
description?: (string | null)
marketing?: (string | null)
project?: (string | null)
[k: string]: unknown
}
} | {
upload_logo: Logo
} | {
delegate_votes: {
recipient: string
[k: string]: unknown
}
})
