# CW-Name-Registry

Allows DAOs to register (at a cost) for a text name.

Query routes:

- `LookUpNameByDao { dao: String }`. Returns the name (`Option<String>`) owned by a given DAO address.
- `LookUpDaoByName { name: String }`. Returns a the DAO's address (`Option<String>`) that owns a given name.
- `IsNameAvailableToRegister { name: String }`. Returns true if the DAO's name is neither taken nor reserved.
