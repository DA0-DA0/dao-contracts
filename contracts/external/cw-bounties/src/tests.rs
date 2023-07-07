use cosmwasm_std::{coin, Addr, Coin, Empty, StdResult, Uint128};
use cw_multi_test::{App, AppResponse, Contract, ContractWrapper, Executor};
use cw_utils::PaymentError;

use crate::{msg::InstantiateMsg, state::BountyStatus, ContractError};

pub struct Test {
    pub app: App,
    pub addr: Addr,
    pub owner: Addr,
    pub recipient: Addr,
}

impl Test {
    pub fn new() -> Self {
        let owner = Addr::unchecked("owner");
        let recipient = Addr::unchecked("recipient");
        let mut app = App::new(|router, _, storage| {
            router
                .bank
                .init_balance(
                    storage,
                    &owner,
                    vec![coin(10000, "ujuno"), coin(10000, "uatom")],
                )
                .unwrap();
            router
                .bank
                .init_balance(
                    storage,
                    &recipient,
                    vec![coin(10000, "ujuno"), coin(10000, "uatom")],
                )
                .unwrap();
        });
        let code_id = app.store_code(bounty_countract());
        let addr = app
            .instantiate_contract(
                code_id,
                owner.clone(),
                &InstantiateMsg {
                    owner: owner.to_string(),
                },
                &[],
                "cw-bounties",
                None,
            )
            .unwrap();
        Self {
            app,
            addr,
            owner,
            recipient,
        }
    }

    pub fn create(
        &mut self,
        amount: Coin,
        title: String,
        description: Option<String>,
        send_funds: &[Coin],
    ) -> Result<AppResponse, anyhow::Error> {
        let msg = crate::msg::ExecuteMsg::Create {
            amount,
            title,
            description,
        };
        let res =
            self.app
                .execute_contract(self.owner.clone(), self.addr.clone(), &msg, send_funds)?;
        Ok(res)
    }

    pub fn close(&mut self, id: u64) -> Result<AppResponse, anyhow::Error> {
        let msg = crate::msg::ExecuteMsg::Close { id };
        let res = self
            .app
            .execute_contract(self.owner.clone(), self.addr.clone(), &msg, &[])?;
        Ok(res)
    }

    pub fn update(
        &mut self,
        id: u64,
        amount: Coin,
        title: String,
        description: Option<String>,
        send_funds: &[Coin],
    ) -> Result<AppResponse, anyhow::Error> {
        let msg = crate::msg::ExecuteMsg::Update {
            id,
            amount,
            title,
            description,
        };
        let res =
            self.app
                .execute_contract(self.owner.clone(), self.addr.clone(), &msg, send_funds)?;
        Ok(res)
    }

    pub fn pay_out(&mut self, id: u64) -> Result<AppResponse, anyhow::Error> {
        let msg = crate::msg::ExecuteMsg::PayOut {
            id,
            recipient: self.recipient.to_string(),
        };
        let res = self
            .app
            .execute_contract(self.owner.clone(), self.addr.clone(), &msg, &[])?;
        Ok(res)
    }

    pub fn query(&self, id: u64) -> StdResult<crate::state::Bounty> {
        let msg = crate::msg::QueryMsg::Bounty { id };
        self.app.wrap().query_wasm_smart(&self.addr, &msg)
    }
}

fn bounty_countract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

#[test]
pub fn test_create_bounty() {
    let mut test = Test::new();

    let balance_before = test
        .app
        .wrap()
        .query_balance(test.owner.clone(), "ujuno")
        .unwrap();
    // create bounty
    test.create(
        coin(100, "ujuno"),
        "title".to_string(),
        Some("description".to_string()),
        &[coin(100, "ujuno")],
    )
    .unwrap();
    // assert balance
    let balance_after = test
        .app
        .wrap()
        .query_balance(test.owner.clone(), "ujuno")
        .unwrap();
    assert!(
        balance_before.amount.u128() == (balance_after.amount.u128() + 100),
        "before: {}, after: {}",
        balance_before.amount.u128(),
        balance_after.amount.u128()
    );

    // create bounty without sending funds
    let err: ContractError = test
        .create(
            coin(100, "ujuno"),
            "title".to_string(),
            Some("description".to_string()),
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::PaymentError(PaymentError::NoFunds {}));

    // create bounty with lower amount
    let err: ContractError = test
        .create(
            coin(100, "ujuno"),
            "title".to_string(),
            Some("description".to_string()),
            &[coin(50, "ujuno")],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::InvalidAmount {
            expected: Uint128::new(100),
            actual: Uint128::new(50)
        }
    );

    // create bounty with bigger amount
    let err: ContractError = test
        .create(
            coin(100, "ujuno"),
            "title".to_string(),
            Some("description".to_string()),
            &[coin(150, "ujuno")],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::InvalidAmount {
            expected: Uint128::new(100),
            actual: Uint128::new(150)
        }
    );
}

