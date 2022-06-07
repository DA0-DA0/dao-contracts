export type ExecuteMsg = ({
update: {
addresses_to_add?: (string[] | null)
addresses_to_remove?: (string[] | null)
group: string
[k: string]: unknown
}
} | {
remove_group: {
group: string
[k: string]: unknown
}
} | {
update_owner: {
owner: string
[k: string]: unknown
}
})
