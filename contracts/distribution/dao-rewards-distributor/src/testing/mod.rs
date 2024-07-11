use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub mod suite;
pub mod tests;

pub const DENOM: &str = "ujuno";
pub const ALT_DENOM: &str = "unotjuno";
pub const OWNER: &str = "owner";
pub const ADDR1: &str = "addr0001";
pub const ADDR2: &str = "addr0002";
pub const ADDR3: &str = "addr0003";

pub fn contract_rewards() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

mod cw4_setup {
    use cosmwasm_std::Addr;
    use cw4::Member;
    use cw_multi_test::{App, Executor};
    use dao_testing::contracts::{cw4_group_contract, dao_voting_cw4_contract};

    use super::OWNER;

    pub fn setup_cw4_test(app: &mut App, initial_members: Vec<Member>) -> (Addr, Addr) {
        let cw4_group_code_id = app.store_code(cw4_group_contract());
        let vp_code_id = app.store_code(dao_voting_cw4_contract());

        let msg = dao_voting_cw4::msg::InstantiateMsg {
            group_contract: dao_voting_cw4::msg::GroupContract::New {
                cw4_group_code_id,
                initial_members,
            },
        };

        let vp_addr = app
            .instantiate_contract(
                vp_code_id,
                Addr::unchecked(OWNER),
                &msg,
                &[],
                "cw4-vp",
                None,
            )
            .unwrap();

        let cw4_addr: Addr = app
            .wrap()
            .query_wasm_smart(
                vp_addr.clone(),
                &dao_voting_cw4::msg::QueryMsg::GroupContract {},
            )
            .unwrap();

        (vp_addr, cw4_addr)
    }
}

mod native_setup {
    use cosmwasm_std::{coins, Addr};
    use cw_multi_test::{App, Executor};
    use dao_testing::contracts::native_staked_balances_voting_contract;

    use super::{DENOM, OWNER};

    pub fn stake_tokenfactory_tokens(
        app: &mut App,
        staking_addr: &Addr,
        address: &str,
        amount: u128,
    ) {
        let msg = dao_voting_token_staked::msg::ExecuteMsg::Stake {};
        app.execute_contract(
            Addr::unchecked(address),
            staking_addr.clone(),
            &msg,
            &coins(amount, DENOM),
        )
        .unwrap();
    }

    pub fn unstake_tokenfactory_tokens(
        app: &mut App,
        staking_addr: &Addr,
        address: &str,
        amount: u128,
    ) {
        let msg = dao_voting_token_staked::msg::ExecuteMsg::Unstake {
            amount: amount.into(),
        };
        app.execute_contract(Addr::unchecked(address), staking_addr.clone(), &msg, &[])
            .unwrap();
    }

    pub fn setup_native_token_test(app: &mut App) -> Addr {
        let vp_code_id = app.store_code(native_staked_balances_voting_contract());

        let msg = dao_voting_token_staked::msg::InstantiateMsg {
            active_threshold: None,
            unstaking_duration: None,
            token_info: dao_voting_token_staked::msg::TokenInfo::Existing {
                denom: DENOM.to_string(),
            },
        };

        app.instantiate_contract(
            vp_code_id,
            Addr::unchecked(OWNER),
            &msg,
            &[],
            "native-vp",
            None,
        )
        .unwrap()
    }
}

mod cw20_setup {
    use cosmwasm_std::{to_json_binary, Addr, Uint128};
    use cw20::Cw20Coin;
    use cw_multi_test::{App, Executor};
    use cw_utils::Duration;
    use dao_testing::contracts::{
        cw20_base_contract, cw20_stake_contract, cw20_staked_balances_voting_contract,
    };

    use super::OWNER;

    pub fn instantiate_cw20(app: &mut App, name: &str, initial_balances: Vec<Cw20Coin>) -> Addr {
        let cw20_id = app.store_code(cw20_base_contract());
        let msg = cw20_base::msg::InstantiateMsg {
            name: name.to_string(),
            symbol: name.to_string(),
            decimals: 6,
            initial_balances,
            mint: None,
            marketing: None,
        };

        app.instantiate_contract(cw20_id, Addr::unchecked(OWNER), &msg, &[], "cw20", None)
            .unwrap()
    }

    pub fn instantiate_cw20_staking(
        app: &mut App,
        cw20: Addr,
        unstaking_duration: Option<Duration>,
    ) -> Addr {
        let staking_code_id = app.store_code(cw20_stake_contract());
        let msg = cw20_stake::msg::InstantiateMsg {
            owner: Some(OWNER.to_string()),
            token_address: cw20.to_string(),
            unstaking_duration,
        };
        app.instantiate_contract(
            staking_code_id,
            Addr::unchecked(OWNER),
            &msg,
            &[],
            "staking",
            None,
        )
        .unwrap()
    }

