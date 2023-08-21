use cosmwasm_std::Coin;
use cw_tokenfactory_issuer::{
    msg::{
        AllowanceResponse, AllowancesResponse, BlacklisteesResponse, BlacklisterAllowancesResponse,
        DenomResponse, ExecuteMsg, FreezerAllowancesResponse, InstantiateMsg, IsFrozenResponse,
        Metadata, MigrateMsg, OwnerResponse, QueryMsg, StatusResponse,
    },
    ContractError,
};
use osmosis_test_tube::{
    osmosis_std::types::{
        cosmos::bank::v1beta1::{MsgSend, MsgSendResponse},
        cosmwasm::wasm::v1::{
            MsgExecuteContractResponse, MsgMigrateContract, MsgMigrateContractResponse,
        },
        osmosis::tokenfactory::v1beta1::QueryDenomAuthorityMetadataRequest,
    },
    Account, Bank, Module, OsmosisTestApp, Runner, RunnerError, RunnerExecuteResult,
    SigningAccount, TokenFactory, Wasm,
};
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Debug)]
pub struct TokenfactoryIssuer<'a> {
    pub app: &'a OsmosisTestApp,
    pub code_id: u64,
    pub contract_addr: String,
}

impl<'a> TokenfactoryIssuer<'a> {
    pub fn new(
        app: &'a OsmosisTestApp,
        instantiate_msg: &InstantiateMsg,
        signer: &SigningAccount,
    ) -> Result<Self, RunnerError> {
        let wasm = Wasm::new(app);
        let token_creation_fee = Coin::new(10000000, "uosmo");

        let code_id = wasm
            .store_code(&Self::get_wasm_byte_code(), None, signer)?
            .data
            .code_id;

        let contract_addr = wasm
            .instantiate(
                code_id,
                &instantiate_msg,
                Some(&signer.address()),
                None,
                &[token_creation_fee],
                signer,
            )?
            .data
            .address;

        Ok(Self {
            app,
            code_id,
            contract_addr,
        })
    }

    pub fn new_with_values(
        app: &'a OsmosisTestApp,
        code_id: u64,
        contract_addr: String,
    ) -> Result<Self, RunnerError> {
        Ok(Self {
            app,
            code_id,
            contract_addr,
        })
    }

    /// uploads contract and returns a code ID
    pub fn upload(app: &OsmosisTestApp, signer: &SigningAccount) -> Result<u64, RunnerError> {
        let wasm = Wasm::new(app);

        let code_id = wasm
            .store_code(&Self::get_wasm_byte_code(), None, signer)?
            .data
            .code_id;

        Ok(code_id)
    }

    // executes
    pub fn execute(
        &self,
        execute_msg: &ExecuteMsg,
        funds: &[Coin],
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        let wasm = Wasm::new(self.app);
        wasm.execute(&self.contract_addr, execute_msg, funds, signer)
    }

    pub fn change_contract_owner(
        &self,
        new_owner: &str,
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        self.execute(
            &ExecuteMsg::ChangeContractOwner {
                new_owner: new_owner.to_string(),
            },
            &[],
            signer,
        )
    }

    pub fn change_tokenfactory_admin(
        &self,
        new_admin: &str,
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        self.execute(
            &ExecuteMsg::ChangeTokenFactoryAdmin {
                new_admin: new_admin.to_string(),
            },
            &[],
            signer,
        )
    }

    pub fn set_denom_metadata(
        &self,
        metadata: Metadata,
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        self.execute(&ExecuteMsg::SetDenomMetadata { metadata }, &[], signer)
    }

    pub fn set_minter(
        &self,
        address: &str,
        allowance: u128,
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        self.execute(
            &ExecuteMsg::SetMinter {
                address: address.to_string(),
                allowance: allowance.into(),
            },
            &[],
            signer,
        )
    }

    pub fn mint(
        &self,
        address: &str,
        amount: u128,
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        self.execute(
            &ExecuteMsg::Mint {
                to_address: address.to_string(),
                amount: amount.into(),
            },
            &[],
            signer,
        )
    }

    pub fn set_burner(
        &self,
        address: &str,
        allowance: u128,
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        self.execute(
            &ExecuteMsg::SetBurner {
                address: address.to_string(),
                allowance: allowance.into(),
            },
            &[],
            signer,
        )
    }
    pub fn burn(
        &self,
        address: &str,
        amount: u128,
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        self.execute(
            &ExecuteMsg::Burn {
                from_address: address.to_string(),
                amount: amount.into(),
            },
            &[],
            signer,
        )
    }

    pub fn set_freezer(
        &self,
        address: &str,
        status: bool,
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        self.execute(
            &ExecuteMsg::SetFreezer {
                address: address.to_string(),
                status,
            },
            &[],
            signer,
        )
    }

