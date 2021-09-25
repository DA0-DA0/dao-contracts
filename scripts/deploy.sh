#!/bin/sh

# NOTE: you will need to update these to deploy on different network
BINARY='starsd'
DENOM='ustarx'
CHAIN_ID='localnet-1'
RPC='http://localhost:26657/'
TXFLAG="--gas-prices 0.01$DENOM --gas auto --gas-adjustment 1.3 -y -b block --chain-id $CHAIN_ID --node $RPC"

: ${1?"Usage: $0 <address-to-deploy-conract-with>"}

RUSTFLAGS='-C link-arg=-s' cargo wasm
COMPILED_CONTRACTS_DIR='../target/wasm32-unknown-unknown/release'

echo "Address to deploy contracts: $1"
echo "TX Flags: $TXFLAG"

#### CW20-GOV ####
# Upload cw20 contract code
# download cw20-gov contract code
CW20_CODE=$($BINARY tx wasm store "$COMPILED_CONTRACTS_DIR/cw20_gov.wasm" --from $1 $TXFLAG --output json | jq -r '.raw_log | fromjson | .[0].events[1].attributes[0].value' )

# Instantiate cw20 contract
CW20_INIT='{
  "name": "daodao",
  "symbol": "DAO",
  "decimals": 6,
  "initial_balances": []
}'
$BINARY tx wasm instantiate $CW20_CODE "$CW20_INIT" --from "$1" --label "gov token" $TXFLAG

# Get cw20 contract address
$BINARY q wasm list-contract-by-code $CW20_CODE --output json
CW20_CONTRACT=$($BINARY q wasm list-contract-by-code $CW20_CODE --output json | jq -r '.contracts[-1]')

echo "cw20: $CW20_CONTRACT"

#### CW-DAO ####
# Upload cw-dao contract code
CW_DAO_CODE=$($BINARY tx wasm store "$COMPILED_CONTRACTS_DIR/cw_dao.wasm" --from $1 $TXFLAG --output json | jq -r '.raw_log | fromjson | .[0].events[1].attributes[0].value' )

echo $CW_DAO_CODE

# Instantiate cw-dao contract
CW_DAO_INIT="{
  \"cw20_addr\": \"$CW20_CONTRACT\",
  \"threshold\": {
    \"absolute_percentage\": {
        \"percentage\": \"0.5\"
    }
  },
  \"max_voting_period\": {
    \"height\": 100
  },
  \"proposal_deposit_amount\": \"1000000\",
  \"proposal_deposit_token_address\": \"$CW20_CONTRACT\"
}"

echo $CW_DAO_INIT | jq .
$BINARY tx wasm instantiate "$CW_DAO_CODE" "$CW_DAO_INIT" --from $1 --label "cw-dao" $TXFLAG