    pub fn instantiate_cw20_vp_contract(app: &mut App, cw20: Addr, staking_contract: Addr) -> Addr {
        let vp_code_id = app.store_code(cw20_staked_balances_voting_contract());
        let msg = dao_voting_cw20_staked::msg::InstantiateMsg {
            token_info: dao_voting_cw20_staked::msg::TokenInfo::Existing {
                address: cw20.to_string(),
                staking_contract: dao_voting_cw20_staked::msg::StakingInfo::Existing {
                    staking_contract_address: staking_contract.to_string(),
                },
            },
            active_threshold: None,
        };
        app.instantiate_contract(vp_code_id, Addr::unchecked(OWNER), &msg, &[], "vp", None)
            .unwrap()
    }

    pub fn setup_cw20_test(app: &mut App, initial_balances: Vec<Cw20Coin>) -> (Addr, Addr, Addr) {
        // Instantiate cw20 contract
        let cw20_addr = instantiate_cw20(app, "test", initial_balances.clone());

        // Instantiate staking contract
        let staking_addr = instantiate_cw20_staking(app, cw20_addr.clone(), None);

        // Instantiate vp contract
        let vp_addr = instantiate_cw20_vp_contract(app, cw20_addr.clone(), staking_addr.clone());

        (staking_addr, cw20_addr, vp_addr)
    }

    #[allow(dead_code)]
    pub fn stake_cw20_tokens<T: Into<String>>(
        app: &mut App,
        staking_addr: &Addr,
        cw20_addr: &Addr,
        sender: T,
        amount: u128,
    ) {
        let msg = cw20::Cw20ExecuteMsg::Send {
            contract: staking_addr.to_string(),
            amount: Uint128::new(amount),
            msg: to_json_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
        };
        app.execute_contract(Addr::unchecked(sender), cw20_addr.clone(), &msg, &[])
            .unwrap();
    }
}

mod cw721_setup {

    use cosmwasm_std::{to_json_binary, Addr, Binary, Empty};
    use cw_multi_test::{App, Executor};
    use dao_testing::contracts::{cw721_base_contract, cw721_staked_voting_contract};
    use dao_voting_cw721_staked::state::Config;

    use super::OWNER;

    pub fn stake_cw721(
        app: &mut App,
        vp_addr: &Addr,
        cw721_addr: &Addr,
        address: &str,
        token_id: &str,
    ) {
        let msg = cw721_base::msg::ExecuteMsg::<Empty, Empty>::SendNft {
            contract: vp_addr.to_string(),
            token_id: token_id.to_string(),
            msg: Binary::default(),
        };

        app.execute_contract(Addr::unchecked(address), cw721_addr.clone(), &msg, &[])
            .unwrap();
    }

    pub fn unstake_cw721(app: &mut App, vp_addr: &Addr, address: &str, token_id: &str) {
        app.execute_contract(
            Addr::unchecked(address),
            vp_addr.clone(),
            &dao_voting_cw721_staked::msg::ExecuteMsg::Unstake {
                token_ids: vec![token_id.to_string()],
            },
            &[],
        )
        .unwrap();
    }

    pub fn setup_cw721_test(app: &mut App, initial_nfts: Vec<Binary>) -> (Addr, Addr) {
        let cw721_code_id = app.store_code(cw721_base_contract());
        let vp_code_id = app.store_code(cw721_staked_voting_contract());

        let msg = dao_voting_cw721_staked::msg::InstantiateMsg {
            nft_contract: dao_voting_cw721_staked::msg::NftContract::New {
                code_id: cw721_code_id,
                label: "Test NFT contract".to_string(),
                msg: to_json_binary(&cw721_base::msg::InstantiateMsg {
                    name: "Test NFT".to_string(),
                    symbol: "TEST".to_string(),
                    minter: OWNER.to_string(),
                })
                .unwrap(),
                initial_nfts,
            },
            active_threshold: None,
            unstaking_duration: None,
        };

        let vp_addr = app
            .instantiate_contract(
                vp_code_id,
                Addr::unchecked(OWNER),
                &msg,
                &[],
                "cw721-vp",
                None,
            )
            .unwrap();

        let cw721_addr = app
            .wrap()
            .query_wasm_smart::<Config>(
                vp_addr.clone(),
                &dao_voting_cw721_staked::msg::QueryMsg::Config {},
            )
            .unwrap()
            .nft_address;

        (vp_addr, cw721_addr)
    }
}
