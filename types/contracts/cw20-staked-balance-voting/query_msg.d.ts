export type QueryMsg = ({
staking_contract: {
[k: string]: unknown
}
} | {
dao: {
[k: string]: unknown
}
} | {
voting_power_at_height: {
address: string
height?: (number | null)
[k: string]: unknown
}
} | {
total_power_at_height: {
height?: (number | null)
[k: string]: unknown
}
} | {
info: {
[k: string]: unknown
}
} | {
token_contract: {
[k: string]: unknown
}
})
