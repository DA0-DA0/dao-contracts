# cw-tokenfactory-types

This package supports contracts that depend on varying tokenfactory standards,
which use very similar or identical Cosmos SDK msgs with different type URLs:

- `/osmosis.tokenfactory...`
- `/cosmwasm.tokenfactory...`
- `/thorchain.denom.v1...`

Build features:

- `osmosis_tokenfactory` (default)
- `cosmwasm_tokenfactory`
- `thorchain_tokenfactory`
