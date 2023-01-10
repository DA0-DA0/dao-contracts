# CW Escrow Streams

This contract enables the creation of native and cw20 token streams, which allows a cw20 payment to be vested continuously over time. This contract is forked off of [cw20-streams](https://github.com/CosmWasm/cw-tokens/tree/main/contracts/cw20-streams) to enable additional features.

## Instantiation

To instantiate a new instance of this contract you must specify a contract owner.

```sh
junod tx wasm instantiate <code-id> '{"admin": "juno12xyz..."}'  --label "cw-payroll contract" --from <your-key> 
```

## Creating a Native Token Stream

```sh
params='{
  "create": {
      "owner": <owner_key>,
      "recipient": <recipient_key>,
       "denom": {
        "Native": "ujunox"
        },
      "balance":0,
      "start_time":"2000000000",
      "end_time":"4000000000",
      "title":"",
      "description":"",
      "is_detachable":false,
    }
}';
junod tx wasm execute <contract-address> "$params" --amount 10000000ujunox --from <your-key> 

```
## Creating a CW20 Stream
A stream can be created using the cw20 [Send / Receive](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw20/README.md#receiver) flow. This involves triggering a Send message from the cw20 token contract, with a Receive callback that's sent to the token streaming contract. The callback message must include the start time and end time of the stream in seconds, as well as the payment recipient. 

```sh
CW20_SEND='{"send": {"contract": "<contract-address>", "amount": "1000000", "msg": "$params"}}'
junod tx wasm execute <cw20-address> "$CW20_SEND" --from <your-key> 
```
## Distribute payments
Streamed payments can be claimed continously at any point after the start time by triggering a Distribute message.
```sh
junod tx wasm execute <contract-address> "'{"distribute": {"id": "<stream_id>"}}'" --from <your-key> 
```
## Linking Streams
Streams can be linked, first stream can distribute native tokens and the linked one for distributing CW20s. 
Linked streams are mirrored, so stopping one will stop the other stream, the same goes for resuming.
```sh
link_params='{
  "link_stream": {
      "0": <left_id>,
      "1": <right_id>,
    }
}';
junod tx wasm execute <contract-address> "'{"link_stream": {"ids": "$link_params"}}'" --from <your-key> 
```