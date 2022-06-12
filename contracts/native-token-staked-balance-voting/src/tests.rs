#[cfg(test)]
mod tests {
    use crate::msg::{InstantiateMsg, QueryMsg};
    use cosmwasm_std::{coin, coins, Addr, CosmosMsg, Empty, StakingMsg, Uint128};
    use cw_core_interface::voting::VotingPowerAtHeightResponse;
    use cw_multi_test::{App, AppBuilder, Contract, ContractWrapper, Executor};

    const DAO_ADDR: &str = "dao";
    const CREATOR_ADDR: &str = "creator";
    const DENOM: &str = "pebble";

    fn mock_app(owner: Addr, initial_balance: u128) -> App {
        AppBuilder::new().build(|router, _, storage| {
            router
                .bank
                .init_balance(storage, &owner, coins(initial_balance, DENOM))
                .unwrap();
        })
    }

    fn balance_voting_contract() -> Box<dyn Contract<Empty>> {
        let contract = ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        );
        Box::new(contract)
    }

    fn instantiate_voting(app: &mut App, voting_id: u64, msg: InstantiateMsg) -> Addr {
        app.instantiate_contract(
            voting_id,
            Addr::unchecked(DAO_ADDR),
            &msg,
            &[],
            "voting module",
            None,
        )
        .unwrap()
    }

    fn proper_instantiate(app: &mut App) -> Addr {
        let voting_id = app.store_code(balance_voting_contract());
        instantiate_voting(app, voting_id, InstantiateMsg {})
    }

    #[test]
    fn test_voting_power_at_height() {
        let owner = Addr::unchecked(CREATOR_ADDR);
        let mut app = mock_app(owner.clone(), 5000);
        let voting_addr = proper_instantiate(&mut app);
        let creator_voting_power: VotingPowerAtHeightResponse = app
            .wrap()
            .query_wasm_smart(
                voting_addr.clone(),
                &QueryMsg::VotingPowerAtHeight {
                    address: CREATOR_ADDR.to_string(),
                    height: None,
                },
            )
            .unwrap();

        assert_eq!(
            creator_voting_power,
            VotingPowerAtHeightResponse {
                power: Uint128::from(0u128),
                height: app.block_info().height,
            }
        );

        app.execute(
            Addr::unchecked(CREATOR_ADDR),
            CosmosMsg::Staking(StakingMsg::Delegate {
                validator: "sdf".into(),
                amount: coin(100, "Sdf"),
            }),
        )
        .unwrap();

        let creator_voting_power: VotingPowerAtHeightResponse = app
            .wrap()
            .query_wasm_smart(
                voting_addr.clone(),
                &QueryMsg::VotingPowerAtHeight {
                    address: CREATOR_ADDR.to_string(),
                    height: None,
                },
            )
            .unwrap();

        assert_eq!(
            creator_voting_power,
            VotingPowerAtHeightResponse {
                power: Uint128::from(100u128),
                height: app.block_info().height,
            }
        );
    }
}
