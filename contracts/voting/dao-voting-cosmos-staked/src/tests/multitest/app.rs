use std::ops::{Deref, DerefMut};

use crate::tests::multitest::stargate::StargateKeeper;
use cosmwasm_std::{
    testing::MockApi, Api, Empty, GovMsg, IbcMsg, IbcQuery, MemoryStorage, Storage,
};
use cw_multi_test::{
    no_init, App, AppBuilder, BankKeeper, DistributionKeeper, FailingModule, GovFailingModule,
    IbcFailingModule, Router, StakeKeeper, WasmKeeper,
};
#[allow(clippy::type_complexity)]
pub struct CustomApp(
    App<
        BankKeeper,
        MockApi,
        MemoryStorage,
        FailingModule<Empty, Empty, Empty>,
        WasmKeeper<Empty, Empty>,
        StakeKeeper,
        DistributionKeeper,
        FailingModule<IbcMsg, IbcQuery, Empty>,
        FailingModule<GovMsg, Empty, Empty>,
        StargateKeeper,
    >,
);
impl Deref for CustomApp {
    type Target = App<
        BankKeeper,
        MockApi,
        MemoryStorage,
        FailingModule<Empty, Empty, Empty>,
        WasmKeeper<Empty, Empty>,
        StakeKeeper,
        DistributionKeeper,
        FailingModule<IbcMsg, IbcQuery, Empty>,
        FailingModule<GovMsg, Empty, Empty>,
        StargateKeeper,
    >;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CustomApp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Default for CustomApp {
    fn default() -> Self {
        Self::new(no_init)
    }
}

impl CustomApp {
    pub fn new<F>(init_fn: F) -> Self
    where
        F: FnOnce(
            &mut Router<
                BankKeeper,
                FailingModule<Empty, Empty, Empty>,
                WasmKeeper<Empty, Empty>,
                StakeKeeper,
                DistributionKeeper,
                IbcFailingModule,
                GovFailingModule,
                StargateKeeper,
            >,
            &dyn Api,
            &mut dyn Storage,
        ),
    {
        let app_builder = AppBuilder::default();
        let stargate = StargateKeeper {};
        let app = app_builder.with_stargate(stargate).build(init_fn);
        CustomApp(app)
    }
}
