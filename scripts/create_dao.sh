#!/bin/bash

# Given you are using the docker query feel free to do:
# bash scripts/create_dao.sh.sh juno10j9gpw9t4jsz47qgnkvl5n3zlm2fz72k67rxsg 1 2 5

ADDRESS=$1
CW20_CODE=$2
CW3_DAO_CODE=$3
STAKE_CW20_CODE=$4

IMAGE_TAG="pr-135" # moneta
BINARY='docker exec -i cosmwasm junod'
DENOM='ujunox'
CHAIN_ID='testing'
RPC='http://localhost:26657/'
TXFLAG="--gas-prices 0.1$DENOM --gas auto --gas-adjustment 1.5 -y -b block --chain-id $CHAIN_ID --node $RPC"

RANDOM_STRING=$(LC_ALL=C tr -dc A-Z </dev/urandom | head -c 4)

echo $ADDRESS
echo $CW20_CODE
echo $CW3_DAO_CODE
echo $STAKE_CW20_CODE
echo $RANDOM_STRING

# Instantiate a DAO contract instantiates its own cw20
# shellcheck disable=SC2089
CW3_DAO_INIT='{
  "name": "'"$RANDOM_STRING"'",
  "description": "A DAO that makes DAO tooling",
  "gov_token": {
    "instantiate_new_cw20": {
      "cw20_code_id": '$CW20_CODE',
      "stake_contract_code_id": '$STAKE_CW20_CODE',
      "label": "DAO DAO v0.1.1",
      "initial_dao_balance": "1000000000",
      "msg": {
        "name": "'"$RANDOM_STRING"'",
        "symbol": "'"$RANDOM_STRING"'",
        "decimals": 6,
        "initial_balances": [{"address":"'"$1"'","amount":"1000000000"}]
      }
    }
  },
  "threshold": {
    "absolute_percentage": {
        "percentage": "0.5"
    }
  },
  "max_voting_period": {
    "height": 100
  },
  "proposal_deposit_amount": "0",
  "only_members_execute": false
}'

echo $CW3_DAO_INIT | jq .

echo xxxxxxxxx | $BINARY tx wasm instantiate "$CW3_DAO_CODE" "$CW3_DAO_INIT" --from validator --label "DAO DAO" $TXFLAG --output json

CW3_DAO_CONTRACT=$($BINARY q wasm list-contract-by-code $CW3_DAO_CODE --output json | jq -r '.contracts[-1]')

# Send some coins to the dao contract to initializae its
# treasury. Unless this is done the DAO will be unable to perform
# actions like executing proposals that require it to pay gas fees.
$BINARY tx bank send validator $CW3_DAO_CONTRACT 9000000$DENOM --chain-id testing $TXFLAG -y
