# Vote Hooks

This package provides methods for working with vote hooks which
fire when a vote is cast.

The hooks that fire here reply on error. Proposal modules may listen
for these replies and remove hook contracts that missbehave to prevent 
a bad hook receiver from locking the module.
