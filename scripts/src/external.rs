use cw_orch::prelude::*;
use dao_cw_orch::*;

// admin factory
pub struct AdminFactorySuite<Chain> {
    pub factory: DaoExternalAdminFactory<Chain>,
}
impl<Chain: CwEnv> AdminFactorySuite<Chain> {
    pub fn new(chain: Chain) -> AdminFactorySuite<Chain> {
        AdminFactorySuite::<Chain> {
            factory: DaoExternalAdminFactory::new("cw_admin_factory", chain.clone()),
        }
    }

    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.factory.upload()?;

        Ok(())
    }
}

// bitsong fantoken factory
pub struct FantokenFactorySuite<Chain> {
    pub factory: DaoExternalFantokenFactory<Chain>,
}

impl<Chain: CwEnv> FantokenFactorySuite<Chain> {
    pub fn new(chain: Chain) -> FantokenFactorySuite<Chain> {
        FantokenFactorySuite::<Chain> {
            factory: DaoExternalFantokenFactory::new("btsg_ft_factory", chain.clone()),
        }
    }

    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.factory.upload()?;

        Ok(())
    }
}

// payroll factory
pub struct PayrollSuite<Chain> {
    pub payroll: DaoExternalPayrollFactory<Chain>,
    pub vesting: DaoExternalCwVesting<Chain>,
}
impl<Chain: CwEnv> PayrollSuite<Chain> {
    pub fn new(chain: Chain) -> PayrollSuite<Chain> {
        PayrollSuite::<Chain> {
            payroll: DaoExternalPayrollFactory::new("cw_payroll", chain.clone()),
            vesting: DaoExternalCwVesting::new("cw_vesting", chain.clone()),
        }
    }

    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.payroll.upload()?;
        self.vesting.upload()?;
        Ok(())
    }
}

// cw tokenswap
pub struct TokenSwapSuite<Chain> {
    pub tokenswap: DaoExternalTokenSwap<Chain>,
}
impl<Chain: CwEnv> TokenSwapSuite<Chain> {
    pub fn new(chain: Chain) -> TokenSwapSuite<Chain> {
        TokenSwapSuite::<Chain> {
            tokenswap: DaoExternalTokenSwap::new("cw_tokenswap", chain.clone()),
        }
    }

    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.tokenswap.upload()?;

        Ok(())
    }
}

// cw-tokenfactory issuer
pub struct TokenFactorySuite<Chain> {
    pub tokenfactory: DaoExternalTokenfactoryIssuer<Chain>,
}
impl<Chain: CwEnv> TokenFactorySuite<Chain> {
    pub fn new(chain: Chain) -> TokenFactorySuite<Chain> {
        TokenFactorySuite::<Chain> {
            tokenfactory: DaoExternalTokenfactoryIssuer::new("cw_tokenfactory", chain.clone()),
        }
    }

    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.tokenfactory.upload()?;

        Ok(())
    }
}

// cw-vesting
pub struct VestingSuite<Chain> {
    pub vesting: DaoExternalCwVesting<Chain>,
}

impl<Chain: CwEnv> VestingSuite<Chain> {
    pub fn new(chain: Chain) -> VestingSuite<Chain> {
        VestingSuite::<Chain> {
            vesting: DaoExternalCwVesting::new("dao_dao_core", chain.clone()),
        }
    }

    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.vesting.upload()?;

        Ok(())
    }
}

// cw721 roles
pub struct Cw721RolesSuite<Chain> {
    pub roles: DaoExternalCw721Roles<Chain>,
}

impl<Chain: CwEnv> Cw721RolesSuite<Chain> {
    pub fn new(chain: Chain) -> Cw721RolesSuite<Chain> {
        Cw721RolesSuite::<Chain> {
            roles: DaoExternalCw721Roles::new("cw721_roles", chain.clone()),
        }
    }

    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.roles.upload()?;

        Ok(())
    }
}

// migrator
pub struct DaoMigrationSuite<Chain> {
    pub migrator: DaoExternalMigrator<Chain>,
}

impl<Chain: CwEnv> DaoMigrationSuite<Chain> {
    pub fn new(chain: Chain) -> DaoMigrationSuite<Chain> {
        DaoMigrationSuite::<Chain> {
            migrator: DaoExternalMigrator::new("dao_migrator", chain.clone()),
        }
    }

    pub fn upload(&self) -> Result<(), CwOrchError> {
        self.migrator.upload()?;

        Ok(())
    }
}
