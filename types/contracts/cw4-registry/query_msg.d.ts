export type QueryMsg = {
list_groups: {
limit?: (number | null)
start_after?: (string | null)
user_addr: string
[k: string]: unknown
}
}