#[test]
pub fn test_close_bounty() {
    let mut test = Test::new();

    // create bounty
    test.create(
        coin(100, "ujuno"),
        "title".to_string(),
        Some("description".to_string()),
        &[coin(100, "ujuno")],
    )
    .unwrap();

    // close bounty
    test.close(1).unwrap();

    // close bounty again
    let err: ContractError = test.close(1).unwrap_err().downcast().unwrap();
    assert_eq!(err, ContractError::NotOpen {});
}

#[test]
pub fn test_update_bounty() {
    let mut test = Test::new();

    let initial_juno_balance = test
        .app
        .wrap()
        .query_balance(test.owner.clone(), "ujuno")
        .unwrap();
    let initial_atom_balance = test
        .app
        .wrap()
        .query_balance(test.owner.clone(), "uatom")
        .unwrap();
    // create bounty
    test.create(
        coin(100, "ujuno"),
        "title".to_string(),
        Some("description".to_string()),
        &[coin(100, "ujuno")],
    )
    .unwrap();

    // update bounty
    test.update(
        1,
        coin(200, "ujuno"),
        "title".to_string(),
        Some("description".to_string()),
        &[coin(100, "ujuno")],
    )
    .unwrap();
    // assert balance
    let balance_after = test
        .app
        .wrap()
        .query_balance(test.owner.clone(), "ujuno")
        .unwrap();
    assert!(
        initial_juno_balance.amount.u128() == (balance_after.amount.u128() + 200),
        "before: {}, after: {}",
        initial_juno_balance.amount.u128(),
        balance_after.amount.u128()
    );

    // update bounty without sending funds
    let err: ContractError = test
        .update(
            1,
            coin(300, "ujuno"),
            "title".to_string(),
            Some("description".to_string()),
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::PaymentError(PaymentError::NoFunds {}));

    // update bounty with lower amount
    let err: ContractError = test
        .update(
            1,
            coin(300, "ujuno"),
            "title".to_string(),
            Some("description".to_string()),
            &[coin(50, "ujuno")],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::InvalidAmount {
            expected: Uint128::new(100),
            actual: Uint128::new(250)
        }
    );

    // update bounty with bigger amount
    let err: ContractError = test
        .update(
            1,
            coin(300, "ujuno"),
            "title".to_string(),
            Some("description".to_string()),
            &[coin(150, "ujuno")],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::InvalidAmount {
            expected: Uint128::new(100),
            actual: Uint128::new(350)
        }
    );

    // update bounty with different denom
    test.update(
        1,
        coin(200, "uatom"),
        "title".to_string(),
        Some("description".to_string()),
        &[coin(200, "uatom")],
    )
    .unwrap();
    // assert juno balance
    let juno_balance_after = test
        .app
        .wrap()
        .query_balance(test.owner.clone(), "ujuno")
        .unwrap();
    assert!(
        juno_balance_after.amount == initial_juno_balance.amount,
        "before: {}, after: {}",
        initial_juno_balance.amount.u128(),
        balance_after.amount.u128()
    );
    let atom_balance_after = test
        .app
        .wrap()
        .query_balance(test.owner.clone(), "uatom")
        .unwrap();
    assert!(
        atom_balance_after.amount.u128() == (initial_atom_balance.amount.u128() - 200),
        "before: {}, after: {}",
        initial_atom_balance.amount.u128(),
        atom_balance_after.amount.u128()
    );

    // test closed bounty
    test.close(1).unwrap();
    let err: ContractError = test
        .update(
            1,
            coin(200, "ujuno"),
            "title".to_string(),
            Some("description".to_string()),
            &[coin(200, "ujuno")],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(err, ContractError::NotOpen {});
}

#[test]
pub fn test_pay_out_bounty() {
    let mut test = Test::new();

    // create bounty
    test.create(
        coin(100, "ujuno"),
        "title".to_string(),
        Some("description".to_string()),
        &[coin(100, "ujuno")],
    )
    .unwrap();

    let initial_juno_balance = test
        .app
        .wrap()
        .query_balance(test.recipient.clone(), "ujuno")
        .unwrap();
    // pay out bounty
    test.pay_out(1).unwrap();
    // assert balance
    let balance_after = test
        .app
        .wrap()
        .query_balance(test.recipient.clone(), "ujuno")
        .unwrap();
    assert!(
        initial_juno_balance.amount.u128() + 100 == (balance_after.amount.u128()),
        "before: {}, after: {}",
        initial_juno_balance.amount.u128(),
        balance_after.amount.u128()
    );
    // assert bounty claimed
    let bounty = test.query(1).unwrap();
    assert_eq!(
        bounty.status,
        BountyStatus::Claimed {
            claimed_by: test.recipient.to_string(),
            claimed_at: test.app.block_info().time.seconds()
        }
    );

    // test bounty already claimed
    let err: ContractError = test.pay_out(1).unwrap_err().downcast().unwrap();
    assert_eq!(err, ContractError::NotOpen {});
}
