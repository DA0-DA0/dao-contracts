#!/bin/bash

export CHAIN_ID="uni-6"
export TESTNET_NAME="uni-6"
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
export NODE="https://rpc.uni.junonetwork.io:443/"

MODULE_MSG_SINGLE='{
        "allow_revoting": false,
        "max_voting_period": {
          "time": 604800
        },
        "close_proposal_on_execution_failure": true,
        "pre_propose_info": {"anyone_may_propose":{}},
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

ENCODED_PROP_MESSAGE_SINGLE=`echo -n $MODULE_MSG_SINGLE | tr -d '[:space:]' | openssl base64 | tr -d '[:space:]'`
echo -e '\nENCODED PROP MESSAGE SINGLE CHOICE'
echo $ENCODED_PROP_MESSAGE_SINGLE

MODULE_MSG_MULTIPLE='{
	"allow_revoting": false,
	"max_voting_period": {
		"time": 604800
	},
	"close_proposal_on_execution_failure": true,
	"pre_propose_info": {
		"anyone_may_propose": {}
	},
	"only_members_execute": true,
	"voting_strategy": {
		"single_choice": {
			"quorum": {
				"percent": "0.20"
			}
		}
	}
}'

ENCODED_PROP_MESSAGE_MULTIPLE=`echo -n $MODULE_MSG_MULTIPLE | tr -d '[:space:]' | openssl base64 | tr -d '[:space:]'`
echo -e '\nENCODED PROP MESSAGE MULTIPLE CHOICE'
echo $ENCODED_PROP_MESSAGE_MULTIPLE

VOTING_MSG='{"cw4_group_code_id":3472,"initial_members":[{"addr":"juno1873my89qs478e56austefw0ewpp774xmq5m4xv","weight":30}]}'

ENCODED_VOTING_MESSAGE=`echo $VOTING_MSG | tr -d '[:space:]' | openssl base64 | tr -d '[:space:]'`
echo -e '\nENCODED VOTING MESSAGE'
echo $ENCODED_VOTING_MESSAGE

CW_CORE_INIT='{
  "admin": "juno1873my89qs478e56austefw0ewpp774xmq5m4xv",
  "automatically_add_cw20s": true,
  "automatically_add_cw721s": true,
  "description": "V2 DAO with 2 proposal modules",
  "name": "test DAO",
  "proposal_modules_instantiate_info": [
     {
      "admin": {
        "core_module": {}
      },
      "code_id": 3462,
      "label": "multiple choice proposal module",
      "msg": "'$ENCODED_PROP_MESSAGE_MULTIPLE'"
    }
  ],
  "voting_module_instantiate_info": {
    "admin": {
      "core_module": {}
    },
    "code_id": 3465,
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
INIT_MSG='{"instantiate_contract_with_self_admin":{"code_id":1967, "label": "v2 DAO multiple choice", "instantiate_msg":"'$CW_CORE_ENCODED'"}}'

# instantiate with factory 
echo 'instantiating cw-core with factory'
junod tx wasm execute juno143quaa25ynh6j5chhqh8w6lj647l4kpu5r6aqxvzul8du0mwyvns2g6u45 "$INIT_MSG" --from bluenote --node $NODE $TXFLAG
