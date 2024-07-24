use cw_orch::prelude::*;
use dao_cw_orch::*;

// gauge suite
pub struct GaugeSuite<Chain> {
    pub adapter: DaoGaugeAdapter<Chain>,
    pub orchestrator: DaoGaugeOrchestrator<Chain>,
}

impl<Chain: CwEnv> GaugeSuite<Chain> {
    pub fn new(chain: Chain) -> GaugeSuite<Chain> {
        GaugeSuite::<Chain> {
            adapter: DaoGaugeAdapter::new("gauge_adapter", chain.clone()),
            orchestrator: DaoGaugeOrchestrator::new("gauge_orchestrator", chain.clone()),
        }
    }

    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.adapter.upload()?;
        self.orchestrator.upload()?;
        Ok(())
    }
}
