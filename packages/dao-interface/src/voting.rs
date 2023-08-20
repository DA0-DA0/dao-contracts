use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use cw2::ContractVersion;

#[cw_serde]
#[derive(QueryResponses)]
pub enum Query {
    /// Returns the token contract address, if set.
    #[returns(::cosmwasm_std::Addr)]
    TokenContract {},
    /// Returns the voting power for an address at a given height.
    #[returns(VotingPowerAtHeightResponse)]
    VotingPowerAtHeight {
        address: ::std::string::String,
        height: ::std::option::Option<::std::primitive::u64>,
    },
    /// Returns the total voting power at a given block heigh.
    #[returns(TotalPowerAtHeightResponse)]
    TotalPowerAtHeight {
        height: ::std::option::Option<::std::primitive::u64>,
    },
    /// Returns the address of the DAO this module belongs to.
    #[returns(cosmwasm_std::Addr)]
    Dao {},
    /// Returns contract version info.
    #[returns(InfoResponse)]
    Info {},
    /// Whether the DAO is active or not.
    #[returns(::std::primitive::bool)]
    IsActive {},
}

#[cw_serde]
pub struct VotingPowerAtHeightResponse {
    pub power: Uint128,
    pub height: u64,
}

#[cw_serde]
pub struct TotalPowerAtHeightResponse {
    pub power: Uint128,
    pub height: u64,
}

#[cw_serde]
pub struct InfoResponse {
    pub info: ContractVersion,
}

#[cw_serde]
pub struct IsActiveResponse {
    pub active: bool,
}
