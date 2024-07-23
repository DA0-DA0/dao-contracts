use crate::external::*;
use cw_orch::prelude::*;
use cw_tokenfactory_issuer::msg::InstantiateMsg as TokenfactoryIssuerInit;

// admin factory
impl<Chain: CwEnv> cw_orch::contract::Deploy<Chain> for AdminFactorySuite<Chain> {
    // We don't have a custom error type
    type Error = CwOrchError;
    type DeployData = Addr;

    fn store_on(chain: Chain) -> Result<Self, Self::Error> {
        let factory = AdminFactorySuite::new(chain.clone());
        factory.upload()?;
        Ok(factory)
    }

    fn deployed_state_file_path() -> Option<String> {
        None
    }

    fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        vec![Box::new(&mut self.factory)]
    }

    fn load_from(chain: Chain) -> Result<Self, Self::Error> {
        let factory = Self::new(chain.clone());
        Ok(factory)
    }

    fn deploy_on(chain: Chain, _data: Self::DeployData) -> Result<Self, Self::Error> {
        // ########### Upload ##############
        let suite: AdminFactorySuite<Chain> = AdminFactorySuite::store_on(chain.clone()).unwrap();
        suite.factory.instantiate(
            &cw_admin_factory::msg::InstantiateMsg { admin: None },
            None,
            None,
        )?;
        Ok(suite)
    }
}

// payroll
impl<Chain: CwEnv> cw_orch::contract::Deploy<Chain> for PayrollSuite<Chain> {
    // We don't have a custom error type
    type Error = CwOrchError;
    type DeployData = Addr;

    fn store_on(chain: Chain) -> Result<Self, Self::Error> {
        let payroll = PayrollSuite::new(chain.clone());
        payroll.upload()?;

        Ok(payroll)
    }

    fn deployed_state_file_path() -> Option<String> {
        None
    }

    fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        vec![Box::new(&mut self.payroll)]
    }

    fn load_from(chain: Chain) -> Result<Self, Self::Error> {
        let payroll = Self::new(chain.clone());
        Ok(payroll)
    }

    fn deploy_on(chain: Chain, _data: Self::DeployData) -> Result<Self, Self::Error> {
        // ########### Upload ##############
        let suite: PayrollSuite<Chain> = PayrollSuite::store_on(chain.clone()).unwrap();
        // ########### Instantiate ##############
        let _init = suite.payroll.instantiate(
            &cw_payroll_factory::msg::InstantiateMsg {
                owner: Some(chain.sender_addr().to_string()),
                vesting_code_id: suite.vesting.code_id().unwrap(),
            },
            None,
            None,
        )?;

        Ok(suite)
    }
}

// tokenswap

impl<Chain: CwEnv> cw_orch::contract::Deploy<Chain> for TokenSwapSuite<Chain> {
    // We don't have a custom error type
    type Error = CwOrchError;
    type DeployData = Addr;

    fn store_on(chain: Chain) -> Result<Self, Self::Error> {
        let tokenswap = TokenSwapSuite::new(chain.clone());
        tokenswap.upload()?;
        Ok(tokenswap)
    }

    fn deployed_state_file_path() -> Option<String> {
        None
    }

    fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        vec![Box::new(&mut self.tokenswap)]
    }

    fn load_from(chain: Chain) -> Result<Self, Self::Error> {
        let tokenswap = Self::new(chain.clone());
        Ok(tokenswap)
    }

    fn deploy_on(chain: Chain, _data: Self::DeployData) -> Result<Self, Self::Error> {
        // ########### Upload ##############
        let suite: TokenSwapSuite<Chain> = TokenSwapSuite::store_on(chain.clone()).unwrap();
        Ok(suite)
    }
}

// tokenfactory issuer
impl<Chain: CwEnv> cw_orch::contract::Deploy<Chain> for TokenFactorySuite<Chain> {
    // We don't have a custom error type
    type Error = CwOrchError;
    type DeployData = Addr;

    fn store_on(chain: Chain) -> Result<Self, Self::Error> {
        let factory = TokenFactorySuite::new(chain.clone());
        factory.upload()?;
        Ok(factory)
    }

    fn deployed_state_file_path() -> Option<String> {
        None
    }

    fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        vec![Box::new(&mut self.tokenfactory)]
    }

    fn load_from(chain: Chain) -> Result<Self, Self::Error> {
        let factory = Self::new(chain.clone());
        Ok(factory)
    }

    fn deploy_on(chain: Chain, _data: Self::DeployData) -> Result<Self, Self::Error> {
        // ########### Upload ##############
        let suite: TokenFactorySuite<Chain> = TokenFactorySuite::store_on(chain.clone()).unwrap();
        // ########### Instantiate ##############
        let init = TokenfactoryIssuerInit::NewToken {
            subdenom: "DAOTOKEN".into(),
        };
        suite.tokenfactory.instantiate(&init, None, None)?;
        Ok(suite)
    }
}

