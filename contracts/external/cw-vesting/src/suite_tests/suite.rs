use cosmwasm_std::{
    coins, testing::mock_env, Addr, BlockInfo, Decimal, Timestamp, Uint128, Uint64, Validator,
};
use cw_multi_test::{App, BankSudo, Executor, StakingInfo, StakingSudo};
use dao_testing::contracts::cw_vesting_contract;

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    vesting::{Schedule, Vest},
    StakeTrackerQuery,
};

pub(crate) struct Suite {
    app: App,
    pub owner: Option<Addr>,
    pub receiver: Addr,
    pub vesting: Addr,
    pub total: Uint128,
}

pub(crate) struct SuiteBuilder {
    pub instantiate: InstantiateMsg,
}

impl Default for SuiteBuilder {
    fn default() -> Self {
        // default multi-test staking setup.
        let staking_defaults = StakingInfo::default();

        Self {
            instantiate: InstantiateMsg {
                owner: Some("owner".to_string()),
                recipient: "recipient".to_string(),
                title: "title".to_string(),
                description: Some("description".to_string()),
                total: Uint128::new(100_000_000),
                denom: cw_denom::UncheckedDenom::Native(staking_defaults.bonded_denom),
                schedule: Schedule::SaturatingLinear,
                start_time: None,
                vesting_duration_seconds: 60 * 60 * 24 * 7, // one week
                unbonding_duration_seconds: staking_defaults.unbonding_time,
            },
        }
    }
}

impl SuiteBuilder {
    pub fn build(self) -> Suite {
        let mut app = App::new(|router, api, storage| {
            router
                .staking
                .add_validator(
                    api,
                    storage,
                    &mock_env().block,
                    Validator {
                        address: "validator".to_string(),
                        commission: Decimal::zero(), // zero percent comission to keep math simple.
                        max_commission: Decimal::percent(10),
                        max_change_rate: Decimal::percent(2),
                    },
                )
                .unwrap();
            router
                .staking
                .add_validator(
                    api,
                    storage,
                    &mock_env().block,
                    Validator {
                        address: "otherone".to_string(),
                        commission: Decimal::zero(), // zero percent comission to keep math simple.
                        max_commission: Decimal::percent(10),
                        max_change_rate: Decimal::percent(2),
                    },
                )
                .unwrap();
        });

        let funds = if let cw_denom::UncheckedDenom::Native(ref denom) = self.instantiate.denom {
            let funds = coins(self.instantiate.total.u128(), denom);
            app.sudo(
                BankSudo::Mint {
                    to_address: "owner".to_string(),
                    amount: funds.clone(),
                }
                .into(),
            )
            .unwrap();
            funds
        } else {
            vec![]
        };

        let vesting_id = app.store_code(cw_vesting_contract());
        let vesting = app
            .instantiate_contract(
                vesting_id,
                Addr::unchecked("owner"),
                &self.instantiate,
                &funds,
                "cw_vesting",
                self.instantiate.owner.clone(),
            )
            .unwrap();

        Suite {
            app,
            owner: self.instantiate.owner.map(Addr::unchecked),
            total: self.instantiate.total,
            receiver: Addr::unchecked(self.instantiate.recipient),
            vesting,
        }
    }

    pub fn with_start_time(mut self, t: Timestamp) -> Self {
        self.instantiate.start_time = Some(t);
        self
    }

    pub fn with_vesting_duration(mut self, duration_seconds: u64) -> Self {
        self.instantiate.vesting_duration_seconds = duration_seconds;
        self
    }

    pub fn with_curve(mut self, s: Schedule) -> Self {
        self.instantiate.schedule = s;
        self
    }
}

impl Suite {
    pub fn time(&self) -> Timestamp {
        self.app.block_info().time
    }

    pub fn a_second_passes(&mut self) {
        self.app.update_block(|b| b.time = b.time.plus_seconds(1))
    }

    pub fn a_day_passes(&mut self) {
        self.app
            .update_block(|b| b.time = b.time.plus_seconds(60 * 60 * 24))
    }

    pub fn a_week_passes(&mut self) {
        self.a_day_passes();
        self.a_day_passes();
        self.a_day_passes();
        self.a_day_passes();
        self.a_day_passes();
        self.a_day_passes();
        self.a_day_passes();
    }

    pub fn what_block_is_it(&self) -> BlockInfo {
        self.app.block_info()
    }

    pub fn slash(&mut self, percent: u64) {
        self.app
            .sudo(
                StakingSudo::Slash {
                    validator: "validator".to_string(),
                    percentage: Decimal::percent(percent),
                }
                .into(),
            )
            .unwrap();
    }

    pub fn process_unbonds(&mut self) {
        self.app.sudo(StakingSudo::ProcessQueue {}.into()).unwrap();
    }
}

// execute
impl Suite {
    pub fn distribute<S: Into<String>>(
        &mut self,
        sender: S,
        amount: Option<Uint128>,
    ) -> anyhow::Result<()> {
        self.app
            .execute_contract(
                Addr::unchecked(sender),
                self.vesting.clone(),
                &ExecuteMsg::Distribute { amount },
                &[],
            )
            .map(|_| ())
    }

    pub fn cancel<S: Into<String>>(&mut self, sender: S) -> anyhow::Result<()> {
        self.app
            .execute_contract(
                Addr::unchecked(sender),
                self.vesting.clone(),
                &ExecuteMsg::Cancel {},
                &[],
            )
            .map(|_| ())
    }

