# CW Escrow Streams

This contract enables the creation of native and cw20 token streams, which allows a cw20 payment to be vested continuously over time. This contract is forked off of [cw20-streams](https://github.com/CosmWasm/cw-tokens/tree/main/contracts/cw20-streams) to enable additional features.

## Instantiation

To instantiate a new instance of this contract you must specify a contract owner.

```sh
junod tx wasm instantiate <code-id> '{"admin": "juno12xyz..."}'  --label "cw-payroll contract" --from <your-key> 
```

## Creating a Native Token Stream

TODO: Update upon native token completion

## Creating a CW20 Stream
A stream can be created using the cw20 [Send / Receive](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw20/README.md#receiver) flow. This involves triggering a Send message from the cw20 token contract, with a Receive callback that's sent to the token streaming contract. The callback message must include the start time and end time of the stream in seconds, as well as the payment recipient. 

## Distribute payments
Streamed payments can be claimed continously at any point after the start time by triggering a Distribute message.

## Linking Streams
TODO: describe this feature and how it works.
