export type QueryMsg = ({
config: {
[k: string]: unknown
}
} | {
voting_module: {
[k: string]: unknown
}
} | {
governance_modules: {
limit?: (number | null)
start_at?: (string | null)
[k: string]: unknown
}
} | {
dump_state: {
[k: string]: unknown
}
} | {
get_item: {
key: string
[k: string]: unknown
}
} | {
list_items: {
limit?: (number | null)
start_at?: (string | null)
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
})
