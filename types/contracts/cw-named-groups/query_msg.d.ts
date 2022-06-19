export type QueryMsg = ({
dump: {
[k: string]: unknown
}
} | {
list_groups: {
address: string
limit?: (number | null)
offset?: (number | null)
[k: string]: unknown
}
} | {
list_addresses: {
group: string
limit?: (number | null)
offset?: (number | null)
[k: string]: unknown
}
} | {
is_address_in_group: {
address: string
group: string
[k: string]: unknown
}
})
