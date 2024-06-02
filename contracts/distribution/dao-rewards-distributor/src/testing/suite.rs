use std::borrow::BorrowMut;

use cosmwasm_std::{coin, coins, to_json_binary, Addr, Coin, Empty, Timestamp, Uint128};
use cw20::{Cw20Coin, Expiration};
use cw4::Member;
use cw_multi_test::{App, BankSudo, Executor, SudoMsg};
use cw_ownable::Ownership;
use cw_utils::Duration;

use crate::msg::{
    ExecuteMsg, InfoResponse, InstantiateMsg, PendingRewardsResponse, QueryMsg,
    RewardDenomRegistrationMsg, RewardEmissionConfig,
};

use super::{
    contract_rewards,
    cw20_setup::{self, setup_cw20_test},
    cw4_setup::setup_cw4_test,
    cw721_setup::{setup_cw721_test, stake_cw721, unstake_cw721},
    native_setup::{
        setup_native_token_test, stake_tokenfactory_tokens, unstake_tokenfactory_tokens,
    },
    ADDR1, ADDR2, ADDR3, DENOM, OWNER,
};

pub enum DaoType {
    CW20,
    CW721,
    Native,
    CW4,
}

pub struct SuiteBuilder {
    pub instantiate: InstantiateMsg,
    pub dao_type: DaoType,
    pub reward_funding: Coin,
}

impl SuiteBuilder {
    pub fn base(dao_type: DaoType) -> Self {
        Self {
            instantiate: InstantiateMsg {
                owner: Some(OWNER.to_string()),
            },
            dao_type,
            reward_funding: coin(100_000_000, DENOM.to_string()),
        }
    }
}

