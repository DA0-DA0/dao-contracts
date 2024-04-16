# cw20-hooks

This is a slight modification of the cw20-base contract that allows the minter
to add or remove hooks that are executed on transfer attempts. Hooks are smart
contracts executed via submessage on both transfer and send events. If a hook
throws an error, the transfer is aborted. A hook is the smart contract address
to execute.
