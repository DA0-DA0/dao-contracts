use crate::gauges::*;
use cw_orch::prelude::*;

// DAO-cw4 w/ gauges
impl<Chain: CwEnv> cw_orch::contract::Deploy<Chain> for DaoDaoCw4Gauge<Chain> {
    // We don't have a custom error type
    type Error = CwOrchError;
    type DeployData = Addr;

    fn store_on(chain: Chain) -> Result<Self, Self::Error> {
        let gauge = DaoDaoCw4Gauge::new(chain);
        gauge.upload()?;
        Ok(gauge)
    }

    fn deployed_state_file_path() -> Option<String> {
        None
    }

    fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        let mut cs: Vec<Box<&mut dyn ContractInstance<Chain>>> = vec![];
        let res: Vec<Box<&mut dyn ContractInstance<Chain>>> = vec![
            Box::new(&mut self.dao_core),
            Box::new(&mut self.prop_single),
            Box::new(&mut self.cw4_vote),
        ];

        cs.extend(res);
        cs.extend(self.gauge_suite.get_contracts_mut());
        cs
    }

    fn load_from(chain: Chain) -> Result<Self, Self::Error> {
        let gauge = Self::new(chain.clone());
        Ok(gauge)
    }

    fn deploy_on(chain: Chain, _data: Self::DeployData) -> Result<Self, Self::Error> {
        // ########### Upload ##############
        let suite: DaoDaoCw4Gauge<Chain> = DaoDaoCw4Gauge::store_on(chain.clone())?;
        Ok(suite)
    }
}

// Gauge Suite
impl<Chain: CwEnv> cw_orch::contract::Deploy<Chain> for GaugeSuite<Chain> {
    // We don't have a custom error type
    type Error = CwOrchError;
    type DeployData = Addr;

    fn store_on(chain: Chain) -> Result<Self, Self::Error> {
        let gauge = GaugeSuite::new(chain.clone());
        gauge.upload()?;
        Ok(gauge)
    }

    fn deployed_state_file_path() -> Option<String> {
        None
    }

    fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        vec![
            Box::new(&mut self.orchestrator),
            Box::new(&mut self.adapter),
        ]
    }

    fn load_from(chain: Chain) -> Result<Self, Self::Error> {
        let gauge = Self::new(chain.clone());
        Ok(gauge)
    }

    fn deploy_on(chain: Chain, _data: Self::DeployData) -> Result<Self, Self::Error> {
        // ########### Upload ##############
        let suite: GaugeSuite<Chain> = GaugeSuite::store_on(chain.clone())?;
        Ok(suite)
    }
}