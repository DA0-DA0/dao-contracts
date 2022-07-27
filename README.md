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
osmosisd config broadcast-mode block
```

Next, we recommend importing the validator test account into your `osmosisd` test keyring backend.  [This is the standard
validator test account that is used by localosmosis, beaker, and other tooling](https://docs.osmosis.zone/developing/tools/localosmosis.html#accounts).

```
osmosisd keys add test1 --recover
notice oak worry limit wrap speak medal online prefer cluster roof addict wrist behave treat actual wasp year salad speed social layer crew genius
```

This should generate the address: `osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks`

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
osmosisd tx tokenfactory create-denom uusdc --from test1
```

If successful, you can query the base name of your new denom by using the `denoms-from-creator` query.
Here we will assume the creator address was `osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks`.

```
osmosisd q tokenfactory denoms-from-creator osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks
```

#### Set Denom Metadata

TODO

#### Compile and deploy contract

Beaker provides a single command to compile your contracts, deploy wasm code, and instantiate your contract all in one.

To do this, you just need to use the following command from the root folder of the cw-usdc repo.  Note that you put the denom from the previous section in the InstantiateMsg raw json.

```
beaker wasm deploy cw-usdc --signer-account test1  --raw '{"denom":"factory/osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks/uusdc"}'
```

This process could take a little while to compile and download dependencies if it is your first time.  Once it is completed, it will give you the address that the contract was deployed to.  For the rest of this demo, we will assume it was deployed to the address `osmo14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sq2r9g9`.

The contract by default makes the instantiator of the contract the original owner. So the current owner of the contract is the `osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks` account. We can transfer ownership later if we would like.

#### Set BeforeSend hook

Now that the contract is deployed, we want to set the token's BeforeSend hook to call the Sudo::BeforeSend function
in the CosmWasm contract.

To do this we use the following command:

```
osmosisd tx tokenfactory set-beforesend-hook factory/osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks/uusdc osmo14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sq2r9g9 --from test1
```

#### Transfer Admin Control

Finally we will transfer tokenfactory admin control over to the contract.

```
osmosisd tx tokenfactory change-admin factory/osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks/uusdc osmo14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sq2r9g9 --from test1
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

### Beaker Config

We'll do a couple of variable setting in the beaker console to make the commands easier to type out.

- Set the cw-usdc contract object to a variable called `sc`
- All the test accounts are in a variable called accounts.  Because the test1 account is the owner of the contract (having been the one that created it), we'll save the validator account to a variable called `owner`.
- We'll create a signer object for the owner account.

```
sc = contract['cw-usdc']
owner = account.test1
signer = sc.signer(owner)
```

### Basic Queries

We can do some basic queries right away.

Query the denom that the contract is meant to control.

```
await sc.denom()
```

Query the owner of the contract.
```
await sc.owner()
```

### Minting

Now we will try to do some minting.

Although the test1 account is the owner of the contract, it doesn't yet have minting capabilities.
Let's first give minting capabilities to it.

```
await signer.setMinter({ address: owner.address, allowance: "100000" })
```

Now we can mint tokens!
```
await signer.mint({ toAddress: owner.address, amount: "100" })
```

From outside beaker console (open a new tab), use `osmosisd` to query the balances of the test1 address.
The balance of the tokenfactory denom should have increased!

```
osmosisd q bank balances osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks
```

Now let's make sure the minting allowance of the validator address actually successfully decreased.

```
await sc.mintAllowance({address: owner.address})
```

You can also query all the minter allowances that have been given.

```
await sc.mintAllowances({})
```

### Blacklisting

Let's try using the blacklist functionality of the contract.

First lets give the validator account blacklist capabilitities.
```
await signer.setBlacklister({address: owner.address, status: true})
```

Now use the owner account to blacklist itself!
```
await signer.blacklist({address: owner.address, status: true})
```

You can query the all the blacklisted addresses.
```
await sc.blacklistees({})
```

Use osmosisd to try to create a send tx from the owner account.  It should fail with a blacklist error.
```
osmosisd tx bank send test1 osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks 10factory/osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks/uusdc
```

Let's try to unblacklist the owner account.
```
await signer.blacklist({address: owner.address, status: false})
```

Now lets use osmosisd to try to create a send tx from the owner account again. It should pass this time!
```
osmosisd tx bank send test1 osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks 10factory/osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks/uusdc
```

### Freezing

Now let's try the global freezing functionality.

First give the owner freezing capability and then freeze the contract.

```
await signer.setFreezer({ address: owner.address, status: true })
await signer.freeze({ status: true })
```

Query the frozen status of the contract.
```
await sc.isFrozen({})
```


Use osmosisd to try to create a send tx.  It should fail with a contract is frozen error.
```
osmosisd tx bank send test1 osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks 10factory/osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks/uusdc
```
