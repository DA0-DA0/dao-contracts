# cwd-core

This contract is the core module for all DAO DAO DAOs. It handles
management of voting power and proposal modules, executes messages,
and holds the DAO's treasury.

For more information about how these modules fit together see
[this](https://github.com/DA0-DA0/dao-contracts/wiki/DAO-DAO-v1-Contracts-Design)
wiki page.

In additon to the wiki spec this contract may also pause. To do so a
`Pause` message must by executed by a proposal module. Pausing the
core module will stop all actions on the module for the duration of
the pause.
