use cw_multi_test::App;

use super::setup::{execute_migration, init_dao_v1};

#[test]
fn test_migration_v1_v2() {
    let app = App::default();

    // ----
    // instantiate a v1 DAO
    // ----
    let (app, core_addr, proposal_addr, v1_code_ids) = init_dao_v1(app, None);

    let res = execute_migration(app, core_addr, proposal_addr, v1_code_ids).unwrap();
    println!("{:?}", res)
}
