#!/bin/sh

if [ "$1" = "" ]
then
  echo "Usage: $0 1 arg required, wasm address. See \"Deploying in a development environment\" in README."
  exit
fi

# NOTE: you will need to update these to deploy on different network
BINARY='docker exec -i cosmwasm wasmd'
DENOM='ustake'
CHAIN_ID='testing'
RPC='http://localhost:26657/'
TXFLAG="--gas-prices 0.01$DENOM --gas auto --gas-adjustment 1.3 -y -b block --chain-id $CHAIN_ID --node $RPC"

# Deploy wasmd in Docker
docker kill cosmwasm

docker volume rm -f wasmd_data

# Run wasmd setup script
docker run --rm -it \
    -e PASSWORD=xxxxxxxxx \
    --mount type=volume,source=wasmd_data,target=/root \
    cosmwasm/wasmd:v0.20.0 /opt/setup_wasmd.sh $1

# Add custom app.toml to wasmd_data volume
docker run -v wasmd_data:/root --name helper busybox true
docker cp docker/app.toml helper:/root/.wasmd/config/app.toml
docker cp docker/config.toml helper:/root/.wasmd/config/config.toml
docker rm helper

# Start wasmd
docker run --rm -d --name cosmwasm -p 26657:26657 -p 26656:26656 -p 1317:1317 \
    --mount type=volume,source=wasmd_data,target=/root \
    cosmwasm/wasmd:v0.20.0 /opt/run_wasmd.sh

# Compile code
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/rust-optimizer:0.12.3

# Copy binaries to docker container
docker cp artifacts/cw_dao.wasm cosmwasm:/cw_dao.wasm
docker cp artifacts/cw20_gov.wasm cosmwasm:/cw20_gov.wasm

# Sleep while waiting for chain to post genesis block
sleep 3

echo "Address to deploy contracts: $1"
echo "TX Flags: $TXFLAG"


#### CW20-GOV ####
# Upload cw20 contract code
# download cw20-gov contract code
$(echo xxxxxxxxx | $BINARY tx wasm store "/cw20_gov.wasm" --from validator $TXFLAG)
CW20_CODE=1

# Instantiate cw20 contract
CW20_INIT='{
  "name": "daodao",
  "symbol": "DAO",
  "decimals": 6,
  "initial_balances": [{"address":"'"$1"'","amount":"1000000000"}]
}'
echo "$CW20_INIT"
$(echo xxxxxxxxx | $BINARY tx wasm instantiate $CW20_CODE "$CW20_INIT" --from "validator" --label "gov token" $TXFLAG)

# Get cw20 contract address
CW20_CONTRACT=$($BINARY q wasm list-contract-by-code $CW20_CODE --output json | jq -r '.contracts[-1]')

#### CW-DAO ####
# Upload cw-dao contract code
$(echo xxxxxxxxx | $BINARY tx wasm store "/cw_dao.wasm" --from validator $TXFLAG)
CW_DAO_CODE=2

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
  \"proposal_deposit_amount\": \"0\",
  \"proposal_deposit_token_address\": \"$CW20_CONTRACT\"
}"

echo $CW_DAO_INIT | jq .

$(echo xxxxxxxxx | $BINARY tx wasm instantiate "$CW_DAO_CODE" "$CW_DAO_INIT" --from validator --label "cw-dao" $TXFLAG)

CW_DAO_CONTRACT=$($BINARY q wasm list-contract-by-code $CW_DAO_CODE --output json | jq -r '.contracts[-1]')

# Print out config variables
printf "\n ------------------------ \n"
printf "Config Variables \n\n"

echo "NEXT_PUBLIC_DAO_TOKEN_ADDRESS: $CW20_CONTRACT"
echo "NEXT_PUBLIC_DAO_CONTRACT_ADDRESS: $CW_DAO_CONTRACT"
