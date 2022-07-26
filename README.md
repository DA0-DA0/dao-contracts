# cw-usdc

This repo contains a set of contracts that when used in conjunction with the x/tokenfactory module
in Osmosis will enable a centrally issued stablecoin with the ability to mint, burn, freeze, and blacklist.

The contract would have an owner, but can delegate capabilities to other acccounts. For example, the owner
of a contract can delegate minting allowance of 1000 tokens to a new address.

The contract would be the admin of a tokenfactory denom.  For minting and burning, users then interact with the contract using its own ExecuteMsgs which trigger the contract's access control logic, and the contract then dispatches tokenfactory sdk.Msgs
from its own contract account.

The contract also contains a SudoMsg::BeforeSend hook that allows for the blacklisting of specific accounts as well as the
freezing of all transfers if necessary.

## Deployment

The contract does not create its own tokenfactory denom.  Instead, it is expected that a tokenfactory denom is created by an external account which sets denom metadata, points to the contract as the BeforeSend hook, and then passes over admin control to the contract.

Here we will present guide for getting the contract deployed and setup.

### Prerequisites

There are a few prerequisite tools that we will use for this demo.  We will assume that you have basic tools like Go, Rust, node.js, Docker, and make already installed.  If not, please make sure they are installed before continuing.

#### LocalOsmosis

First, you will need to use an instance of LocalOsmosis using the `fullpowered-tokenfactory` branch.
You can do that using the following commands.

```
git clone https://github.com/osmosis-labs/osmosis
cd osmosis
git checkout fullpowered-tokenfactory
make install
make localnet-build
make localnet-start
```

It's recommended that you configure your `osmosisd` for usage with LocalOsmosis as this will make it easier to interact.

```
osmosisd config keyring-backend test
osmosisd config node http://localhost:26657
osmosisd config chain-id localosmosis
```

Next, we recommend importing the validator test account into your `osmosisd` test keyring backend.  This is the standard
validator test account that is used by localosmosis, beaker, and other tooling.

```
osmosisd keys add validator2 --recover
satisfy adjust timber high purchase tuition stool faith fine install that you unaware feed domain license impose boss human eager hat rent enjoy dawn
```

This should generate the address: `osmo1phaxpevm5wecex2jyaqty2a4v02qj7qmlmzk5a`

#### Beaker

Next we will use the beaker tool for deployment and interacting with CosmWasm contracts on Osmosis.

Please install beaker using cargo.
```
cargo install -f beaker
```

Please clone this repo if you have not already.

```
git clone https://github.com/osmosis-labs/cw-usdc
cd cw-usdc
```

### Creating Token, Contract and Transferring Ownership 

Now we will begin interacting with the chain to begin the instantiation process.

#### Creating tokenfactory denom

Use osmosisd to create a new tokenfactory denom from your validator account. Here we will call the subdenom `uusdc`.

```
osmosisd tx tokenfactory create-denom uusdc --from validator
```

If successful, you can query the base name of your new denom by using the `denoms-from-creator` query.
Here we will assume the creator address was `osmo1phaxpevm5wecex2jyaqty2a4v02qj7qmlmzk5a`.

```
osmosisd q tokenfactory denoms-from-creator osmo1phaxpevm5wecex2jyaqty2a4v02qj7qmlmzk5a
```

#### Set Denom Metadata

TODO

#### Compile and deploy contract

Beaker provides a single command to compile your contracts, deploy wasm code, and instantiate your contract all in one.

To do this, you just need to use the following command from the root folder of the cw-usdc repo.  Note that you put the denom from the previous section in the InstantiateMsg raw json.

```
beaker wasm deploy cw-usdc --signer-account validator  --raw '{"denom":"{factory/osmo1phaxpevm5wecex2jyaqty2a4v02qj7qmlmzk5a/uusdc}"}'
```

This process could take a little while to compile and download dependencies if it is your first time.  Once it is completed, it will give you the address that the contract was deployed to.  For the rest of this demo, we will assume it was deployed to the address `osmo14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sq2r9g9`.

The contract by default makes the instantiator of the contract the original owner. So the current owner of the contract is the `osmo1phaxpevm5wecex2jyaqty2a4v02qj7qmlmzk5a` account. We can transfer ownership later if we would like.

#### Set BeforeSend hook

Now that the contract is deployed, we want to set the token's BeforeSend hook to call the Sudo::BeforeSend function
in the CosmWasm contract.

To do this we use the following command:

```
osmosisd tx tokenfactory set-beforesend-hook factory/osmo1phaxpevm5wecex2jyaqty2a4v02qj7qmlmzk5a/uusdc osmo14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sq2r9g9
```

#### Transfer Admin Control

Finally we will transfer tokenfactory admin control over to the contract.

```
osmosisd tx tokenfactory change-admin factory/osmo1phaxpevm5wecex2jyaqty2a4v02qj7qmlmzk5a/uusdc osmo14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sq2r9g9 --from validator
```

Great! Now we have a deployed contract with beforesend hook and admin control over a new tokenfactory denom!


## Interacting

Now that we have the contract deployed, it is time to start interacting with it. To do this, we will use beaker,
which contains an easy to use tool to interact with CosmWasm contracts from the CLI.

From in the cw-usdc repo, run

```
beaker console
```

If prompted to generate the project's typescript, select Yes.


