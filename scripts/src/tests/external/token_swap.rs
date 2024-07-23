use cw_orch::{anyhow, prelude::*};
use cw_token_swap::msg::{Counterparty, InstantiateMsg, TokenInfo};

use crate::{
    dao::TokenSwapSuite,
    tests::{ADMIN, DAO1, DENOM, PREFIX},
};
#[test]
fn test_tokenswap() -> anyhow::Result<()> {
    let mock = MockBech32::new(PREFIX);
    let admin = mock.addr_make(ADMIN);
    let app = TokenSwapSuite::deploy_on(mock.clone(), admin.clone())?;
    setup_tokenswap_helper(
        app,
        mock.sender.to_string(),
        mock.addr_make(DAO1).to_string(),
    )?;

    mock.next_block().unwrap();
    Ok(())
}

fn setup_tokenswap_helper(
    app: TokenSwapSuite<MockBech32>,
    sender: String,
    counterparty: String,
) -> anyhow::Result<()> {
    app.tokenswap.instantiate(
        &InstantiateMsg {
            counterparty_one: Counterparty {
                address: sender,
                promise: TokenInfo::Native {
                    denom: DENOM.to_string(),
                    amount: 1_000u128.into(),
                },
            },
            counterparty_two: Counterparty {
                address: counterparty,
                promise: TokenInfo::Native {
                    denom: DENOM.to_string(),
                    amount: 1_000u128.into(),
                },
            },
        },
        None,
        None,
    )?;
    Ok(())
}