impl SuiteBuilder {
    pub fn build(self) -> Suite {
        let owner = Addr::unchecked(OWNER);

        let mut suite_built = Suite {
            app: App::default(),
            owner: Some(owner.clone()),
            staking_addr: Addr::unchecked(""),
            voting_power_addr: Addr::unchecked(""),
            distribution_contract: Addr::unchecked(""),
            cw20_addr: Addr::unchecked(""),
        };

        // start at 0 height and time
        suite_built.app.borrow_mut().update_block(|b| b.height = 0);
        suite_built
            .app
            .borrow_mut()
            .update_block(|b| b.time = Timestamp::from_seconds(0));

        match self.dao_type {
            DaoType::CW4 => {
                let members = vec![
                    Member {
                        addr: ADDR1.to_string(),
                        weight: 2,
                    },
                    Member {
                        addr: ADDR2.to_string(),
                        weight: 1,
                    },
                    Member {
                        addr: ADDR3.to_string(),
                        weight: 1,
                    },
                ];

                let (voting_power_addr, dao_voting_addr) =
                    setup_cw4_test(suite_built.app.borrow_mut(), members);
                suite_built.voting_power_addr = voting_power_addr.clone();
                suite_built.staking_addr = dao_voting_addr.clone();
            }
            DaoType::CW20 => {
                let initial_balances = vec![
                    Cw20Coin {
                        address: ADDR1.to_string(),
                        amount: Uint128::new(100),
                    },
                    Cw20Coin {
                        address: ADDR2.to_string(),
                        amount: Uint128::new(50),
                    },
                    Cw20Coin {
                        address: ADDR3.to_string(),
                        amount: Uint128::new(50),
                    },
                ];

                let (staking_addr, cw20_addr, vp_addr) =
                    setup_cw20_test(suite_built.app.borrow_mut(), initial_balances.clone());

                suite_built.voting_power_addr = vp_addr.clone();
                suite_built.cw20_addr = cw20_addr.clone();
                suite_built.staking_addr = staking_addr.clone();

                for coin in initial_balances.clone() {
                    suite_built.stake_cw20_tokens(coin.amount.u128(), coin.address.as_str());
                }
            }
            DaoType::CW721 => {
                let initial_nfts = vec![
                    to_json_binary(&cw721_base::msg::ExecuteMsg::<Empty, Empty>::Mint {
                        token_id: "1".to_string(),
                        owner: ADDR1.to_string(),
                        token_uri: Some("https://jpegs.com".to_string()),
                        extension: Empty {},
                    })
                    .unwrap(),
                    to_json_binary(&cw721_base::msg::ExecuteMsg::<Empty, Empty>::Mint {
                        token_id: "2".to_string(),
                        owner: ADDR1.to_string(),
                        token_uri: Some("https://jpegs.com".to_string()),
                        extension: Empty {},
                    })
                    .unwrap(),
                    to_json_binary(&cw721_base::msg::ExecuteMsg::<Empty, Empty>::Mint {
                        token_id: "3".to_string(),
                        owner: ADDR2.to_string(),
                        token_uri: Some("https://jpegs.com".to_string()),
                        extension: Empty {},
                    })
                    .unwrap(),
                    to_json_binary(&cw721_base::msg::ExecuteMsg::<Empty, Empty>::Mint {
                        token_id: "4".to_string(),
                        owner: ADDR3.to_string(),
                        token_uri: Some("https://jpegs.com".to_string()),
                        extension: Empty {},
                    })
                    .unwrap(),
                ];

                let (vp_addr, cw721) = setup_cw721_test(suite_built.app.borrow_mut(), initial_nfts);

                suite_built.voting_power_addr = vp_addr.clone();
                suite_built.staking_addr = cw721.clone();

                suite_built.stake_nft(ADDR1, 1);
                suite_built.stake_nft(ADDR1, 2);
                suite_built.stake_nft(ADDR2, 3);
                suite_built.stake_nft(ADDR3, 4);
            }
            DaoType::Native => {
                let initial_balances = vec![
                    (ADDR1, coins(100, DENOM)),
                    (ADDR2, coins(50, DENOM)),
                    (ADDR3, coins(50, DENOM)),
                ];

                // Mint tokens for initial balances
                for init_bal in initial_balances {
                    suite_built
                        .app
                        .borrow_mut()
                        .sudo(SudoMsg::Bank({
                            BankSudo::Mint {
                                to_address: init_bal.0.to_string(),
                                amount: init_bal.1,
                            }
                        }))
                        .unwrap();
                }

                // Create Native token staking contract
                let vp_addr = setup_native_token_test(suite_built.app.borrow_mut());
                suite_built.voting_power_addr = vp_addr.clone();
                suite_built.staking_addr = vp_addr.clone();
                suite_built.stake_native_tokens(ADDR1, 100);
                suite_built.stake_native_tokens(ADDR2, 50);
                suite_built.stake_native_tokens(ADDR3, 50);
            }
        };

        // initialize the rewards distributor
        let reward_code_id = suite_built.app.borrow_mut().store_code(contract_rewards());
        let msg = InstantiateMsg {
            owner: Some(owner.clone().into_string()),
        };
        let reward_addr = suite_built
            .app
            .borrow_mut()
            .instantiate_contract(reward_code_id, owner.clone(), &msg, &[], "reward", None)
            .unwrap();
        suite_built.distribution_contract = reward_addr.clone();

        if let DaoType::CW721 = self.dao_type {
            suite_built.register_hook(suite_built.voting_power_addr.clone());
            suite_built.register_reward_denom(
                DENOM,
                1000,
                10,
                suite_built.voting_power_addr.to_string().as_ref(),
            );
        } else {
            suite_built.register_hook(suite_built.staking_addr.clone());
            suite_built.register_reward_denom(
                DENOM,
                1000,
                10,
                suite_built.staking_addr.to_string().as_ref(),
            );
        }

        println!("voting power addr: {}", suite_built.voting_power_addr);
        println!("staking addr: {}", suite_built.staking_addr);

        suite_built.fund_distributor_native(coin(100_000_000, DENOM.to_string()));

        suite_built
    }

    #[allow(dead_code)]
    pub fn with_reward_funding(mut self, reward_funding: Coin) -> Self {
        self.reward_funding = reward_funding;
        self
    }
}

pub struct Suite {
    pub app: App,
    pub owner: Option<Addr>,

    pub staking_addr: Addr,
    pub voting_power_addr: Addr,

    pub distribution_contract: Addr,

    // cw20 type fields
    pub cw20_addr: Addr,
}

// SUITE QUERIES
impl Suite {
    pub fn get_time_until_rewards_expiration(&mut self) -> u64 {
        let info_response = self.get_info_response();
        let current_block = self.app.block_info();
        let (expiration_unit, current_unit) =
            match info_response.reward_configs[0].distribution_expiration {
                cw20::Expiration::AtHeight(h) => (h, current_block.height),
                cw20::Expiration::AtTime(t) => (t.seconds(), current_block.time.seconds()),
                cw20::Expiration::Never {} => return 0,
            };

        if expiration_unit > current_unit {
            expiration_unit - current_unit
        } else {
            0
        }
    }

