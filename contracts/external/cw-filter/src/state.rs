use cosmwasm_std::Addr;
use cw_storage_plus::Item;

/// The address of the protobuf registry, if any.
pub const PROTOBUF_REGISTRY: Item<Addr> = Item::new("protobuf_registry");
