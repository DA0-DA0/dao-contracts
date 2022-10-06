#!/usr/bin/env bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

# assuming test1,test2 has been setup to keyring

# create denom
echo "Creating denom ..."
osmosisd tx tokenfactory create-denom uusd --from test1 -y &> /dev/null

# deploy contract
echo "Deploying contract ..."
beaker wasm deploy tokenfactory-issuer --signer-account test1  --raw '{"denom":"factory/osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks/uusd"}' --no-wasm-opt &> /dev/null

CONTRACT_ADDR=$(cat $SCRIPT_DIR/.beaker/state.local.json | jq '.local."tokenfactory-issuer".addresses.default' | sed 's/"//g') 

# setup beforesend listener
echo "Setting Before send listener ..."
osmosisd tx tokenfactory set-beforesend-listener factory/osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks/uusd $CONTRACT_ADDR --from test1 -y &> /dev/null

# transfer admin control
echo "Transfering admin to contract ..."
osmosisd tx tokenfactory change-admin factory/osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks/uusd $CONTRACT_ADDR --from test1 -y &> /dev/null

# =======================================

# # set mint/burn allowances for test1
echo "Setting mint allowance ..."
beaker wasm execute tokenfactory-issuer --signer-account test1 --raw '{ "set_minter": { "address": "osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks", "allowance": "20000" }}' &> /dev/null

echo "Setting burn allowance ..."
beaker wasm execute tokenfactory-issuer --signer-account test1 --raw '{ "set_burner": { "address": "osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks", "allowance": "20000" }}' &> /dev/null

# mint to test2 
echo "Minting to non owner account (test2 +200uusd) ..."
beaker wasm execute tokenfactory-issuer --signer-account test1 --raw '{ "mint": { "to_address": "osmo18s5lynnmx37hq4wlrw9gdn68sg2uxp5rgk26vv", "amount": "200" }}'  &> /dev/null
echo "=== test2 after mint ==="
osmosisd q bank balances osmo18s5lynnmx37hq4wlrw9gdn68sg2uxp5rgk26vv --denom factory/osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks/uusd
echo

# burn from test2
echo "Burning from non owner account (test2 -100uusd) ..."
beaker wasm execute tokenfactory-issuer --signer-account test1 --raw '{ "burn": { "from_address": "osmo18s5lynnmx37hq4wlrw9gdn68sg2uxp5rgk26vv", "amount": "100" }}' &> /dev/null
echo "=== test2 after burn ==="
osmosisd q bank balances osmo18s5lynnmx37hq4wlrw9gdn68sg2uxp5rgk26vv --denom factory/osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks/uusd
echo


# for transfer test
# osmosisd tx bank send test2 osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks 1factory/osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks/uusd