    pub fn get_balance_native<T: Into<String>, U: Into<String>>(
        &self,
        address: T,
        denom: U,
    ) -> u128 {
        self.app
            .wrap()
            .query_balance(address, denom)
            .unwrap()
            .amount
            .u128()
    }

    #[allow(dead_code)]
    pub fn get_balance_cw20<T: Into<String>, U: Into<String>>(
        &self,
        contract_addr: T,
        address: U,
    ) -> u128 {
        let msg = cw20::Cw20QueryMsg::Balance {
            address: address.into(),
        };
        let result: cw20::BalanceResponse = self
            .app
            .wrap()
            .query_wasm_smart(contract_addr, &msg)
            .unwrap();
        result.balance.u128()
    }

    #[allow(dead_code)]
    pub fn get_ownership<T: Into<String>>(&mut self, address: T) -> Ownership<Addr> {
        self.app
            .wrap()
            .query_wasm_smart(address, &QueryMsg::Ownership {})
            .unwrap()
    }

    pub fn get_info_response(&mut self) -> InfoResponse {
        self.app
            .wrap()
            .query_wasm_smart(self.distribution_contract.clone(), &QueryMsg::Info {})
            .unwrap()
    }
}

// SUITE ASSERTIONS
impl Suite {
    pub fn assert_distribution_expiration(&mut self, expected: Expiration) {
        let info_response = self.get_info_response();
        assert_eq!(
            info_response.reward_configs[0].distribution_expiration,
            expected
        );
    }

    pub fn assert_period_start_date(&mut self, expected: Expiration) {
        let denom_configs = self.get_info_response();
        assert_eq!(denom_configs.reward_configs[0].period_start_date, expected);
    }

    pub fn assert_reward_rate_emission(&mut self, expected: u128) {
        let info_response = self.get_info_response();
        assert_eq!(
            info_response.reward_configs[0]
                .reward_emission_config
                .reward_rate_emission,
            Uint128::new(expected)
        );
    }

    pub fn assert_reward_rate_time(&mut self, expected: u64) {
        let info_response = self.get_info_response();
        let units = match info_response.reward_configs[0]
            .reward_emission_config
            .reward_rate_time
        {
            Duration::Height(h) => h,
            Duration::Time(t) => t,
        };
        assert_eq!(units, expected);
    }

    pub fn assert_pending_rewards(&mut self, address: &str, denom: &str, expected: u128) {
        let res: PendingRewardsResponse = self
            .app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(
                self.distribution_contract.clone(),
                &QueryMsg::GetPendingRewards {
                    address: address.to_string(),
                },
            )
            .unwrap();
        let pending = res.pending_rewards.get(denom).unwrap();

        assert_eq!(
            pending,
            &Uint128::new(expected),
            "expected {} pending rewards, got {}",
            expected,
            pending
        );
    }

    pub fn assert_native_balance(&mut self, address: &str, denom: &str, expected: u128) {
        let balance = self.get_balance_native(address, denom);
        assert_eq!(balance, expected);
    }
}

// SUITE ACTIONS
impl Suite {
    pub fn shutdown_denom_distribution(&mut self, denom: &str) {
        let msg = ExecuteMsg::Shutdown {
            denom: denom.to_string(),
        };
        self.app
            .execute_contract(
                Addr::unchecked(OWNER),
                self.distribution_contract.clone(),
                &msg,
                &[],
            )
            .unwrap();
    }

    pub fn register_hook(&mut self, addr: Addr) {
        let msg = cw4_group::msg::ExecuteMsg::AddHook {
            addr: self.distribution_contract.to_string(),
        };
        // TODO: cw721 check here
        self.app
            .execute_contract(Addr::unchecked(OWNER), addr, &msg, &[])
            .unwrap();
    }

    pub fn update_reward_emission_config(
        &mut self,
        denom: &str,
        reward_rate_emission: u128,
        reward_rate_time: Duration,
    ) {
        self.app
            .execute_contract(
                Addr::unchecked(OWNER),
                self.distribution_contract.clone(),
                &ExecuteMsg::UpdateRewardEmissionConfig {
                    denom: denom.to_string(),
                    new_emission_rate: reward_rate_emission.into(),
                    new_emission_time: reward_rate_time,
                },
                &[],
            )
            .unwrap();
    }

