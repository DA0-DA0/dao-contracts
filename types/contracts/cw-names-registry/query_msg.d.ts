export type QueryMsg = ({
config: {
[k: string]: unknown
}
} | {
look_up_name_by_dao: {
dao: string
[k: string]: unknown
}
} | {
look_up_dao_by_name: {
name: string
[k: string]: unknown
}
} | {
is_name_available_to_register: {
name: string
[k: string]: unknown
}
})
