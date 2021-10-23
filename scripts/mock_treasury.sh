#!/bin/sh

if [ "$1" = "" ]
then
  echo "Usage: $0 1 arg required, DAO wasm address. See \"Deploying in a development environment\" in README."
  exit
fi

CW_DAO_CONTRACT=$1

# NOTE: you will need to update these to deploy on different network
BINARY='docker exec -i cosmwasm wasmd'
DENOM='ustake'
CHAIN_ID='testing'
RPC='http://localhost:26657/'
TXFLAG="--gas-prices 0.01$DENOM --gas auto --gas-adjustment 1.3 -y -b block --chain-id $CHAIN_ID --node $RPC"

echo "Address to deploy contracts: $1"
echo "TX Flags: $TXFLAG"


#### CW20-GOV ####
CW20_CODE=1

instantiate_cw20() {
    CW20_INIT="{
        \"name\": \"$1\",
        \"symbol\": \"$2\",
        \"decimals\": 6,
        \"initial_balances\": [{\"address\":\"$CW_DAO_CONTRACT\",\"amount\":\"1000000000\"}]
    }"
    echo "$CW20_INIT"
    echo "BINARY: $BINARY"
    echo "CW20_CODE: $CW20_CODE"
    echo "TXFLAGS: $TXFLAG"
    $(echo 'xxxxxxxxx' | $BINARY tx wasm instantiate $CW20_CODE "$CW20_INIT" --from "validator" --label 'other gov token' $TXFLAG)
}

# Instantiate cw20 contract
instantiate_cw20 'earth' 'EARTH'

# Get cw20 contract address
CW20_CONTRACT_1=$($BINARY q wasm list-contract-by-code $CW20_CODE --output json | jq -r '.contracts[-1]')
echo "CW20_CONTRACT_1: $CW20_CONTRACT_1"

# Instantiate cw20 contract
instantiate_cw20 'moon' 'MOON'

# Get cw20 contract address
CW20_CONTRACT_2=$($BINARY q wasm list-contract-by-code $CW20_CODE --output json | jq -r '.contracts[-1]')
echo "CW20_CONTRACT_2: $CW20_CONTRACT_2"

# Instantiate cw20 contract
instantiate_cw20 'sun' 'SUN'

# Get cw20 contract address
CW20_CONTRACT_3=$($BINARY q wasm list-contract-by-code $CW20_CODE --output json | jq -r '.contracts[-1]')
echo "CW20_CONTRACT_3: $CW20_CONTRACT_3"
