use cosmwasm_std::{coin, Addr, Coin, Empty, StdResult, Uint128};
use cw_multi_test::{App, AppResponse, Contract, ContractWrapper, Executor};
use cw_utils::PaymentError;

use crate::{
    msg::InstantiateMsg,
    state::{Bounty, BountyStatus},
    ContractError,
};

pub struct Test {
    pub app: App,
    pub contract: Addr,
    pub owner: Addr,
    pub recipient: Addr,
}

const ATOM_DENOM: &str = "uatom";
const JUNO_DENOM: &str = "ujuno";

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
                    vec![coin(10000, JUNO_DENOM), coin(10000, ATOM_DENOM)],
                )
                .unwrap();
            router
                .bank
                .init_balance(
                    storage,
                    &recipient,
                    vec![coin(10000, JUNO_DENOM), coin(10000, ATOM_DENOM)],
                )
                .unwrap();
        });
        let code_id = app.store_code(bounty_countract());
        let contract = app
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
            contract,
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
        let res = self.app.execute_contract(
            self.owner.clone(),
            self.contract.clone(),
            &msg,
            send_funds,
        )?;
        Ok(res)
    }

    pub fn close(&mut self, id: u64) -> Result<AppResponse, anyhow::Error> {
        let msg = crate::msg::ExecuteMsg::Close { id };
        let res =
            self.app
                .execute_contract(self.owner.clone(), self.contract.clone(), &msg, &[])?;
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
        let res = self.app.execute_contract(
            self.owner.clone(),
            self.contract.clone(),
            &msg,
            send_funds,
        )?;
        Ok(res)
    }

    pub fn pay_out(&mut self, id: u64) -> Result<AppResponse, anyhow::Error> {
        let msg = crate::msg::ExecuteMsg::PayOut {
            id,
            recipient: self.recipient.to_string(),
        };
        let res =
            self.app
                .execute_contract(self.owner.clone(), self.contract.clone(), &msg, &[])?;
        Ok(res)
    }

    pub fn query(&self, id: u64) -> StdResult<crate::state::Bounty> {
        let msg = crate::msg::QueryMsg::Bounty { id };
        self.app.wrap().query_wasm_smart(&self.contract, &msg)
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

    let bounty_amount = 100;
    let balance_before = test
        .app
        .wrap()
        .query_balance(test.owner.clone(), JUNO_DENOM)
        .unwrap();
    // create bounty
    test.create(
        coin(bounty_amount, JUNO_DENOM),
        "title".to_string(),
        Some("description".to_string()),
        &[coin(bounty_amount, JUNO_DENOM)],
    )
    .unwrap();
    // - assert bounty
    let bounty = test.query(1).unwrap();
    assert_eq!(
        bounty,
        Bounty {
            id: 1,
            amount: coin(bounty_amount, JUNO_DENOM),
            title: "title".to_string(),
            description: Some("description".to_string()),
            status: BountyStatus::Open,
            created_at: test.app.block_info().time.seconds(),
            updated_at: None,
        }
    );
    // assert balance
    let balance_after = test
        .app
        .wrap()
        .query_balance(test.owner.clone(), JUNO_DENOM)
        .unwrap();
    assert!(
        balance_before.amount.u128() == (balance_after.amount.u128() + bounty_amount),
        "before: {}, after: {}",
        balance_before.amount.u128(),
        balance_after.amount.u128()
    );

    // create bounty without sending funds
    let err: ContractError = test
        .create(
            coin(bounty_amount, JUNO_DENOM),
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
            coin(bounty_amount, JUNO_DENOM),
            "title".to_string(),
            Some("description".to_string()),
            &[coin(bounty_amount / 2, JUNO_DENOM)],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::InvalidAmount {
            expected: Uint128::new(bounty_amount),
            actual: Uint128::new(bounty_amount / 2)
        }
    );

    // create bounty with bigger amount
    let err: ContractError = test
        .create(
            coin(bounty_amount, JUNO_DENOM),
            "title".to_string(),
            Some("description".to_string()),
            &[coin(2 * bounty_amount, JUNO_DENOM)],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(
        err,
        ContractError::InvalidAmount {
            expected: Uint128::new(bounty_amount),
            actual: Uint128::new(2 * bounty_amount)
        }
    );
}

#[test]
pub fn test_close_bounty() {
    let mut test = Test::new();
    let bounty_amount = 100;

    // create bounty
    test.create(
        coin(bounty_amount, JUNO_DENOM),
        "title".to_string(),
        Some("description".to_string()),
        &[coin(bounty_amount, JUNO_DENOM)],
    )
    .unwrap();

    // close bounty
    test.close(1).unwrap();
    // - assert bounty status
    let bounty = test.query(1).unwrap();
    assert_eq!(
        bounty.status,
        BountyStatus::Closed {
            closed_at: test.app.block_info().time.seconds(),
        }
    );

    // close bounty again
    let err: ContractError = test.close(1).unwrap_err().downcast().unwrap();
    assert_eq!(err, ContractError::NotOpen {});
}

#[test]
pub fn test_update_bounty() {
    let bounty_amount = 100;
    // case: update bounty with higher amount
    {
        let mut test = Test::new();
        let initial_juno_balance = test
            .app
            .wrap()
            .query_balance(test.owner.clone(), JUNO_DENOM)
            .unwrap();
        // create bounty
        test.create(
            coin(bounty_amount, JUNO_DENOM),
            "title".to_string(),
            Some("description".to_string()),
            &[coin(bounty_amount, JUNO_DENOM)],
        )
        .unwrap();

        test.update(
            1,
            coin(2 * bounty_amount, JUNO_DENOM),
            "title".to_string(),
            Some("description".to_string()),
            &[coin(bounty_amount, JUNO_DENOM)],
        )
        .unwrap();
        // - assert bounty
        let bounty = test.query(1).unwrap();
        assert_eq!(
            bounty,
            Bounty {
                id: 1,
                amount: coin(2 * bounty_amount, JUNO_DENOM),
                title: "title".to_string(),
                description: Some("description".to_string()),
                status: BountyStatus::Open,
                created_at: test.app.block_info().time.seconds(),
                updated_at: Some(test.app.block_info().time.seconds()),
            }
        );
        // assert balance
        let balance_after = test
            .app
            .wrap()
            .query_balance(test.owner.clone(), JUNO_DENOM)
            .unwrap();
        assert!(
            initial_juno_balance.amount.u128() == (balance_after.amount.u128() + 2 * bounty_amount),
            "before: {}, after: {}",
            initial_juno_balance.amount.u128(),
            balance_after.amount.u128()
        );
    }

    // case: update bounty with lower amount
    {
        let mut test = Test::new();
        let initial_juno_balance = test
            .app
            .wrap()
            .query_balance(test.owner.clone(), JUNO_DENOM)
            .unwrap();
        // create bounty
        test.create(
            coin(bounty_amount, JUNO_DENOM),
            "title".to_string(),
            Some("description".to_string()),
            &[coin(bounty_amount, JUNO_DENOM)],
        )
        .unwrap();

        test.update(
            1,
            coin(bounty_amount / 2, JUNO_DENOM),
            "title".to_string(),
            Some("description".to_string()),
            &[], // no funds needed, since update amount is lower
        )
        .unwrap();
        // - assert bounty
        let bounty = test.query(1).unwrap();
        assert_eq!(
            bounty,
            Bounty {
                id: 1,
                amount: coin(bounty_amount / 2, JUNO_DENOM),
                title: "title".to_string(),
                description: Some("description".to_string()),
                status: BountyStatus::Open,
                created_at: test.app.block_info().time.seconds(),
                updated_at: Some(test.app.block_info().time.seconds()),
            }
        );
        // assert balance
        let balance_after = test
            .app
            .wrap()
            .query_balance(test.owner.clone(), JUNO_DENOM)
            .unwrap();
        assert!(
            initial_juno_balance.amount.u128() == (balance_after.amount.u128() + bounty_amount / 2),
            "before: {}, after: {}",
            initial_juno_balance.amount.u128(),
            balance_after.amount.u128()
        );
    }

    // case: update bounty with lower amount + owner accidentally send funds
    {
        let mut test = Test::new();
        let initial_juno_balance = test
            .app
            .wrap()
            .query_balance(test.owner.clone(), JUNO_DENOM)
            .unwrap();
        // create bounty
        test.create(
            coin(bounty_amount, JUNO_DENOM),
            "title".to_string(),
            Some("description".to_string()),
            &[coin(bounty_amount, JUNO_DENOM)],
        )
        .unwrap();

        test.update(
            1,
            coin(bounty_amount / 2, JUNO_DENOM),
            "title".to_string(),
            Some("description".to_string()),
            &[coin(bounty_amount, JUNO_DENOM)],
        )
        .unwrap();
        // - assert bounty
        let bounty = test.query(1).unwrap();
        assert_eq!(
            bounty,
            Bounty {
                id: 1,
                amount: coin(bounty_amount / 2, JUNO_DENOM),
                title: "title".to_string(),
                description: Some("description".to_string()),
                status: BountyStatus::Open,
                created_at: test.app.block_info().time.seconds(),
                updated_at: Some(test.app.block_info().time.seconds()),
            }
        );
        // assert balance
        let balance_after = test
            .app
            .wrap()
            .query_balance(test.owner.clone(), JUNO_DENOM)
            .unwrap();
        assert!(
            initial_juno_balance.amount.u128() == (balance_after.amount.u128() + bounty_amount / 2),
            "before: {}, after: {}",
            initial_juno_balance.amount.u128(),
            balance_after.amount.u128()
        );
    }

    // case: update bounty sending incorrect funds
    {
        let mut test = Test::new();
        // create bounty
        test.create(
            coin(bounty_amount, JUNO_DENOM),
            "title".to_string(),
            Some("description".to_string()),
            &[coin(bounty_amount, JUNO_DENOM)],
        )
        .unwrap();
        let err: ContractError = test
            .update(
                1,
                coin(3 * bounty_amount, JUNO_DENOM),
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
                coin(2 * bounty_amount, JUNO_DENOM),
                "title".to_string(),
                Some("description".to_string()),
                &[coin(bounty_amount / 2, JUNO_DENOM)],
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(
            err,
            ContractError::InvalidAmount {
                expected: Uint128::new(bounty_amount),
                actual: Uint128::new(bounty_amount + bounty_amount / 2)
            }
        );

        // update bounty with bigger amount
        let err: ContractError = test
            .update(
                1,
                coin(2 * bounty_amount, JUNO_DENOM),
                "title".to_string(),
                Some("description".to_string()),
                &[coin(2 * bounty_amount, JUNO_DENOM)], // 100 already in bounty, now sending 200
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(
            err,
            ContractError::InvalidAmount {
                expected: Uint128::new(bounty_amount),
                actual: Uint128::new(3 * bounty_amount)
            }
        );
    }

    // case: update bounty with different denom
    {
        let mut test = Test::new();
        let initial_juno_balance = test
            .app
            .wrap()
            .query_balance(test.owner.clone(), JUNO_DENOM)
            .unwrap();
        let initial_atom_balance = test
            .app
            .wrap()
            .query_balance(test.owner.clone(), ATOM_DENOM)
            .unwrap();
        // create bounty
        test.create(
            coin(bounty_amount, JUNO_DENOM),
            "title".to_string(),
            Some("description".to_string()),
            &[coin(bounty_amount, JUNO_DENOM)],
        )
        .unwrap();

        test.update(
            1,
            coin(200, ATOM_DENOM),
            "title".to_string(),
            Some("description".to_string()),
            &[coin(200, ATOM_DENOM)],
        )
        .unwrap();
        // - assert bounty
        let bounty = test.query(1).unwrap();
        assert_eq!(
            bounty,
            Bounty {
                id: 1,
                amount: coin(2 * bounty_amount, ATOM_DENOM),
                title: "title".to_string(),
                description: Some("description".to_string()),
                status: BountyStatus::Open,
                created_at: test.app.block_info().time.seconds(),
                updated_at: Some(test.app.block_info().time.seconds()),
            }
        );
        // assert juno balance
        let juno_balance_after = test
            .app
            .wrap()
            .query_balance(test.owner.clone(), JUNO_DENOM)
            .unwrap();
        assert!(
            juno_balance_after.amount == initial_juno_balance.amount,
            "before: {}, after: {}",
            initial_juno_balance.amount.u128(),
            juno_balance_after.amount.u128()
        );
        // assert atom balance
        let atom_balance_after = test
            .app
            .wrap()
            .query_balance(test.owner.clone(), ATOM_DENOM)
            .unwrap();
        assert!(
            atom_balance_after.amount.u128() == (initial_atom_balance.amount.u128() - 200),
            "before: {}, after: {}",
            initial_atom_balance.amount.u128(),
            atom_balance_after.amount.u128()
        );
    }

    // case: update bounty with different denom, but owner sends with incorrect denom
    {
        let mut test = Test::new();
        // create bounty
        test.create(
            coin(bounty_amount, JUNO_DENOM),
            "title".to_string(),
            Some("description".to_string()),
            &[coin(bounty_amount, JUNO_DENOM)],
        )
        .unwrap();

        let err: ContractError = test
            .update(
                1,
                coin(200, ATOM_DENOM), // update with atom denom
                "title".to_string(),
                Some("description".to_string()),
                &[coin(200, JUNO_DENOM)], // but send juno denom
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(
            err,
            ContractError::PaymentError(PaymentError::MissingDenom(ATOM_DENOM.to_string()))
        );
    }

    // case: update on closed bounty
    {
        let mut test = Test::new();
        // create bounty
        test.create(
            coin(bounty_amount, JUNO_DENOM),
            "title".to_string(),
            Some("description".to_string()),
            &[coin(bounty_amount, JUNO_DENOM)],
        )
        .unwrap();

        test.close(1).unwrap();
        let err: ContractError = test
            .update(
                1,
                coin(2 * bounty_amount, JUNO_DENOM),
                "title".to_string(),
                Some("description".to_string()),
                &[coin(2 * bounty_amount, JUNO_DENOM)],
            )
            .unwrap_err()
            .downcast()
            .unwrap();
        assert_eq!(err, ContractError::NotOpen {});
    }
}

#[test]
pub fn test_pay_out_bounty() {
    let bounty_amount = 100;
    // case: payout
    {
        let mut test = Test::new();
        test.create(
            coin(bounty_amount, JUNO_DENOM),
            "title".to_string(),
            Some("description".to_string()),
            &[coin(bounty_amount, JUNO_DENOM)],
        )
        .unwrap();

        let initial_juno_balance = test
            .app
            .wrap()
            .query_balance(test.recipient.clone(), JUNO_DENOM)
            .unwrap();

        test.pay_out(1).unwrap();
        // assert balance
        let balance_after = test
            .app
            .wrap()
            .query_balance(test.recipient.clone(), JUNO_DENOM)
            .unwrap();
        assert!(
            initial_juno_balance.amount.u128() + bounty_amount == (balance_after.amount.u128()),
            "before: {}, after: {}",
            initial_juno_balance.amount.u128(),
            balance_after.amount.u128()
        );
        // assert bounty
        let bounty = test.query(1).unwrap();
        assert_eq!(
            bounty,
            Bounty {
                id: 1,
                amount: coin(bounty_amount, JUNO_DENOM),
                title: "title".to_string(),
                description: Some("description".to_string()),
                status: BountyStatus::Claimed {
                    claimed_by: test.recipient.to_string(),
                    claimed_at: test.app.block_info().time.seconds()
                },
                created_at: test.app.block_info().time.seconds(),
                updated_at: None,
            }
        );

        // - test bounty already claimed
        let err: ContractError = test.pay_out(1).unwrap_err().downcast().unwrap();
        assert_eq!(err, ContractError::NotOpen {});
    }

    // case: payout of closed bounty, this covered above, but just to be sure and test on manual close
    {
        let mut test = Test::new();
        test.create(
            coin(bounty_amount, JUNO_DENOM),
            "title".to_string(),
            Some("description".to_string()),
            &[coin(bounty_amount, JUNO_DENOM)],
        )
        .unwrap();
        test.close(1).unwrap();

        // - test bounty already claimed
        let err: ContractError = test.pay_out(1).unwrap_err().downcast().unwrap();
        assert_eq!(err, ContractError::NotOpen {});
    }
}
