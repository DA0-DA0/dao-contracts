#!/bin/sh

if [ "$1" = "" ]
then
  echo "Usage: $0 1 arg required, DAO wasm address. See \"Deploying in a development environment\" in README."
  exit
fi

CW_DAO_CONTRACT=$1
PASSWORD=xxxxxxxxx
VALIDATOR_ADDRESS=$(echo xxxxxxxxx | docker exec -i cosmwasm  wasmd keys show validator -a)

echo "VALIDATOR_ADDRESS: $VALIDATOR_ADDRESS"

# NOTE: you will need to update these to deploy on different network
BINARY='docker exec -i cosmwasm wasmd'
DENOM='ustake'
CHAIN_ID='testing'
RPC='http://localhost:26657/'
FLAGS="--chain-id $CHAIN_ID --node $RPC"
TXFLAG="--gas-prices 0.01$DENOM --gas auto --gas-adjustment 1.3 -y -b block $FLAGS"

echo "Address to deploy contracts: $1"
echo "TX Flags: $TXFLAG"

#### NATIVE ####

echo $PASSWORD | $BINARY tx send $VALIDATOR_ADDRESS $CW_DAO_CONTRACT 100000000ustake --from $VALIDATOR_ADDRESS $TXFLAG --output json

echo $PASSWORD | $BINARY tx send $VALIDATOR_ADDRESS $CW_DAO_CONTRACT 200000000ustake --from $VALIDATOR_ADDRESS $TXFLAG --output json

echo $PASSWORD | $BINARY tx send $VALIDATOR_ADDRESS $CW_DAO_CONTRACT 300000000ustake --from $VALIDATOR_ADDRESS $TXFLAG --output json

NATIVE_BALANCE=$($BINARY q bank balances $CW_DAO_CONTRACT $FLAGS)
echo "DAO NATIVE BALANCE: $NATIVE_BALANCE"

#### CW20-GOV EXAMPLE TOKENS ####
CW20_CODE=1

instantiate_cw20() {
    CW20_INIT="{
        \"name\": \"$1\",
        \"symbol\": \"$2\",
        \"decimals\": 6,
        \"initial_balances\": [{\"address\":\"$VALIDATOR_ADDRESS\",\"amount\":\"1000000000\"}]
    }"
    echo "$CW20_INIT"
    echo "BINARY: $BINARY"
    echo "CW20_CODE: $CW20_CODE"
    echo "TXFLAGS: $TXFLAG"
    echo 'xxxxxxxxx' | $BINARY tx wasm instantiate $CW20_CODE "$CW20_INIT" --from "validator" --label 'other gov token' $TXFLAG
}

# Instantiate cw20 contract
instantiate_cw20 'uearth' 'EARTH'

# Get cw20 contract address
CW20_CONTRACT_1=$($BINARY q wasm list-contract-by-code $CW20_CODE --output json $FLAGS | jq -r '.contracts[-1]')
echo "CW20_CONTRACT_1: $CW20_CONTRACT_1"

# Instantiate cw20 contract
instantiate_cw20 'umoon' 'MOON'

# Get cw20 contract address
CW20_CONTRACT_2=$($BINARY q wasm list-contract-by-code $CW20_CODE --output json | jq -r '.contracts[-1]')
echo "CW20_CONTRACT_2: $CW20_CONTRACT_2"

# Instantiate cw20 contract
instantiate_cw20 'usun' 'SUN'

# Get cw20 contract address
CW20_CONTRACT_3=$($BINARY q wasm list-contract-by-code $CW20_CODE --output json | jq -r '.contracts[-1]')
echo "CW20_CONTRACT_3: $CW20_CONTRACT_3"

# Send funds from validator to DAO and print balances
echo $PASSWORD | $BINARY tx wasm execute $CW20_CONTRACT_1 "{\"send\":{\"contract\":\"$CW_DAO_CONTRACT\", \"amount\": \"900000000\", \"msg\": \"\"}}" --from validator $TXFLAG

echo "CW20_CONTRACT_1: $CW20_CONTRACT_1"
$BINARY q wasm contract-state smart $CW20_CONTRACT_1 "{\"balance\":{\"address\":\"$CW_DAO_CONTRACT\"}}" $FLAGS

echo $PASSWORD | $BINARY tx wasm execute $CW20_CONTRACT_2 "{\"send\":{\"contract\":\"$CW_DAO_CONTRACT\", \"amount\": \"800000000\", \"msg\": \"\"}}" --from validator $TXFLAG

echo "CW20_CONTRACT_1: $CW20_CONTRACT_1"
$BINARY q wasm contract-state smart $CW20_CONTRACT_2 "{\"balance\":{\"address\":\"$CW_DAO_CONTRACT\"}}" $FLAGS

echo $PASSWORD | $BINARY tx wasm execute $CW20_CONTRACT_3 "{\"send\":{\"contract\":\"$CW_DAO_CONTRACT\", \"amount\": \"700000000\", \"msg\": \"\"}}" --from validator $TXFLAG

echo "CW20_CONTRACT_1: $CW20_CONTRACT_1"
$BINARY q wasm contract-state smart $CW20_CONTRACT_3 "{\"balance\":{\"address\":\"$CW_DAO_CONTRACT\"}}" $FLAGS
