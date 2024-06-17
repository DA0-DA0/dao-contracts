use std::borrow::BorrowMut;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, coins, to_json_binary, Addr, Coin, Empty, Timestamp, Uint128};
use cw20::{Cw20Coin, Expiration, UncheckedDenom};
use cw20_stake::msg::ReceiveMsg;
use cw4::{Member, MemberListResponse};
use cw_multi_test::{App, BankSudo, Executor, SudoMsg};
use cw_ownable::{Action, Ownership};
use cw_utils::Duration;

use crate::{
    msg::{
        ExecuteMsg, InstantiateMsg, PendingRewardsResponse, QueryMsg, RewardEmissionRate,
        RewardsStateResponse,
    },
    state::DenomRewardState,
    testing::cw20_setup::instantiate_cw20,
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

#[cw_serde]
pub struct RewardsConfig {
    pub amount: u128,
    pub denom: UncheckedDenom,
    pub duration: Duration,
    pub destination: Option<String>,
}

pub struct SuiteBuilder {
    pub _instantiate: InstantiateMsg,
    pub dao_type: DaoType,
    pub rewards_config: RewardsConfig,
}

impl SuiteBuilder {
    pub fn base(dao_type: DaoType) -> Self {
        Self {
            _instantiate: InstantiateMsg {
                owner: Some(OWNER.to_string()),
            },
            dao_type,
            rewards_config: RewardsConfig {
                amount: 1_000,
                denom: UncheckedDenom::Native(DENOM.to_string()),
                duration: Duration::Height(10),
                destination: None,
            },
        }
    }

    pub fn with_rewards_config(mut self, rewards_config: RewardsConfig) -> Self {
        self.rewards_config = rewards_config;
        self
    }

    pub fn with_withdraw_destination(mut self, withdraw_destination: Option<String>) -> Self {
        self.rewards_config.destination = withdraw_destination;
        self
    }
}

impl SuiteBuilder {
    pub fn build(mut self) -> Suite {
        let owner = Addr::unchecked(OWNER);

        let mut suite_built = Suite {
            app: App::default(),
            owner: Some(owner.clone()),
            staking_addr: Addr::unchecked(""),
            voting_power_addr: Addr::unchecked(""),
            distribution_contract: Addr::unchecked(""),
            cw20_addr: Addr::unchecked(""),
            reward_denom: DENOM.to_string(),
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
        let reward_addr = suite_built
            .app
            .borrow_mut()
            .instantiate_contract(
                reward_code_id,
                owner.clone(),
                &InstantiateMsg {
                    owner: Some(owner.clone().into_string()),
                },
                &[],
                "reward",
                None,
            )
            .unwrap();
        suite_built.distribution_contract = reward_addr.clone();

        // depending on the dao type we register rewards differently
        match self.dao_type {
            DaoType::CW721 => {
                suite_built.register_hook(suite_built.voting_power_addr.clone());
                suite_built.register_reward_denom(
                    self.rewards_config.clone(),
                    suite_built.voting_power_addr.to_string().as_ref(),
                );
                match self.rewards_config.denom {
                    UncheckedDenom::Native(_) => {
                        suite_built.fund_distributor_native(coin(100_000_000, DENOM.to_string()));
                    }
                    UncheckedDenom::Cw20(_) => {
                        suite_built.fund_distributor_cw20(Cw20Coin {
                            address: suite_built.cw20_addr.to_string(),
                            amount: Uint128::new(100_000_000),
                        });
                    }
                };
            }
            _ => {
                self.rewards_config.denom = match self.rewards_config.denom {
                    UncheckedDenom::Native(denom) => UncheckedDenom::Native(denom),
                    UncheckedDenom::Cw20(_) => UncheckedDenom::Cw20(
                        instantiate_cw20(
                            suite_built.app.borrow_mut(),
                            "rewardcw",
                            vec![Cw20Coin {
                                address: OWNER.to_string(),
                                amount: Uint128::new(1_000_000_000),
                            }],
                        )
                        .to_string(),
                    ),
                };
                suite_built.reward_denom = match self.rewards_config.denom.clone() {
                    UncheckedDenom::Native(denom) => denom,
                    UncheckedDenom::Cw20(addr) => addr,
                };

                suite_built.register_hook(suite_built.staking_addr.clone());
                suite_built.register_reward_denom(
                    self.rewards_config.clone(),
                    suite_built.staking_addr.to_string().as_ref(),
                );
                match &self.rewards_config.denom {
                    UncheckedDenom::Native(_) => {
                        suite_built.fund_distributor_native(coin(100_000_000, DENOM.to_string()));
                    }
                    UncheckedDenom::Cw20(addr) => {
                        suite_built.fund_distributor_cw20(Cw20Coin {
                            address: addr.to_string(),
                            amount: Uint128::new(100_000_000),
                        });
                    }
                };
            }
        }

        println!("voting power addr: {}", suite_built.voting_power_addr);
        println!("staking addr: {}", suite_built.staking_addr);
        suite_built
    }
}

pub struct Suite {
    pub app: App,
    pub owner: Option<Addr>,

    pub staking_addr: Addr,
    pub voting_power_addr: Addr,
    pub reward_denom: String,

    pub distribution_contract: Addr,

    // cw20 type fields
    pub cw20_addr: Addr,
}

// SUITE QUERIES
impl Suite {
    pub fn get_time_until_rewards_expiration(&mut self) -> u64 {
        let rewards_state_response = self.get_rewards_state_response();
        let current_block = self.app.block_info();
        let (expiration_unit, current_unit) = match rewards_state_response.rewards[0].ends_at {
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

    pub fn get_rewards_state_response(&mut self) -> RewardsStateResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.distribution_contract.clone(),
                &QueryMsg::RewardsState {},
            )
            .unwrap()
    }

    pub fn _get_denom_reward_state(&mut self, denom: &str) -> DenomRewardState {
        let resp: DenomRewardState = self
            .app
            .wrap()
            .query_wasm_smart(
                self.distribution_contract.clone(),
                &QueryMsg::DenomRewardState {
                    denom: denom.to_string(),
                },
            )
            .unwrap();
        println!("[{} REWARD STATE] {:?}", denom, resp);
        resp
    }
}

// SUITE ASSERTIONS
impl Suite {
    pub fn assert_ends_at(&mut self, expected: Expiration) {
        let rewards_state_response = self.get_rewards_state_response();
        assert_eq!(rewards_state_response.rewards[0].ends_at, expected);
    }

    pub fn assert_started_at(&mut self, expected: Expiration) {
        let denom_configs = self.get_rewards_state_response();
        assert_eq!(denom_configs.rewards[0].started_at, expected);
    }

    pub fn assert_amount(&mut self, expected: u128) {
        let rewards_state_response = self.get_rewards_state_response();
        assert_eq!(
            rewards_state_response.rewards[0].emission_rate.amount,
            Uint128::new(expected)
        );
    }

    pub fn assert_duration(&mut self, expected: u64) {
        let rewards_state_response = self.get_rewards_state_response();
        let units = match rewards_state_response.rewards[0].emission_rate.duration {
            Duration::Height(h) => h,
            Duration::Time(t) => t,
        };
        assert_eq!(units, expected);
    }

    pub fn get_owner(&mut self) -> Addr {
        let ownable_response: cw_ownable::Ownership<Addr> = self
            .app
            .borrow_mut()
            .wrap()
            .query_wasm_smart(self.distribution_contract.clone(), &QueryMsg::Ownership {})
            .unwrap();
        ownable_response.owner.unwrap()
    }

    pub fn assert_pending_rewards(&mut self, address: &str, _denom: &str, expected: u128) {
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

        let pending = res.pending_rewards.get(self.reward_denom.as_str()).unwrap();

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

    pub fn assert_cw20_balance(&mut self, address: &str, expected: u128) {
        let balance = self.get_balance_cw20(self.reward_denom.clone(), address);
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

    pub fn register_reward_denom(&mut self, reward_config: RewardsConfig, hook_caller: &str) {
        let register_reward_denom_msg = ExecuteMsg::RegisterRewardDenom {
            denom: reward_config.denom.clone(),
            emission_rate: RewardEmissionRate {
                amount: Uint128::new(reward_config.amount),
                duration: reward_config.duration,
            },
            hook_caller: hook_caller.to_string(),
            vp_contract: self.voting_power_addr.to_string(),
            withdraw_destination: reward_config.destination,
        };

        self.app
            .borrow_mut()
            .execute_contract(
                self.owner.clone().unwrap(),
                self.distribution_contract.clone(),
                &register_reward_denom_msg,
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

    pub fn mint_cw20_coin(&mut self, coin: Cw20Coin, dest: &str, name: &str) -> Addr {
        let _msg = cw20::Cw20ExecuteMsg::Mint {
            recipient: dest.to_string(),
            amount: coin.amount,
        };
        cw20_setup::instantiate_cw20(self.app.borrow_mut(), name, vec![coin])
    }

    pub fn fund_distributor_native(&mut self, coin: Coin) {
        self.mint_native_coin(coin.clone(), OWNER);
        println!("[FUNDING EVENT] native funding: {}", coin);
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

    pub fn fund_distributor_cw20(&mut self, coin: Cw20Coin) {
        println!("[FUNDING EVENT] cw20 funding: {}", coin);

        let fund_sub_msg = to_json_binary(&ReceiveMsg::Fund {}).unwrap();
        self.app
            .execute_contract(
                Addr::unchecked(OWNER),
                Addr::unchecked(coin.address),
                &cw20::Cw20ExecuteMsg::Send {
                    contract: self.distribution_contract.to_string(),
                    amount: coin.amount,
                    msg: fund_sub_msg,
                },
                &[],
            )
            .unwrap();
    }

    pub fn skip_blocks(&mut self, blocks: u64) {
        self.app.borrow_mut().update_block(|b| {
            println!("skipping blocks {:?} -> {:?}", b.height, b.height + blocks);
            b.height += blocks
        });
    }

    pub fn skip_seconds(&mut self, seconds: u64) {
        self.app.borrow_mut().update_block(|b| {
            let new_block_time = b.time.plus_seconds(seconds);
            println!(
                "skipping seconds {:?} -> {:?}",
                b.time.seconds(),
                new_block_time.seconds()
            );
            b.time = new_block_time;
            // this is needed because voting power query only exists based on height.
            // for time-based unit tests we assume that 1 block = 1 second.
            // only implication I can think of is that during mainnet network downtime,
            // rewards would continue to accrue for time-based distributions, whereas
            // height-based distributions would not.
            b.height += seconds;
        });
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

    pub fn update_members(&mut self, add: Vec<Member>, remove: Vec<String>) {
        let msg = cw4_group::msg::ExecuteMsg::UpdateMembers { remove, add };

        self.app
            .execute_contract(Addr::unchecked(OWNER), self.staking_addr.clone(), &msg, &[])
            .unwrap();
    }

    pub fn query_members(&mut self) -> Vec<Member> {
        let members: MemberListResponse = self
            .app
            .wrap()
            .query_wasm_smart(
                self.staking_addr.clone(),
                &cw4_group::msg::QueryMsg::ListMembers {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        println!("[UPDATE CW4] new members: {:?}", members);
        members.members
    }

    pub fn update_owner(&mut self, new_owner: &str) {
        let msg = ExecuteMsg::UpdateOwnership(Action::TransferOwnership {
            new_owner: new_owner.to_string(),
            expiry: None,
        });

        self.app
            .execute_contract(
                Addr::unchecked(OWNER),
                self.distribution_contract.clone(),
                &msg,
                &[],
            )
            .unwrap();

        self.app
            .execute_contract(
                Addr::unchecked(new_owner),
                self.distribution_contract.clone(),
                &ExecuteMsg::UpdateOwnership(Action::AcceptOwnership {}),
                &[],
            )
            .unwrap();
    }
}
