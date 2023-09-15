# CosmWasm DAO Hooks

This package provides shared hook functionality used for
[dao-hooks](../dao-hooks).

It deviates from other CosmWasm hook packages in that hooks can be
modified based on their index in the hook list AND based on the
address receiving the hook. This allows dispatching hooks with their
index as the reply ID of a submessage and removing hooks if they fail
to process the hook message.
