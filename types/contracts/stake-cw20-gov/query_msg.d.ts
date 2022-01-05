export type QueryMsg = ({
voting_power_at_height: {
address: string
height?: (number | null)
[k: string]: unknown
}
} | {
delegation: {
address: string
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
