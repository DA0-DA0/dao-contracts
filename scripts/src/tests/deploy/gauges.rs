use crate::gauges::*;
use cw_orch::prelude::*;

// gauge
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
            Box::new(&mut self.adapter)
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
