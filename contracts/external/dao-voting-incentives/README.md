This contract enables DAOs to offer incentives for voting on DAO proposals. By rewarding active voters, DAOs can encourage greater community involvement and decision-making.

## Instantiate

To instantiate the contract, provide the following parameters:

- `owner`: The DAO sending this contract voting hooks.
- `denom`: The denomination of the tokens to distribute as rewards.
- `expiration`: The expiration of the voting incentives period, defining how long the incentives are active.

## Configuration

- This contract should be added as a `VoteHook` to either the `dao-proposal-single` or `dao-proposal-multiple` proposal modules.
- The DAO must be set as the `owner` of this contract to manage incentives and ownership.

If no votes are cast during the voting incentives period, then the contract's funds are sent to the `owner` on expiration.

Rewards for a user are determined as such: `reward(user) = votes(user) * contract's balance / total votes`

## Execute

- **VoteHook(VoteHookMsg)**: Triggered when a new vote is cast. This is used to track voting activity and allocate rewards accordingly.
- **Claim {}**: Allows voters to claim their rewards after expiration.
- **Expire {}**: Expires the voting incentives period, allowing voters to claim rewards.
- **UpdateOwnership(cw_ownable::Action)**: Updates the ownership of the contract. This can be used to transfer ownership or perform other ownership-related actions.
- **Receive(Cw20ReceiveMsg)**: Handles the receipt of CW20 tokens. This is necessary for managing CW20-based incentives.

## Query

- **Config {}**: Returns the configuration of the voting incentives.
- **Rewards { address: String }**: Queries the claimable rewards for a specific address.
- **ExpectedRewards { address: String }**: Estimates the expected rewards for a specific address, based on current votes.