    pub fn register_reward_denom(
        &mut self,
        denom: &str,
        reward_rate_emission: u128,
        reward_rate_time: u64,
        hook_caller: &str,
    ) {
        let register_reward_denom_msg = RewardDenomRegistrationMsg {
            denom: cw20::UncheckedDenom::Native(denom.to_string()),
            reward_emission_config: RewardEmissionConfig {
                reward_rate_emission: Uint128::new(reward_rate_emission),
                reward_rate_time: Duration::Height(reward_rate_time),
            },
            hook_caller: hook_caller.to_string(),
            vp_contract: self.voting_power_addr.to_string(),
        };

        self.app
            .borrow_mut()
            .execute_contract(
                self.owner.clone().unwrap(),
                self.distribution_contract.clone(),
                &ExecuteMsg::RegisterRewardDenom(register_reward_denom_msg),
                &[],
            )
            .unwrap();
    }

    pub fn mint_native_coin(&mut self, coin: Coin, dest: &str) {
        // mint the tokens to be funded
        self.app
            .borrow_mut()
            .sudo(SudoMsg::Bank({
                BankSudo::Mint {
                    to_address: dest.to_string(),
                    amount: vec![coin.clone()],
                }
            }))
            .unwrap();
    }

    #[allow(dead_code)]
    pub fn mint_cw20_coin(&mut self, coin: Cw20Coin, dest: &str) -> Addr {
        let _msg = cw20::Cw20ExecuteMsg::Mint {
            recipient: dest.to_string(),
            amount: coin.amount,
        };
        cw20_setup::instantiate_cw20(self.app.borrow_mut(), "newcoin", vec![coin])
    }

    pub fn fund_distributor_native(&mut self, coin: Coin) {
        self.mint_native_coin(coin.clone(), OWNER);

        self.app
            .borrow_mut()
            .execute_contract(
                Addr::unchecked(OWNER),
                self.distribution_contract.clone(),
                &ExecuteMsg::Fund {},
                &[coin],
            )
            .unwrap();
    }

    pub fn skip_blocks(&mut self, blocks: u64) {
        self.app.borrow_mut().update_block(|b| b.height += blocks);
    }

    #[allow(dead_code)]
    pub fn skip_seconds(&mut self, seconds: u64) {
        self.app
            .borrow_mut()
            .update_block(|b| b.time = b.time.plus_seconds(seconds));
    }

    pub fn claim_rewards(&mut self, address: &str, denom: &str) {
        let msg = ExecuteMsg::Claim {
            denom: denom.to_string(),
        };

        self.app
            .execute_contract(
                Addr::unchecked(address),
                self.distribution_contract.clone(),
                &msg,
                &[],
            )
            .unwrap();
    }

    #[allow(dead_code)]
    pub fn stake_cw20_tokens(&mut self, amount: u128, sender: &str) {
        let msg = cw20::Cw20ExecuteMsg::Send {
            contract: self.staking_addr.to_string(),
            amount: Uint128::new(amount),
            msg: to_json_binary(&cw20_stake::msg::ReceiveMsg::Stake {}).unwrap(),
        };
        println!("[STAKING EVENT] {} staked {}", sender, amount);
        self.app
            .execute_contract(Addr::unchecked(sender), self.cw20_addr.clone(), &msg, &[])
            .unwrap();
    }

    pub fn unstake_cw20_tokens(&mut self, amount: u128, sender: &str) {
        let msg = cw20_stake::msg::ExecuteMsg::Unstake {
            amount: Uint128::new(amount),
        };
        println!("[STAKING EVENT] {} unstaked {}", sender, amount);
        self.app
            .execute_contract(
                Addr::unchecked(sender),
                self.staking_addr.clone(),
                &msg,
                &[],
            )
            .unwrap();
    }

    pub fn stake_nft(&mut self, sender: &str, token_id: u64) {
        stake_cw721(
            self.app.borrow_mut(),
            &self.voting_power_addr,
            &self.staking_addr,
            sender,
            &token_id.to_string(),
        )
    }

    pub fn unstake_nft(&mut self, sender: &str, token_id: u64) {
        unstake_cw721(
            self.app.borrow_mut(),
            &self.voting_power_addr,
            sender,
            &token_id.to_string(),
        )
    }

    pub fn stake_native_tokens(&mut self, address: &str, amount: u128) {
        stake_tokenfactory_tokens(self.app.borrow_mut(), &self.staking_addr, address, amount)
    }

    pub fn unstake_native_tokens(&mut self, address: &str, amount: u128) {
        unstake_tokenfactory_tokens(self.app.borrow_mut(), &self.staking_addr, address, amount)
    }
}
