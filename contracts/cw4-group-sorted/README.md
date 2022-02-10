# cw4 group sorted

This is an implementation of the [cw4
spec](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw4/README.md). Unlike
the
[cw4-group](https://github.com/CosmWasm/cw-plus/tree/main/contracts/cw4-group)
contract in cw-plus this contract returns queries for the list of
members in order of highest weights first.

As it needs to sort the members by weight in order to get the list of
members the member list query is less gas efficent than the cw-plus
version. Happily, queriers will hit limits on the amount of text that
is possible to return from a query before they approach the gas limits
for a query.

Testing on the uni-2 testnet suggests that this cap to the number of
addresses one can add to the contract is somewhere around `5991`. This
was tested using the following shell script:

```bash
CONTRACT_ADDR=juno1rsdnksm93kqtausqf48k0zjez42lzq9988zsr6u38c6un5v02g4qyxmstd

function query_members() {
    junod query wasm contract-state smart $CONTRACT_ADDR '{"list_members":{}}' --output json | jq '.data.members | length'
}

function generate_address() {
    local body=$(cat /dev/urandom | env LC_ALL=C tr -dc 'a-zA-Z0-9' | fold -w 59 | head -n 1)
    echo juno$body
}

function generate_members() {
    count=$1
    for i in $(seq $count); do
	addr=$(generate_address)
	weight=$RANDOM
	echo -n "{\"addr\":\"$addr\",\"weight\":$weight} "
    done
}

function generate_member_list() {
    count=$1
    echo -n "["
    extra_comma=$(generate_members $count | tr ' ' ,)
    echo -n ${extra_comma%?}
    echo -n "]"
}

function add_members() {
    members_unescaped=$(generate_member_list 500)
    members=$(echo $members_unescaped | tr '"' '\"')
    junod tx wasm execute $CONTRACT_ADDR "{\"update_members\":{\"remove\":[],\"add\":$members}}" --from ekez --fees 50000ujunox --chain-id uni-2 --gas auto --gas-adjustment 5 -y
}

while true; do
    old=$(query_members)
    current=$(query_members)
    add_members
    while [ "$old" == "$current" ]; do
	current=$(query_members)
	sleep 10
    done
    echo "-> $current"
done
```