// cw-vesting
impl<Chain: CwEnv> cw_orch::contract::Deploy<Chain> for VestingSuite<Chain> {
    // We don't have a custom error type
    type Error = CwOrchError;
    type DeployData = Addr;

    fn store_on(chain: Chain) -> Result<Self, Self::Error> {
        let vesting = VestingSuite::new(chain.clone());
        vesting.upload()?;
        Ok(vesting)
    }

    fn deployed_state_file_path() -> Option<String> {
        None
    }

    fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        vec![Box::new(&mut self.vesting)]
    }

    fn load_from(chain: Chain) -> Result<Self, Self::Error> {
        let vesting = Self::new(chain.clone());
        Ok(vesting)
    }

    fn deploy_on(chain: Chain, _data: Self::DeployData) -> Result<Self, Self::Error> {
        // ########### Upload ##############
        let suite: VestingSuite<Chain> = VestingSuite::store_on(chain.clone()).unwrap();
        Ok(suite)
    }
}

// cw721-roles
impl<Chain: CwEnv> cw_orch::contract::Deploy<Chain> for Cw721RolesSuite<Chain> {
    // We don't have a custom error type
    type Error = CwOrchError;
    type DeployData = Addr;

    fn store_on(chain: Chain) -> Result<Self, Self::Error> {
        let roles = Cw721RolesSuite::new(chain.clone());
        roles.upload()?;
        Ok(roles)
    }

    fn deployed_state_file_path() -> Option<String> {
        None
    }

    fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        vec![Box::new(&mut self.roles)]
    }

    fn load_from(chain: Chain) -> Result<Self, Self::Error> {
        let roles = Self::new(chain.clone());
        Ok(roles)
    }

    fn deploy_on(chain: Chain, _data: Self::DeployData) -> Result<Self, Self::Error> {
        // ########### Upload ##############
        let suite: Cw721RolesSuite<Chain> = Cw721RolesSuite::store_on(chain.clone()).unwrap();
        // ########### Instantiate ##############
        Ok(suite)
    }
}

// dao-migrator
impl<Chain: CwEnv> cw_orch::contract::Deploy<Chain> for DaoMigrationSuite<Chain> {
    // We don't have a custom error type
    type Error = CwOrchError;
    type DeployData = Addr;

    fn store_on(chain: Chain) -> Result<Self, Self::Error> {
        let migrator = DaoMigrationSuite::new(chain.clone());
        migrator.upload()?;
        Ok(migrator)
    }

    fn deployed_state_file_path() -> Option<String> {
        None
    }

    fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        vec![Box::new(&mut self.migrator)]
    }

    fn load_from(chain: Chain) -> Result<Self, Self::Error> {
        let migrator = Self::new(chain.clone());
        Ok(migrator)
    }

    fn deploy_on(chain: Chain, _data: Self::DeployData) -> Result<Self, Self::Error> {
        // ########### Upload ##############
        let suite: DaoMigrationSuite<Chain> = DaoMigrationSuite::store_on(chain.clone()).unwrap();
        // ########### Instantiate ##############

        Ok(suite)
    }
}

// bitsong fantoken factory
impl<Chain: CwEnv> cw_orch::contract::Deploy<Chain> for FantokenFactorySuite<Chain> {
    // We don't have a custom error type
    type Error = CwOrchError;
    type DeployData = Addr;

    fn store_on(chain: Chain) -> Result<Self, Self::Error> {
        let factory = FantokenFactorySuite::new(chain.clone());
        factory.upload()?;
        Ok(factory)
    }

    fn deployed_state_file_path() -> Option<String> {
        None
    }

    fn get_contracts_mut(&mut self) -> Vec<Box<&mut dyn ContractInstance<Chain>>> {
        vec![Box::new(&mut self.factory)]
    }

    fn load_from(chain: Chain) -> Result<Self, Self::Error> {
        let factory = Self::new(chain.clone());
        Ok(factory)
    }

    fn deploy_on(chain: Chain, _data: Self::DeployData) -> Result<Self, Self::Error> {
        // ########### Upload ##############
        let suite: FantokenFactorySuite<Chain> =
            FantokenFactorySuite::store_on(chain.clone()).unwrap();
        // ########### Instantiate ##############
        suite
            .factory
            .instantiate(&btsg_ft_factory::msg::InstantiateMsg {}, None, None);
        Ok(suite)
    }
}
