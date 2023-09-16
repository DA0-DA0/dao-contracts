use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct NftFactoryCallback {
    pub nft_contract: String,
}
