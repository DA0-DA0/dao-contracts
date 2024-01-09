# cw-admin-factory

[![cw-admin-factory on crates.io](https://img.shields.io/crates/v/cw-admin-factory.svg?logo=rust)](https://crates.io/crates/cw-admin-factory)
[![docs.rs](https://img.shields.io/docsrs/cw-admin-factory?logo=docsdotrs)](https://docs.rs/cw-admin-factory/latest/cw_admin_factory/)

Serves as a factory that instantiates contracts and sets them as their
own wasm admins.

Useful for allowing contracts (e.g. DAOs) to migrate themselves.

Example instantiation flow:

![](https://bafkreibqsrdnht5chc5mdzbb6pgiyqfjke3yvukvjrokyefwwbl3k3iwaa.ipfs.nftstorage.link)

