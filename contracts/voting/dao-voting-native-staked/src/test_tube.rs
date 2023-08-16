use cosmwasm_std::Coin;
use osmosis_test_tube::OsmosisTestApp;

#[test]
fn test_tube() {
    let app = OsmosisTestApp::new();

    let account = app.init_account(&[
        Coin::new(1_000_000_000_000, "uatom"),
        Coin::new(1_000_000_000_000, "uosmo"),
    ]);
}
