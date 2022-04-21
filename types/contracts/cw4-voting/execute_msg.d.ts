export type ExecuteMsg = ({
increment: {
[k: string]: unknown
}
} | {
reset: {
count: number
[k: string]: unknown
}
})
