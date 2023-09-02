# dao-migrator

[![dao-migrator on crates.io](https://img.shields.io/crates/v/dao-migrator.svg?logo=rust)](https://crates.io/crates/dao-migrator)
[![docs.rs](https://img.shields.io/docsrs/dao-migrator?logo=docsdotrs)](https://docs.rs/dao-migrator/latest/dao_migrator/)

Here is the [discussion](https://github.com/DA0-DA0/dao-contracts/discussions/607).

A migrator module for a DAO DAO DAO which handles migration for DAO modules 
and test it went successfully.

DAO core migration is handled by a proposal, which adds this module and do
init callback to do migration on all regsitered modules.
If custom module is found, this TX fails and migration is cancelled, custom
module requires a custom migration to be done by the DAO.

# General idea
1. Proposal is made to migrate DAO core to V2, which also adds this module to the DAO.
2. On init of this contract, a callback is fired to do the migration.
3. Then we check to make sure the DAO doesn't have custom modules.
4. We query the state before migration
5. We do the migration
6. We query the new state and test it to make sure everything is correct.
7. In any case where 1 migration fails, we fail the whole TX.

# Important notes
* custom modules cannot reliably be migrated by this contract, 
because of that we fail the process to avoid any unwanted results.

* If any module migration fails we fail the whole thing, 
this is to make sure that we either have a fully working V2,
or we do nothing and make sure the DAO is operational at any time.