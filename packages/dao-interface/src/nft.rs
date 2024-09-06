use cosmwasm_schema::cw_serde;

use crate::state::CallbackMessages;

#[cw_serde]
pub struct NftFactoryCallback {
    pub nft_contract: String,
    pub module_instantiate_callback: Option<CallbackMessages>,
}
