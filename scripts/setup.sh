#!/usr/bin/env bash

set -o pipefail

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

CW_PLUS_VERSION=v0.16.0
NETWORK="${1:-local}"

deploy_cw4_group() {
    local msg="$1"
    local state_file="state.json"

    if [[ $NETWORK = "local" ]]; then
        state_file="state.local.json"
    fi

    if [[ ! -f "$SCRIPT_DIR/../artifacts/cw4_group.wasm" ]]; then
        echo "Downloading cw4_group.wasm"
        mkdir -p "$SCRIPT_DIR/../artifacts"
        curl -L "https://github.com/CosmWasm/cw-plus/releases/download/$CW_PLUS_VERSION/cw4_group.wasm" -o "$SCRIPT_DIR/../artifacts/cw4_group.wasm"
    fi
    

    beaker wasm deploy cw4-group --signer-account test1 --no-rebuild --raw "$msg" --network "$NETWORK" > /dev/null


    GROUP_CONTRACT_ADDR=$(cat "$SCRIPT_DIR/../.beaker/$state_file" | jq '.local."cw4-group".addresses.default' | sed 's/"//g') 
    echo $GROUP_CONTRACT_ADDR
}

deploy_cw3_flex_multisig() {
    local msg="$1"
    local state_file="state.json"

    if [[ $NETWORK = "local" ]]; then
        state_file="state.local.json"
    fi

    if [[ ! -f "$SCRIPT_DIR/../artifacts/cw3_flex_multisig.wasm" ]]; then
        echo "Downloading cw3_flex_multisig.wasm"
        mkdir -p "$SCRIPT_DIR/../artifacts"
        curl -L "https://github.com/CosmWasm/cw-plus/releases/download/$CW_PLUS_VERSION/cw3_flex_multisig.wasm" -o "$SCRIPT_DIR/../artifacts/cw3_flex_multisig.wasm"
    fi
    

    beaker wasm deploy cw3-flex-multisig --signer-account test1 --no-rebuild --raw "$msg" --network "$NETWORK" > /dev/null


    MULTISIG_CONTRACT_ADDR=$(cat "$SCRIPT_DIR/../.beaker/$state_file" | jq '.local."cw3-flex-multisig".addresses.default' | sed 's/"//g') 
    echo $MULTISIG_CONTRACT_ADDR
}

deploy_tokenfactory_issuer() {
    local msg="$1"
    local state_file="state.json"

    if [[ $NETWORK = "local" ]]; then
        state_file="state.local.json"
    fi

    beaker wasm deploy tokenfactory-issuer --signer-account test1 --raw "$msg" --network "$NETWORK" > /dev/null


    TOKENFACTORY_ISSUER_CONTRACT_ADDR=$(cat "$SCRIPT_DIR/../.beaker/$state_file" | jq '.local."tokenfactory-issuer".addresses.default' | sed 's/"//g') 
    echo $TOKENFACTORY_ISSUER_CONTRACT_ADDR
}

cw4_group_exec() {
    local msg="$1"
    beaker wasm execute cw4-group --signer-account test1 --raw "$msg" > /dev/null
}

# assuming test1,test2 has been setup to keyring
echo ">>> Deploying cw4-group contract ..."
echo

# admin: test1
# members: test{1,2,3}
read -r -d '' MSG <<- EOF
{
    "admin": "osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks",
    "members": [
        {
            "addr": "osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks",
            "weight": 1
        },
        {
            "addr": "osmo18s5lynnmx37hq4wlrw9gdn68sg2uxp5rgk26vv",
            "weight": 1
        },
        {
            "addr": "osmo1qwexv7c6sm95lwhzn9027vyu2ccneaqad4w8ka",
            "weight": 1
        }
    ]
}
EOF
echo "$MSG" | jq

GROUP_CONTRACT_ADDR=$(deploy_cw4_group "$MSG")
echo "GROUP_CONTRACT_ADDR: $GROUP_CONTRACT_ADDR"

echo
echo ">>> Deploying cw3-flex-multisig contract ..."
echo

# 2/3 multisig
read -r -d '' MSG <<- EOF
{
    "group_addr": "$GROUP_CONTRACT_ADDR",
    "threshold": {
        "absolute_count": {
            "weight": 2
        }
    },
    "max_voting_period": {
        "time": 889200
    }
}
EOF
echo "$MSG" | jq

MULTISIG_CONTRACT_ADDR=$(deploy_cw3_flex_multisig "$MSG")

echo "MULTISIG_CONTRACT_ADDR: $MULTISIG_CONTRACT_ADDR"

echo
echo ">>> Add multisig contract as hook to group contract ..."
echo

read -r -d '' MSG <<- EOF
{ "add_hook": { "addr": "$MULTISIG_CONTRACT_ADDR" }}
EOF

echo "$MSG" | jq
cw4_group_exec "$MSG"

echo
echo ">>> Update multisig contract as admin of group contract ..."
echo

read -r -d '' MSG <<- EOF
{ "update_admin": { "admin": "$MULTISIG_CONTRACT_ADDR" }}
EOF

echo "$MSG" | jq
cw4_group_exec "$MSG"


echo
echo ">>> Deploying tokenfactory issuer ..."
echo

read -r -d '' MSG <<- EOF
{ "new_token": { "subdenom": "uusd" }}
EOF

echo "$MSG" | jq
deploy_tokenfactory_issuer "$MSG"