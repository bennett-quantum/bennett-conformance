use bennett_cert_checker::rat::{parse_decimal, Interval, Rat};
use num_traits::{One, Zero};
fn r(s: &str) -> Rat {
    parse_decimal(s).unwrap()
}
#[test]
fn intervals_round_outward_across_signs_and_tiny_square_roots() {
    let third = Rat::new(1.into(), 3.into());
    let q = Interval::point(third.clone(), 8);
    assert!(q.lo <= third && q.hi >= third);
    let neg = Interval::point(-third.clone(), 8);
    assert_eq!(neg.lo, -q.hi);
    assert_eq!(neg.hi, -q.lo);
    let a = Interval {
        lo: r("-2"),
        hi: r("3"),
    };
    let b = Interval {
        lo: r("-5"),
        hi: r("7"),
    };
    let product = a.mul(&b, 64);
    assert_eq!(product.lo, r("-15"));
    assert_eq!(product.hi, r("21"));
    assert!(a
        .div(
            &Interval {
                lo: r("-1"),
                hi: r("1")
            },
            64
        )
        .is_none());
    for scale in [-400, -80, 0, 80, 400] {
        let v = bennett_cert_checker::rat::scale_pow2(&third, scale);
        let sq = Interval::exact(v.clone()).sqrt(64).unwrap();
        assert!(&sq.lo * &sq.lo <= v);
        assert!(&sq.hi * &sq.hi >= v);
        if !v.is_zero() {
            assert!(
                sq.lo > Rat::zero(),
                "relative-precision enclosure collapsed at exponent {scale}: {sq:?}"
            );
        }
    }
    let one = Interval::exact(Rat::one()).sqrt(64).unwrap();
    assert_eq!(one.lo, Rat::one());
    assert_eq!(one.hi, Rat::one());
}

use bennett_cert_checker::matrix::{verify_dual, ComplexInterval, Matrix};
fn real(v: &str) -> ComplexInterval {
    ComplexInterval::real(r(v))
}
#[test]
fn interval_frobenius_uses_full_matrix_and_preserves_global_phase() {
    let id = Matrix::identity(2);
    let x =
        Matrix::from_rows(vec![vec![real("0"), real("1")], vec![real("1"), real("0")]]).unwrap();
    assert_eq!(
        x.adjoint().mul(&x, 128).sub(&id, 128).frobenius_upper(128),
        Rat::zero()
    );
    let minus_id = Matrix::from_rows(vec![
        vec![real("-1"), real("0")],
        vec![real("0"), real("-1")],
    ])
    .unwrap();
    let bound = id
        .adjoint()
        .mul(&minus_id, 128)
        .sub(&id, 128)
        .frobenius_upper(128);
    assert!(bound >= r("2.828427124746190097603377448419396"));
    assert!(bound < r("2.828427124746190097603377448419397"));
}
#[test]
fn choi_and_partial_trace_use_unnormalized_input_first_convention() {
    let id = Matrix::identity(2);
    let j = id.choi(128);
    assert_eq!(j.dim, 4);
    assert_eq!(j.get(0, 3), &real("1"));
    assert_eq!(j.get(3, 0), &real("1"));
    assert_eq!(j.partial_trace_output(2, 128), id);
    let diag = Matrix::from_rows(vec![
        vec![real("1"), real("0"), real("0"), real("0")],
        vec![real("0"), real("2"), real("0"), real("0")],
        vec![real("0"), real("0"), real("3"), real("0")],
        vec![real("0"), real("0"), real("0"), real("4")],
    ])
    .unwrap();
    assert_eq!(diag.partial_trace_output(2, 128).get(0, 0), &real("3"));
    assert_eq!(diag.partial_trace_output(2, 128).get(1, 1), &real("7"));
}
#[test]
fn dual_requires_strictly_positive_interval_pivots() {
    let zero = Matrix::zeros(4);
    let y = Matrix::identity(4).scale(&Interval::exact(r("0.125")), 128);
    assert!(verify_dual(&zero, &y, &r("0.375"), &r("0.375"), 128).is_ok());
    assert!(
        verify_dual(&zero, &y, &r("0.25"), &r("0.25"), 128).is_err(),
        "zero slack pivot must reject"
    );
    assert!(
        verify_dual(&zero, &y, &r("0.375"), &r("0.374"), 128).is_err(),
        "mu exceeds bound"
    );
    let bad = Matrix::from_rows(vec![
        vec![real("0"), real("0")],
        vec![real("0"), real("-1")],
    ])
    .unwrap();
    assert!(
        bad.positive_definite(128).is_err(),
        "diag(0,-1) is the §12.1 negative witness"
    );
    // Hermitian positive diagonal alone is insufficient: [[1,2],[2,1]].
    let indefinite =
        Matrix::from_rows(vec![vec![real("1"), real("2")], vec![real("2"), real("1")]]).unwrap();
    assert!(indefinite.positive_definite(128).is_err());
}
