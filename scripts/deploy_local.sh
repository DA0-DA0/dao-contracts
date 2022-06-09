#!/bin/bash

# Run this from the root repo directory

## CONFIG
IMAGE_TAG=${2:-"v6.0.0"} # this allows you to pass in an image, e.g. pr-156 as arg 2
CONTAINER_NAME="cosmwasm"
BINARY="docker exec -i $CONTAINER_NAME junod"
DENOM='ujunox'
CHAIN_ID='testing'
RPC='http://localhost:26657/'
TXFLAG="--gas-prices 0.1$DENOM --gas auto --gas-adjustment 1.5 -y -b block --chain-id $CHAIN_ID --node $RPC"
BLOCK_GAS_LIMIT=${GAS_LIMIT:-100000000} # should mirror mainnet

echo "Building $IMAGE_TAG"
echo "Configured Block Gas Limit: $BLOCK_GAS_LIMIT"

if [ "$1" = "" ]
then
  echo "Usage: $0 1 arg required, wasm address. See \"Deploying in a development environment\" in README."
  exit
fi

# kill any orphans
docker kill $CONTAINER_NAME
docker volume rm -f junod_data

# Run junod setup script
docker run --rm -d --name $CONTAINER_NAME \
    -e PASSWORD=xxxxxxxxx \
    -e STAKE_TOKEN=$DENOM \
    -e GAS_LIMIT="$GAS_LIMIT" \
    -e UNSAFE_CORS=true \
    -p 1317:1317 -p 26656:26656 -p 26657:26657 \
    --mount type=volume,source=junod_data,target=/root \
    ghcr.io/cosmoscontracts/juno:$IMAGE_TAG /opt/setup_and_run.sh $1

# Compile code
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  --platform linux/amd64 \
  cosmwasm/workspace-optimizer:0.12.6

# Download cw20_base.wasm
curl -LO https://github.com/CosmWasm/cw-plus/releases/download/v0.11.1/cw20_base.wasm
# Download c4_group.wasm
curl -LO https://github.com/CosmWasm/cw-plus/releases/download/v0.11.1/cw4_group.wasm

# Copy wasm binaries to docker container
docker cp cw20_base.wasm cosmwasm:/cw20_base.wasm
docker cp cw4_group.wasm cosmwasm:/cw4_group.wasm

## Clean up
rm cw20_base.wasm
rm cw4_group.wasm

# Sleep while waiting for chain to post genesis block
sleep 3

echo "Address to deploy contracts: $1"
echo "TX Flags: $TXFLAG"

##### UPLOAD CONTRACT DEPS #####

### CW20-BASE ###
CW20_CODE=$(echo xxxxxxxxx | $BINARY tx wasm store "/cw20_base.wasm" --from validator $TXFLAG --output json | jq -r '.logs[0].events[-1].attributes[0].value')

### CW4-GROUP ###
CW4_GROUP_CODE=$(echo xxxxxxxxx | $BINARY tx wasm store "/cw4_group.wasm" --from validator $TXFLAG --output json | jq -r '.logs[0].events[-1].attributes[0].value')

##### UPLOAD DAO DAO CONTRACTS #####

