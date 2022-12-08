use cosmwasm_schema::write_api;
use cosmwasm_std::Empty;
use dao_pre_propose_base::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use dao_pre_propose_single::ProposeMessage;

fn main() {
    write_api! {
        instantiate: InstantiateMsg<Empty>,
        query: QueryMsg<Empty>,
        execute: ExecuteMsg<ProposeMessage, Empty>,
    }
}
