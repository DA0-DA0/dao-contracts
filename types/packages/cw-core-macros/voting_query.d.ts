export type VotingQuery = ({
VotingPowerAtHeight: {
address: string
height?: (number | null)
[k: string]: unknown
}
} | {
TotalPowerAtHeight: {
height?: (number | null)
[k: string]: unknown
}
})
