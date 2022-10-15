use cosmwasm_std::coins;

use osmosis_testing::{Account, OsmosisTestApp, RunnerError};
use tokenfactory_issuer::{msg::InstantiateMsg, ContractError};

mod helpers;
use helpers::{TestEnv, TokenfactoryIssuer};

// new denom

#[test]
fn transfer_token_factory_admin_by_contract_owner_should_pass() {
    let env = TestEnv::default();
    let owner = &env.test_accs[0];
    let new_admin = &env.test_accs[1];
    let denom = env.tokenfactory_issuer.query_denom().unwrap().denom;

    env.tokenfactory_issuer
        .change_tokenfactory_admin(&new_admin.address(), owner)
        .unwrap();

    assert_eq!(new_admin.address(), env.token_admin(&denom));
}

#[test]
fn transfer_token_factory_admin_by_non_contract_owner_should_fail() {
    let env = TestEnv::default();
    let non_owner = &env.test_accs[1];
    let someone_else = &env.test_accs[1];

    let err = env
        .tokenfactory_issuer
        .change_tokenfactory_admin(&someone_else.address(), non_owner)
        .unwrap_err();

    assert_eq!(
        err,
        TokenfactoryIssuer::execute_error(ContractError::Unauthorized {})
    )
}
