use cw_orch::prelude::*;
use dao_cw_orch::{DaoGaugeAdapter, DaoGaugeAdapterGeneric, DaoGaugeOrchestrator};

// gauge suite
pub struct GaugeSuite<Chain> {
    pub adapter: DaoGaugeAdapter<Chain>,
    pub orchestrator: DaoGaugeOrchestrator<Chain>,
    pub test_adapter: DaoGaugeAdapterGeneric<Chain>,
}

impl<Chain: CwEnv> GaugeSuite<Chain> {
    pub fn new(chain: Chain) -> GaugeSuite<Chain> {
        GaugeSuite::<Chain> {
            adapter: DaoGaugeAdapter::new("gauge_adapter", chain.clone()),
            orchestrator: DaoGaugeOrchestrator::new("gauge_orchestrator", chain.clone()),
            test_adapter: DaoGaugeAdapterGeneric::new("dao_gauge_adapter", chain.clone()),
        }
    }

    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.adapter.upload()?;
        self.orchestrator.upload()?;
        self.test_adapter.upload()?;
        Ok(())
    }
}
