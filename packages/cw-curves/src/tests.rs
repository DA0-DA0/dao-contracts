// TODO: test DecimalPlaces return proper decimals

use cosmwasm_std::{Decimal as StdDecimal, Uint128};

use crate::{
    curves::{Constant, Linear, SquareRoot},
    utils::decimal,
    Curve, DecimalPlaces,
};

#[test]
fn constant_curve() {
    // supply is nstep (9), reserve is uatom (6)
    let normalize = DecimalPlaces::new(9, 6);
    let curve = Constant::new(decimal(15u128, 1), normalize);

    // do some sanity checks....
    // spot price is always 1.5 ATOM
    assert_eq!(
        StdDecimal::percent(150),
        curve.spot_price(Uint128::new(123))
    );

    // if we have 30 STEP, we should have 45 ATOM
    let reserve = curve.reserve(Uint128::new(30_000_000_000));
    assert_eq!(Uint128::new(45_000_000), reserve);

    // if we have 36 ATOM, we should have 24 STEP
    let supply = curve.supply(Uint128::new(36_000_000));
    assert_eq!(Uint128::new(24_000_000_000), supply);
}

#[test]
fn linear_curve() {
    // supply is usdt (2), reserve is btc (8)
    let normalize = DecimalPlaces::new(2, 8);
    // slope is 0.1 (eg hits 1.0 after 10btc)
    let curve = Linear::new(decimal(1u128, 1), normalize);

    // do some sanity checks....
    // spot price is 0.1 with 1 USDT supply
    assert_eq!(
        StdDecimal::permille(100),
        curve.spot_price(Uint128::new(100))
    );
    // spot price is 1.7 with 17 USDT supply
    assert_eq!(
        StdDecimal::permille(1700),
        curve.spot_price(Uint128::new(1700))
    );
    // spot price is 0.212 with 2.12 USDT supply
    assert_eq!(
        StdDecimal::permille(212),
        curve.spot_price(Uint128::new(212))
    );

    // if we have 10 USDT, we should have 5 BTC
    let reserve = curve.reserve(Uint128::new(1000));
    assert_eq!(Uint128::new(500_000_000), reserve);
    // if we have 20 USDT, we should have 20 BTC
    let reserve = curve.reserve(Uint128::new(2000));
    assert_eq!(Uint128::new(2_000_000_000), reserve);

    // if we have 1.25 BTC, we should have 5 USDT
    let supply = curve.supply(Uint128::new(125_000_000));
    assert_eq!(Uint128::new(500), supply);
    // test square root rounding
    // TODO: test when supply has many more decimal places than reserve
    // if we have 1.11 BTC, we should have 4.7116875957... USDT
    let supply = curve.supply(Uint128::new(111_000_000));
    assert_eq!(Uint128::new(471), supply);
}

#[test]
fn sqrt_curve() {
    // supply is utree (6) reserve is chf (2)
    let normalize = DecimalPlaces::new(6, 2);
    // slope is 0.35 (eg hits 0.35 after 1 chf, 3.5 after 100chf)
    let curve = SquareRoot::new(decimal(35u128, 2), normalize);

    // do some sanity checks....
    // spot price is 0.35 with 1 TREE supply
    assert_eq!(
        StdDecimal::percent(35),
        curve.spot_price(Uint128::new(1_000_000))
    );
    // spot price is 3.5 with 100 TREE supply
    assert_eq!(
        StdDecimal::percent(350),
        curve.spot_price(Uint128::new(100_000_000))
    );
    // spot price should be 23.478713763747788 with 4500 TREE supply (test rounding and reporting here)
    // rounds off around 8-9 sig figs (note diff for last points)
    assert_eq!(
        StdDecimal::from_ratio(2347871365u128, 100_000_000u128),
        curve.spot_price(Uint128::new(4_500_000_000))
    );

    // if we have 1 TREE, we should have 0.2333333333333 CHF
    let reserve = curve.reserve(Uint128::new(1_000_000));
    assert_eq!(Uint128::new(23), reserve);
    // if we have 100 TREE, we should have 233.333333333 CHF
    let reserve = curve.reserve(Uint128::new(100_000_000));
    assert_eq!(Uint128::new(23_333), reserve);
    // test rounding
    // if we have 235 TREE, we should have 840.5790828021146 CHF
    let reserve = curve.reserve(Uint128::new(235_000_000));
    assert_eq!(Uint128::new(84_057), reserve); // round down

    // // if we have 0.23 CHF, we should have 0.990453 TREE (round down)
    let supply = curve.supply(Uint128::new(23));
    assert_eq!(Uint128::new(990_000), supply);
    // if we have 840.58 CHF, we should have 235.000170 TREE (round down)
    let supply = curve.supply(Uint128::new(84058));
    assert_eq!(Uint128::new(235_000_000), supply);
}

// Idea: generic test that curve.supply(curve.reserve(supply)) == supply (or within some small rounding margin)
