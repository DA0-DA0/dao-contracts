export type QueryMsg = ({
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
staked_value: {
address: string
[k: string]: unknown
}
} | {
total_value: {
[k: string]: unknown
}
} | {
get_config: {
[k: string]: unknown
}
} | {
claims: {
address: string
[k: string]: unknown
}
})
