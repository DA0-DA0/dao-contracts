use cosmwasm_std::{testing::mock_dependencies, Uint128};

use crate::Wormhole;

#[test]
fn test_increment() {
    let storage = &mut mock_dependencies().storage;
    let w: Wormhole<String, Uint128> = Wormhole::new("ns");

    w.increment(storage, "ekez".to_string(), 10, Uint128::new(1))
        .unwrap();
    // incrementing 9 shoud cause the value at 10 to become 3
    w.increment(storage, "ekez".to_string(), 9, Uint128::new(2))
        .unwrap();

    assert_eq!(w.load(storage, "ekez".to_string(), 8).unwrap(), None);
    assert_eq!(
        w.load(storage, "ekez".to_string(), 9).unwrap(),
        Some(Uint128::new(2))
    );
    assert_eq!(
        w.load(storage, "ekez".to_string(), 10).unwrap(),
        Some(Uint128::new(3))
    );
}

#[test]
fn test_decrement() {
    let storage = &mut mock_dependencies().storage;
    let w: Wormhole<u64, i32> = Wormhole::new("ns");

    w.increment(storage, 1, 11, 4).unwrap();
    w.increment(storage, 1, 10, 10).unwrap();

    w.decrement(storage, 1, 9, 4).unwrap();

    assert_eq!(w.load(storage, 1, 8).unwrap(), None);
    assert_eq!(w.load(storage, 1, 9).unwrap(), Some(-4));
    assert_eq!(w.load(storage, 1, 10).unwrap(), Some(6));
    assert_eq!(w.load(storage, 1, 11).unwrap(), Some(10));
}
