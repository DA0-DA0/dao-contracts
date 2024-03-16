# cw-tokenfactory-types

This package supports contracts that depend on varying tokenfactory standards,
which use very similar or identical Cosmos SDK msgs with different type URLs:

- `/cosmwasm.tokenfactory...`
- `/osmosis.tokenfactory...`

Enabling the `cosmwasm_tokenfactory` build feature will use the
`/cosmwasm.tokenfactory...` msg type URLs, whereas enabling the
`osmosis_tokenfactory` build feature will use the `/osmosis.tokenfactory...` msg
type URLs. `osmosis_tokenfactory` is enabled by default.
