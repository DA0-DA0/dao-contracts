use cw_orch::prelude::*;
use dao_cw_orch::{
    DaoDaoCore, DaoGaugeAdapter, DaoGaugeAdapterGeneric, DaoGaugeOrchestrator, DaoProposalSingle,
    DaoVotingCw4,
};

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

/// DAO-cw4-voting w/ gauges
pub struct DaoDaoCw4Gauge<Chain> {
    pub dao_core: DaoDaoCore<Chain>,
    pub prop_single: DaoProposalSingle<Chain>,
    pub cw4_vote: DaoVotingCw4<Chain>,
    pub gauge_suite: GaugeSuite<Chain>,
    pub cw4_group: Option<u64>,
}

impl<Chain: CwEnv> DaoDaoCw4Gauge<Chain> {
    pub fn new(chain: Chain) -> DaoDaoCw4Gauge<Chain> {
        DaoDaoCw4Gauge::<Chain> {
            dao_core: DaoDaoCore::new("dao_dao_core", chain.clone()),
            prop_single: DaoProposalSingle::new("dao_prop_single", chain.clone()),
            cw4_vote: DaoVotingCw4::new("dao_cw4_voting", chain.clone()),
            gauge_suite: GaugeSuite::new(chain.clone()),
            cw4_group: None,
        }
    }
    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.dao_core.upload()?;
        self.prop_single.upload()?;
        self.cw4_vote.upload()?;
        self.gauge_suite.upload()?;
        Ok(())
    }
}
