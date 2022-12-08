# dao-core

This contract is the core module for all DAO DAO DAOs. It handles
management of voting power and proposal modules, executes messages,
and holds the DAO's treasury.

For more information about how these modules fit together see
[this wiki page](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-Contracts-Design).

In additon to the wiki spec this contract may also pause. To do so a
`Pause` message must by executed by a proposal module. Pausing the
core module will stop all actions on the module for the duration of
the pause.
