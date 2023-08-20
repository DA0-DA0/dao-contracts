#!/bin/bash

if [ "$1" = "" ]
then
  echo "Usage: $0 junod key name required as an argument, e.g. ./create-v2-dao-native-voting.sh mykeyname"
  exit
fi

export CHAIN_ID="uni-5"
export TESTNET_NAME="uni-5"
export DENOM="ujunox"
export BECH32_HRP="juno"
export WASMD_VERSION="v0.20.0"
export JUNOD_VERSION="v2.1.0"
export CONFIG_DIR=".juno"
export BINARY="junod"

export COSMJS_VERSION="v0.26.5"
export GENESIS_URL="https://raw.githubusercontent.com/CosmosContracts/testnets/main/uni-2/genesis.json"
export PERSISTENT_PEERS_URL="https://raw.githubusercontent.com/CosmosContracts/testnets/main/uni-2/persistent_peers.txt"
export SEEDS_URL="https://raw.githubusercontent.com/CosmosContracts/testnets/main/uni-2/seeds.txt"

export RPC="https://rpc.uni.juno.deuslabs.fi:443"
export LCD="https://lcd.uni.juno.deuslabs.fi"
export FAUCET="https://faucet.uni.juno.deuslabs.fi"

export COSMOVISOR_VERSION="v0.1.0"
export COSMOVISOR_HOME=$HOME/.juno
export COSMOVISOR_NAME=junod

export TXFLAG="--chain-id ${CHAIN_ID} --gas-prices 0.025ujunox --gas auto --gas-adjustment 1.3 --broadcast-mode block"
export NODE="https://juno-testnet-rpc.polkachu.com:443"

UPDATE_SUBDAOS_MESSAGE='{"update_sub_daos": {"to_add": [{"addr": "juno1xvkad3623mnrhrse6y0atuj0sqk7ugveqxzu0xll64u230ugckaqvh2z92", "charter": "The test v2 subDAO."}], "to_remove":[]}}'
PARENT_DAO_ADDRESS="juno165enxtrex9lhukghct277umy3tcrvhj5p3rrnkw0q34rckgznekse8vlz7"
PROPOSAL_MODULE_ADDRESS="juno1gxst20k2k8xjwjjaw8vrg340zy9aqym7evk8gmut54nxevluey9s2x7tmy"
KEY_NAME=$1

ENCODED=`echo -n $UPDATE_SUBDAOS_MESSAGE | openssl base64 | tr -d '[:space:]'`

EXECUTE_PROPOSE_MESSAGE='{"propose": {
                "description": "update subdaos", 
                "msgs": [
                  {
                    "wasm": {
                      "execute": {
                        "contract_addr": "'$PARENT_DAO_ADDRESS'",
                        "funds": [],
                        "msg": "'$ENCODED'"
                      }
                    }
                  }
                ],
                "title": "Update subDAOs list"
              }}'

echo $EXECUTE_PROPOSE_MESSAGE

junod tx wasm execute $PROPOSAL_MODULE_ADDRESS "$EXECUTE_PROPOSE_MESSAGE" $TXFLAG --node $NODE --from $KEY_NAME

VOTING_MESSAGE='{"vote": {
  "proposal_id": 1,
  "vote": "yes"
}}'

junod tx wasm execute $PROPOSAL_MODULE_ADDRESS "$VOTING_MESSAGE" $TXFLAG --node $NODE --from $KEY_NAME

EXECUTE_MSG='{"execute": {"proposal_id":1}}'

junod tx wasm execute $PROPOSAL_MODULE_ADDRESS '{"execute": {"proposal_id":1}}' $TXFLAG --node $NODE --from $KEY_NAME

junod query wasm contract-state smart $PARENT_DAO_ADDRESS '{"list_sub_daos":{}}' --node $NODE

LIST_SUBDAOS='{"list_sub_daos":{}}'
junod query wasm contract $PARENT_DAO_ADDRESS '{"list_sub_daos":{}}'--node $NODE