    pub fn set_blacklister(
        &self,
        address: &str,
        status: bool,
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        self.execute(
            &ExecuteMsg::SetBlacklister {
                address: address.to_string(),
                status,
            },
            &[],
            signer,
        )
    }

    pub fn freeze(
        &self,
        status: bool,
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        self.execute(&ExecuteMsg::Freeze { status }, &[], signer)
    }

    pub fn blacklist(
        &self,
        address: &str,
        status: bool,
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgExecuteContractResponse> {
        self.execute(
            &ExecuteMsg::Blacklist {
                address: address.to_string(),
                status,
            },
            &[],
            signer,
        )
    }

    // queries
    pub fn query<T>(&self, query_msg: &QueryMsg) -> Result<T, RunnerError>
    where
        T: DeserializeOwned,
    {
        let wasm = Wasm::new(self.app);
        wasm.query(&self.contract_addr, query_msg)
    }

    pub fn query_denom(&self) -> Result<DenomResponse, RunnerError> {
        self.query(&QueryMsg::Denom {})
    }

    pub fn query_is_freezer(&self, address: &str) -> Result<StatusResponse, RunnerError> {
        self.query(&QueryMsg::IsFreezer {
            address: address.to_string(),
        })
    }

    pub fn query_is_blacklister(&self, address: &str) -> Result<StatusResponse, RunnerError> {
        self.query(&QueryMsg::IsBlacklister {
            address: address.to_string(),
        })
    }

    pub fn query_freezer_allowances(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<FreezerAllowancesResponse, RunnerError> {
        self.query(&QueryMsg::FreezerAllowances { start_after, limit })
    }

    pub fn query_blacklister_allowances(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<BlacklisterAllowancesResponse, RunnerError> {
        self.query(&QueryMsg::BlacklisterAllowances { start_after, limit })
    }

    pub fn query_blacklistees(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<BlacklisteesResponse, RunnerError> {
        self.query(&QueryMsg::Blacklistees { start_after, limit })
    }

    pub fn query_is_frozen(&self) -> Result<IsFrozenResponse, RunnerError> {
        self.query(&QueryMsg::IsFrozen {})
    }

    pub fn query_is_blacklisted(&self, address: &str) -> Result<StatusResponse, RunnerError> {
        self.query(&QueryMsg::IsBlacklisted {
            address: address.to_string(),
        })
    }
    pub fn query_owner(&self) -> Result<OwnerResponse, RunnerError> {
        self.query(&QueryMsg::Owner {})
    }
    pub fn query_mint_allowance(&self, address: &str) -> Result<AllowanceResponse, RunnerError> {
        self.query(&QueryMsg::MintAllowance {
            address: address.to_string(),
        })
    }

    pub fn query_mint_allowances(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<AllowancesResponse, RunnerError> {
        self.query(&QueryMsg::MintAllowances { start_after, limit })
    }

    pub fn query_burn_allowance(&self, address: &str) -> Result<AllowanceResponse, RunnerError> {
        self.query(&QueryMsg::BurnAllowance {
            address: address.to_string(),
        })
    }

    pub fn query_burn_allowances(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<AllowancesResponse, RunnerError> {
        self.query(&QueryMsg::BurnAllowances { start_after, limit })
    }

    pub fn migrate(
        &self,
        testdata: &str,
        signer: &SigningAccount,
    ) -> RunnerExecuteResult<MsgMigrateContractResponse> {
        let wasm = Wasm::new(self.app);
        let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let wasm_byte_code =
            std::fs::read(manifest_path.join("tests").join("testdata").join(testdata)).unwrap();

        let code_id = wasm.store_code(&wasm_byte_code, None, signer)?.data.code_id;
        self.app.execute(
            MsgMigrateContract {
                sender: signer.address(),
                contract: self.contract_addr.clone(),
                code_id,
                msg: serde_json::to_vec(&MigrateMsg {}).unwrap(),
            },
            "/cosmwasm.wasm.v1.MsgMigrateContract",
            signer,
        )
    }

    fn get_wasm_byte_code() -> Vec<u8> {
        let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        println!("MANIFEST {:?}", manifest_path);
        std::fs::read(
            manifest_path
                .join("..")
                .join("..")
                .join("target")
                .join("wasm32-unknown-unknown")
                .join("release")
                .join("cw_tokenfactory_issuer.wasm"),
        )
        .unwrap()
    }

    pub fn execute_error(err: ContractError) -> RunnerError {
        RunnerError::ExecuteError {
            msg: format!(
                "failed to execute message; message index: 0: {}: execute wasm contract failed",
                err
            ),
        }
    }
}
