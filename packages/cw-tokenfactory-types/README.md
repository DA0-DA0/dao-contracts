# cw-tokenfactory-types

This package supports contracts that depend on varying tokenfactory standards,
which use very similar or identical Cosmos SDK msgs with different type URLs:

- `/osmosis.tokenfactory...`
- `/cosmwasm.tokenfactory...`
- `/kujira.denom...`

Build features:

- `osmosis_tokenfactory` (default)
- `cosmwasm_tokenfactory`
- `kujira_tokenfactory`
