use cosmwasm_schema::{cw_serde, QueryResponses};
use cw20::Cw20ReceiveMsg;
use cw_ownable::cw_ownable;
use cw_vesting::msg::InstantiateMsg as PayrollInstantiateMsg;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Option<String>,
}

#[cw_ownable]
#[cw_serde]
pub enum ExecuteMsg {
    /// Instantiates a new vesting contract that is funded by a cw20 token.
    Receive(Cw20ReceiveMsg),
    /// Instantiates a new vesting contract that is funded by a native token.
    InstantiateNativePayrollContract {
        instantiate_msg: PayrollInstantiateMsg,
        code_id: u64,
        label: String,
    },
}

// Receiver setup
#[cw_serde]
pub enum ReceiveMsg {
    /// Funds a vesting contract with a cw20 token
    InstantiatePayrollContract {
        instantiate_msg: PayrollInstantiateMsg,
        code_id: u64,
        label: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns list of all vesting payment contracts
    #[returns(Vec<::cosmwasm_std::Addr>)]
    ListVestingContracts {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns list of all vesting payment contracts in reverse
    #[returns(Vec<::cosmwasm_std::Addr>)]
    ListVestingContractsReverse {
        start_before: Option<String>,
        limit: Option<u32>,
    },
    /// Returns list of all vesting payment contracts by who instantiated them
    #[returns(Vec<::cosmwasm_std::Addr>)]
    ListVestingContractsByInstantiator {
        instantiator: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns list of all vesting payment contracts by who instantiated them in reverse
    #[returns(Vec<::cosmwasm_std::Addr>)]
    ListVestingContractsByInstantiatorReverse {
        instantiator: String,
        start_before: Option<String>,
        limit: Option<u32>,
    },
    /// Returns list of all vesting payment contracts by recipient
    #[returns(Vec<::cosmwasm_std::Addr>)]
    ListVestingContractsByRecipient {
        recipient: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns list of all vesting payment contracts by recipient in reverse
    #[returns(Vec<::cosmwasm_std::Addr>)]
    ListVestingContractsByRecipientReverse {
        recipient: String,
        start_before: Option<String>,
        limit: Option<u32>,
    },
    /// Returns info about the contract ownership, if set
    #[returns(::cw_ownable::Ownership<::cosmwasm_std::Addr>)]
    Ownership {},
}

#[cw_serde]
pub struct MigrateMsg {}