    pub fn delegate(&mut self, amount: Uint128) -> anyhow::Result<()> {
        self.app
            .execute_contract(
                self.receiver.clone(),
                self.vesting.clone(),
                &ExecuteMsg::Delegate {
                    validator: "validator".to_string(),
                    amount,
                },
                &[],
            )
            .map(|_| ())
    }

    pub fn redelegate(&mut self, amount: Uint128, to_other_one: bool) -> anyhow::Result<()> {
        let (src_validator, dst_validator) = if to_other_one {
            ("validator".to_string(), "otherone".to_string())
        } else {
            ("otherone".to_string(), "validator".to_string())
        };
        self.app
            .execute_contract(
                self.receiver.clone(),
                self.vesting.clone(),
                &ExecuteMsg::Redelegate {
                    src_validator,
                    dst_validator,
                    amount,
                },
                &[],
            )
            .map(|_| ())
    }

    pub fn undelegate<S: Into<String>>(
        &mut self,
        sender: S,
        amount: Uint128,
    ) -> anyhow::Result<()> {
        self.app
            .execute_contract(
                Addr::unchecked(sender),
                self.vesting.clone(),
                &ExecuteMsg::Undelegate {
                    validator: "validator".to_string(),
                    amount,
                },
                &[],
            )
            .map(|_| ())
    }

    pub fn withdraw_delegator_reward(&mut self, validator: &str) -> anyhow::Result<()> {
        self.app
            .execute_contract(
                self.receiver.clone(),
                self.vesting.clone(),
                &ExecuteMsg::WithdrawDelegatorReward {
                    validator: validator.to_string(),
                },
                &[],
            )
            .map(|_| ())
    }

    pub fn withdraw_canceled(&mut self, amount: Option<Uint128>) -> anyhow::Result<()> {
        self.app
            .execute_contract(
                // anyone may call this method on a canceled vesting contract
                Addr::unchecked("random"),
                self.vesting.clone(),
                &ExecuteMsg::WithdrawCanceledPayment { amount },
                &[],
            )
            .map(|_| ())
    }

    pub fn set_withdraw_address<S: Into<String>>(
        &mut self,
        sender: S,
        receiver: S,
    ) -> anyhow::Result<()> {
        self.app
            .execute_contract(
                Addr::unchecked(sender),
                self.vesting.clone(),
                &ExecuteMsg::SetWithdrawAddress {
                    address: receiver.into(),
                },
                &[],
            )
            .map(|_| ())
    }

    pub fn register_bonded_slash<S: Into<String>>(
        &mut self,
        sender: S,
        amount: Uint128,
        time: Timestamp,
    ) -> anyhow::Result<()> {
        self.app
            .execute_contract(
                Addr::unchecked(sender),
                self.vesting.clone(),
                &ExecuteMsg::RegisterSlash {
                    validator: "validator".to_string(),
                    time,
                    amount,
                    during_unbonding: false,
                },
                &[],
            )
            .map(|_| ())
    }

    pub fn register_unbonding_slash<S: Into<String>>(
        &mut self,
        sender: S,
        amount: Uint128,
        time: Timestamp,
    ) -> anyhow::Result<()> {
        self.app
            .execute_contract(
                Addr::unchecked(sender),
                self.vesting.clone(),
                &ExecuteMsg::RegisterSlash {
                    validator: "validator".to_string(),
                    time,
                    amount,
                    during_unbonding: true,
                },
                &[],
            )
            .map(|_| ())
    }
}

// query
impl Suite {
    pub fn query_vest(&self) -> Vest {
        self.app
            .wrap()
            .query_wasm_smart(&self.vesting, &QueryMsg::Info {})
            .unwrap()
    }

    pub fn query_distributable(&self) -> Uint128 {
        self.app
            .wrap()
            .query_wasm_smart(&self.vesting, &QueryMsg::Distributable { t: None })
            .unwrap()
    }

    pub fn query_receiver_vesting_token_balance(&self) -> Uint128 {
        let vest = self.query_vest();
        self.query_vesting_token_balance(vest.recipient)
    }

    pub fn query_vesting_token_balance<S: Into<String>>(&self, who: S) -> Uint128 {
        let vest = self.query_vest();
        vest.denom
            .query_balance(&self.app.wrap(), &Addr::unchecked(who.into()))
            .unwrap()
    }

    pub fn query_stake(&self, q: StakeTrackerQuery) -> Uint128 {
        self.app
            .wrap()
            .query_wasm_smart(&self.vesting, &QueryMsg::Stake(q))
            .unwrap()
    }

    pub fn query_vested(&self, t: Option<Timestamp>) -> Uint128 {
        self.app
            .wrap()
            .query_wasm_smart(&self.vesting, &QueryMsg::Vested { t })
            .unwrap()
    }

    pub fn query_total_to_vest(&self) -> Uint128 {
        self.app
            .wrap()
            .query_wasm_smart(&self.vesting, &QueryMsg::TotalToVest {})
            .unwrap()
    }

    pub fn query_duration(&self) -> Option<Uint64> {
        self.app
            .wrap()
            .query_wasm_smart(&self.vesting, &QueryMsg::VestDuration {})
            .unwrap()
    }
}
