# Introduction

Delegation is basically: “we trust you have the judgment, we give you the power to do this specific action within some X time frame. You have the right but not the obligation.”

## Examples 
- You have the power to kick this specific member after review. Perhaps you make an investigative committee after a member is accused of something, and delegate the power to a multisig to execute the message: “Kick this member”
- You can pause one of our SubDAOs for 1 week if necessary
- You have the power to add a specific item to our DAO config at any time
- You have the power to increase Bob’s salary by 800 JUNO through DAO treasury
- Liquidate 8000 JUNO through JunoSwap at any time. Perhaps you want to delegate this to a hedge fund who promises to “time the market”

## When would it not be useful?

- **You can compose it through something other than a proposal module**. A separate option contract. However, this would not allow you to execute messages through the core module. Only proposal modules can pass messages through the core module.
  - In order to control Treasury funds, you can siphon off to a SubDAO, and then SubDAO can make the decisions. However, this does not constrain the action of the SubDAO. It can totally misuse those funds. We want allowance of specific and concrete messages.
  - In most cases, an escrow contract would do the job, but in certain cases, we want to be very granular with the actions we delegate

## When would it be useful?

- **For specific, constrained DAO-related actions**. For the things the DAO control and in which an escrow would give too much power over the resource. The DAO wants delegation on specific message that gives the delegate constrained power
- **Time-based actions**. Actions where the time of execution matters, and an executive decision maker needs power to execute at any time
- **Judgment-based actions**. Perhaps another DAO or oracle service specializes in identity-proving activities, and you would like to mark certain members as a real person for some sort of one person one vote thing. Or, perhaps, you want to remove a multisig member, and decide to delegate said action to a Judiciary DAO, some sort of tribunal



# Design

This is kind of a “messages escrow” module in which the execution of the proposal is wrapped into an option and execution is given to another party.


It would be a “proposal module” that allows arbitrary wrapping of messages, and would be required for the DAO to add as a Proposal Module. However, it would not use a voting module, but rather, another proposal module like `cwd-proposal-single` would pass a delegation message to the Core module, whereby the core module would execute a delegation message.

Execution messages:

```rust
// Gives back a delegation ID
Delegate { msgs: Vec<CosmosMsg<Empty>>, addr: String, expiration: Expiration }
// Authorized execution only
Execute  { delegation_id: u64 }
```

State:

```rust
struct Config {
    admin: Addr,
}

struct Delegation {
	addr: Addr,
	msgs: Vec<CosmosMsg<Empty>>,
	expiration: Expiration,
}

const DELEGATIONS: Map<u64, Delegation> = Map::new("delegations");
const CONFIG: Item<Config> = Item::new("config");
```

## How could a DaoDao core module use this?

1. Add as a proposal module
2. Pass a delegation message through another proposal module (single or multiple-choice) as a Wasm message (converted into Cosmos message)

   ```rust
   WasmMsg::Execute {
   	contract_addr: // address of Delegation Proposal Module
   	msg: to_binary(&DelegationExecuteMsg::Delegate {
   		msgs: // Cosmos messages to delegate,
   	  addr: "0x2342", // Delegate with power to execute, possibly another DAO
   	  expiration: 123123324
   	})?,
   	funds: vec![]
   }
   ```

3. The delegate has the power to execute said proposal through the Delegate Proposal Module at any time up until expiration. Said execution would go through the DaoDao core module


# Smart Contract Risks
Proposal modules can route arbitrary messages to
the core module so we have to take special care.
I can classify risks into these domains:
* Un-authorized delegation  
* Un-authorized execution
* Un-authorized revocation
* Multiple execution
* Expired execution 

Policy risks: 
* Forbidden revoking when policy enabled
* Does not preserve on failure when policy enabled


Of all the risks, un-authorized delegation will definitely
compromise the core module.
