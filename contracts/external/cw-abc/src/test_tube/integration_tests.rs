use super::test_env::{TestEnv, TestEnvBuilder};
use osmosis_test_tube::{Account, Module, OsmosisTestApp, Wasm};
use token_bindings::Metadata;

#[test]
fn test_happy_path() {
    let app = OsmosisTestApp::new();

    let env = TestEnvBuilder::new();
    let TestEnv { abc, accounts, .. } = env.default_setup(&app);
    // TODO
}
