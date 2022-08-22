# Proposal Hooks

This package provides methods for working with proposal hooks which
fire when a proposal is created and when its status changes.

The hooks that fire here reply on error. Proposal modules may listen
for these replies and remove hook contracts that missbehave to prevent
a bad hook receiver from locking the module. 
