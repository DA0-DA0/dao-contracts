use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use cw2::ContractVersion;

#[cw_serde]
pub struct InfoResponse {
    pub info: ContractVersion,
}

#[cw_serde]
pub struct GenericProposalInfo {
    pub proposer: Addr,
    pub start_height: u64,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum Query {
    /// Returns the address of the DAO this module belongs to
    #[returns(::cosmwasm_std::Addr)]
    Dao {},
    /// Returns contract version info
    #[returns(InfoResponse)]
    Info {},
    /// Returns the proposal ID that will be assigned to the
    /// next proposal created.
    #[returns(::std::primitive::u64)]
    NextProposalId {},
    /// Returns generic proposal information
    #[returns(GenericProposalInfo)]
    GenericProposalInfo { proposal_id: ::std::primitive::u64 },
}

mod tests {
    /// Make sure the enum has all of the fields we expect. This will
    /// fail to compile if not.
    #[test]
    fn test_macro_expansion() {
        use super::Query;

        let query = Query::Info {};

        match query {
            Query::Dao {} => (),
            Query::Info {} => (),
            Query::NextProposalId {} => (),
            Query::GenericProposalInfo { proposal_id: _ } => (),
        }
    }
}
