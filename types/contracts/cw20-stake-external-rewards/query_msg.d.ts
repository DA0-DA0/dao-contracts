export type QueryMsg = ({
info: {
[k: string]: unknown
}
} | {
get_pending_rewards: {
address: string
[k: string]: unknown
}
})
