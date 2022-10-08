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

KEY_NAME=$1

MODULE_MSG='{
        "allow_revoting": false,
        "max_voting_period": {
          "time": 604800
        },
        "close_proposal_on_execution_failure": true,
        "pre_propose_info": {"AnyoneMayPropose":{}},
        "only_members_execute": true,
        "threshold": {
          "threshold_quorum": {
            "quorum": {
              "percent": "0.20"
            },
            "threshold": {
              "majority": {}
            }
          }
        }
      }'

ENCODED_PROP_MESSAGE=`echo -n $MODULE_MSG | tr -d '[:space:]' | openssl base64 | tr -d '[:space:]'`
echo -e '\nENCODED PROP MESSAGE'
echo $ENCODED_PROP_MESSAGE

VOTING_MSG='{"cw4_group_code_id":701,"initial_members":[{"addr":"juno1873my89qs478e56austefw0ewpp774xmq5m4xv","weight":30},{"addr":"juno16mrjtqffn3awme2eczhlpwzj7mnatkeluvhj6c","weight":1}]}'

ENCODED_VOTING_MESSAGE=`echo $VOTING_MSG | tr -d '[:space:]' | openssl base64 | tr -d '[:space:]'`
echo -e '\nENCODED VOTING MESSAGE'
echo $ENCODED_VOTING_MESSAGE

CW_CORE_INIT='{
  "admin": "juno12jphyrpd82v8s8cq4n0nu7fa9qcx5hppdwevulhqdhyqu7vkrscs3sv2ct",
  "automatically_add_cw20s": true,
  "automatically_add_cw721s": true,
  "description": "V2 DAO",
  "name": "V2 DAO",
  "proposal_modules_instantiate_info": [
    {
      "admin": {
        "core_module": {}
      },
      "code_id": 696,
      "label": "v2 dao",
      "msg": "'$ENCODED_PROP_MESSAGE'"
    }
  ],
  "voting_module_instantiate_info": {
    "admin": {
      "core_module": {}
    },
    "code_id": 698,
    "label": "test_v2_dao-cw4-voting",
    "msg": "'$ENCODED_VOTING_MESSAGE'"
  }
}'

# encode
CW_CORE_STRIPPED=`echo -n $CW_CORE_INIT | tr -d '[:space:]'`
echo -e 'CW-CORE INSTANTIATE MESSAGE:\n'
echo -$CW_CORE_STRIPPED 
CW_CORE_ENCODED=`echo -n $CW_CORE_STRIPPED | openssl base64 | tr -d '[:space:]'`
echo -e '\nCW-CORE ENCODED MESSAGE:\n'
echo  $CW_CORE_ENCODED

# init with factory
INIT_MSG='{"instantiate_contract_with_self_admin":{"code_id":695, "label": "v2 subDAO subDAO", "instantiate_msg":"'$CW_CORE_ENCODED'"}}'

# instantiate with factory 
echo 'instantiating cw-core with factory'
junod tx wasm execute juno143quaa25ynh6j5chhqh8w6lj647l4kpu5r6aqxvzul8du0mwyvns2g6u45 "$INIT_MSG" --from $KEY_NAME --node $NODE $TXFLAG