for CONTRACT in ./artifacts/*.wasm; do
  CONTRACT_NAME=`basename $CONTRACT .wasm`
  echo "Processing Contract: $CONTRACT_NAME"

  docker cp artifacts/$CONTRACT_NAME.wasm cosmwasm:/$CONTRACT_NAME.wasm
  CODE_ID=$(echo xxxxxxxxx | $BINARY tx wasm store "/$CONTRACT_NAME.wasm" --from validator $TXFLAG --output json | jq -r '.logs[0].events[-1].attributes[0].value')

  # dynamically create env var to store each contract code id
  declare ${CONTRACT_NAME}_CODE_ID=$CODE_ID
done

##### INSTANTIATE CONTRACTS #####

VOTING_MSG='{
  "token_info": {
    "new": {
      "code_id": '$CW20_CODE',
      "label": "DAO DAO Gov token",
      "name": "DAO",
      "symbol": "DAO",
      "decimals": 6,
      "initial_balances": [
        {
          "address": "'$1'",
          "amount": "1000000000000000"
        }
      ],
      "staking_code_id": '$stake_cw20_CODE_ID',
      "unstaking_duration": {
        "time": 1209600
      }
    }
  }
}'

echo $VOTING_MSG | jq .

ENCODED_VOTING_MSG=$(echo $VOTING_MSG | base64)

PROPOSAL_MSG='{
  "threshold": {
    "threshold_quorum": {
      "threshold": {
        "majority": {}
      },
      "quorum": {
        "percent": "0.1"
      }
    }
  },
  "only_members_execute": true,
  "allow_revoting": false,
  "max_voting_period": {
    "time": 432000
  },
  "deposit_info": {
    "token": {
      "voting_module_token": {}
    },
    "deposit": "1000000000",
    "refund_failed_proposals": true
  }
}'

echo $PROPOSAL_MSG | jq .

ENCODED_PROPOSAL_MSG=$(echo $PROPOSAL_MSG | base64)

# Instantiate a DAO contract instantiates its own cw20
DAO_INIT='{
  "name": "DAO DAO",
  "description": "A DAO that makes DAO tooling",
  "image_url": "https://zmedley.com/raw_logo.png",
  "automatically_add_cw20s": false,
  "automatically_add_cw721s": false,
  "voting_module_instantiate_info": {
    "code_id": '$cw20_staked_balance_voting_CODE_ID',
    "admin": {
      "core_contract": {}
    },
    "label": "DAO DAO Voting Module",
    "msg": "'$ENCODED_VOTING_MSG'"
  },
  "proposal_modules_instantiate_info": [
    {
      "code_id": '$cw_proposal_single_CODE_ID',
      "label": "DAO DAO Proposal Module",
      "admin": {
        "core_contract": {}
      },
      "msg": "'$ENCODED_PROPOSAL_MSG'"
    }
  ]
}'

echo $DAO_INIT | jq .

echo xxxxxxxxx | $BINARY tx wasm instantiate "$cw_core_CODE_ID" "$DAO_INIT" --from validator --label "DAO DAO" $TXFLAG --output json --no-admin

CW_CORE_DAO_CONTRACT=$($BINARY q wasm list-contract-by-code $cw_core_CODE_ID --output json | jq -r '.contracts[-1]')

# Send some coins to the dao contract to initializae its
# treasury. Unless this is done the DAO will be unable to perform
# actions like executing proposals that require it to pay gas fees.
$BINARY tx bank send validator $CW_CORE_DAO_CONTRACT 9000000$DENOM --chain-id testing $TXFLAG -y


# Print out config variables
printf "\n ------------------------ \n"
printf "Config Variables \n\n"

echo "NEXT_PUBLIC_CW20_CODE_ID=$CW20_CODE"
echo "NEXT_PUBLIC_CW4GROUP_CODE_ID=$CW4_GROUP_CODE"
echo "NEXT_PUBLIC_CWCORE_CODE_ID=$cw_core_CODE_ID"
echo "NEXT_PUBLIC_CWPROPOSALSINGLE_CODE_ID=$cw_proposal_single_CODE_ID"
echo "NEXT_PUBLIC_CW4VOTING_CODE_ID=$cw4_voting_CODE_ID"
echo "NEXT_PUBLIC_CW20STAKEDBALANCEVOTING_CODE_ID=$cw20_staked_balance_voting_CODE_ID"
echo "NEXT_PUBLIC_STAKECW20_CODE_ID=$stake_cw20_CODE_ID"
echo "NEXT_PUBLIC_DAO_CONTRACT_ADDRESS=$CW_CORE_DAO_CONTRACT"
