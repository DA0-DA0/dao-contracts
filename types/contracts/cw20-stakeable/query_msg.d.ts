export type QueryMsg = ({
balance: {
address: string
[k: string]: unknown
}
} | {
token_info: {
[k: string]: unknown
}
} | {
minter: {
[k: string]: unknown
}
} | {
allowance: {
owner: string
spender: string
[k: string]: unknown
}
} | {
all_allowances: {
limit?: (number | null)
owner: string
start_after?: (string | null)
[k: string]: unknown
}
} | {
all_accounts: {
limit?: (number | null)
start_after?: (string | null)
[k: string]: unknown
}
} | {
marketing_info: {
[k: string]: unknown
}
} | {
download_logo: {
[k: string]: unknown
}
} | {
staked_balance_at_height: {
address: string
height?: (number | null)
[k: string]: unknown
}
} | {
total_staked_at_height: {
height?: (number | null)
[k: string]: unknown
}
} | {
unstaking_duration: {
[k: string]: unknown
}
} | {
claims: {
address: string
[k: string]: unknown
}
})
