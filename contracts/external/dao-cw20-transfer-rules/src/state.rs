use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub enum Allowance {
    /// The address cannot send nor receive tokens.
    None,
    /// The address can send tokens to allowed recipients.
    Send,
    /// The address can send tokens to anyone, regardless of allowance.
    SendAnywhere,
    /// The address can receive tokens from allowed senders.
    Receive,
    /// The address can receive tokens from anyone, regardless of allowance.
    ReceiveAnywhere,
    /// The address can send/receive tokens to/from allowed recipients/senders.
    SendAndReceive,
    /// The address can send/receive tokens to/from anyone, regardless of
    /// allowance.
    SendAndReceiveAnywhere,
}

#[cw_serde]
pub struct Config {
    /// The DAO whose members may be able to send or receive tokens.
    pub dao: Addr,
    /// The allowance assigned to DAO members with no explicit allowance set. If
    /// None, members with no allowance set cannot send nor receive tokens.
    pub member_allowance: Allowance,
}

/// Config
pub const CONFIG: Item<Config> = Item::new("config");

/// Addresses with allowances that permit them to send, receive, or both.
pub const ALLOWANCES: Map<&Addr, Allowance> = Map::new("allowances");
