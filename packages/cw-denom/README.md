# CosmWasm Denom

This is a simple package for validating cw20 and Cosmos SDK native
denominations. It proves the types, `UncheckedDenom` and
`CheckedDenom`. `UncheckedDenom` may be used in CosmWasm contract
messages and checked via the `into_checked` method.

To validate native denominations, this package uses the [same
rules](https://github.com/cosmos/cosmos-sdk/blob/7728516abfab950dc7a9120caad4870f1f962df5/types/coin.go#L865-L867) as the SDK.

To validate cw20 denominations this package ensures that the
specified address is valid, that the specified address is a
CosmWasm contract, and that the specified address responds
correctly to cw20 `TokenInfo` queries.
