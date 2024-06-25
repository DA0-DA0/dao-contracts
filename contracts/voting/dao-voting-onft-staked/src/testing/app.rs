use std::ops::{Deref, DerefMut};

use crate::testing::omniflix_stargate::StargateKeeper;
use cosmwasm_std::{testing::MockApi, Empty, GovMsg, IbcMsg, IbcQuery, MemoryStorage};
use cw_multi_test::{
    no_init, App, AppBuilder, BankKeeper, DistributionKeeper, FailingModule, StakeKeeper,
    WasmKeeper,
};
#[allow(clippy::type_complexity)]
pub struct OmniflixApp(
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
impl Deref for OmniflixApp {
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

impl DerefMut for OmniflixApp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Default for OmniflixApp {
    fn default() -> Self {
        Self::new()
    }
}

impl OmniflixApp {
    pub fn new() -> Self {
        let app_builder = AppBuilder::default();
        let stargate = StargateKeeper {};
        let app = app_builder.with_stargate(stargate).build(no_init);
        OmniflixApp(app)
    }
}
