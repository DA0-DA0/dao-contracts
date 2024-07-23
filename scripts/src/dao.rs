use cw_orch::prelude::*;
use dao_cw_orch::{DaoDaoCore, DaoProposalSingle, DaoProposalSudo};

// minimal dao
pub struct DaoDao<Chain> {
    pub dao_core: DaoDaoCore<Chain>,
    pub dao_proposal_single: DaoProposalSingle<Chain>,
    pub dao_proposal_sudo: DaoProposalSudo<Chain>,
}

impl<Chain: CwEnv> DaoDao<Chain> {
    pub fn new(chain: Chain) -> DaoDao<Chain> {
        DaoDao::<Chain> {
            dao_core: DaoDaoCore::new("dao_dao_core", chain.clone()),
            dao_proposal_single: DaoProposalSingle::new("dao_proposal_single", chain.clone()),
            dao_proposal_sudo: DaoProposalSudo::new("dao_proposal_sudo", chain.clone()),
        }
    }

    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.dao_core.upload()?;
        self.dao_proposal_single.upload()?;
        self.dao_proposal_sudo.upload()?;

        Ok(())
    }
}
