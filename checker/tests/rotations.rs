//! Exact binary64 arguments and rigorous §17.8 Taylor enclosures.
use bennett_cert_checker::rat::{parse_decimal, scale_pow2, Rat};
use bennett_cert_checker::transcendental::{binary64_rational, pi, sin_cos};
use num_traits::{One, Zero};
fn r(s: &str) -> Rat {
    parse_decimal(s).unwrap()
}
#[test]
fn binary64_is_decoded_as_its_exact_rational_value() {
    assert_eq!(binary64_rational(0x3ff0000000000000).unwrap(), Rat::one());
    assert_eq!(binary64_rational(0xbff0000000000000).unwrap(), -Rat::one());
    assert_eq!(binary64_rational(0x3fe0000000000000).unwrap(), r("0.5"));
    assert_eq!(
        binary64_rational(0x0000000000000001).unwrap(),
        scale_pow2(&Rat::one(), -1074)
    );
    assert_eq!(binary64_rational(0x8000000000000000).unwrap(), Rat::zero());
    assert!(binary64_rational(0x7ff0000000000000).is_err());
    assert!(binary64_rational(0x7ff8000000000001).is_err());
}
#[test]
fn pi_interval_encloses_independently_pinned_decimal_bracket() {
    let p = pi(128).unwrap();
    assert!(p.lo <= r("3.1415926535897932384626433832795028841971"));
    assert!(p.hi >= r("3.1415926535897932384626433832795028841972"));
    assert!(p.hi - p.lo < scale_pow2(&Rat::one(), -124));
}
#[test]
fn sin_cos_enclose_pinned_values_after_reduction_and_preserve_zero() {
    // Brackets, not floating-point tolerances. Each includes the true value.
    let cases = [
        (
            "1",
            "0.8414709848078965066525023216302989996225",
            "0.8414709848078965066525023216302989996226",
            "0.5403023058681397174009366074429766037323",
            "0.5403023058681397174009366074429766037324",
        ),
        (
            "10",
            "-0.5440211108893698134047476618513772816837",
            "-0.5440211108893698134047476618513772816836",
            "-0.83907152907645245225886394782406483451994",
            "-0.83907152907645245225886394782406483451993",
        ),
    ];
    for (angle, slo, shi, clo, chi) in cases {
        let (s, c) = sin_cos(&r(angle), 128).unwrap();
        assert!(s.lo <= r(slo) && s.hi >= r(shi), "{angle}: {s:?}");
        assert!(c.lo <= r(clo) && c.hi >= r(chi), "{angle}: {c:?}");
        assert!(&s.hi - &s.lo < scale_pow2(&Rat::one(), -113));
        assert!(&c.hi - &c.lo < scale_pow2(&Rat::one(), -113));
        let (ns, nc) = sin_cos(&-r(angle), 128).unwrap();
        assert_eq!(ns, s.neg());
        assert_eq!(nc, c);
    }
    let (s, c) = sin_cos(&Rat::zero(), 128).unwrap();
    assert_eq!(s.lo, Rat::zero());
    assert_eq!(s.hi, Rat::zero());
    assert_eq!(c.lo, Rat::one());
    assert_eq!(c.hi, Rat::one());
}
#[test]
fn insufficient_precision_for_huge_argument_is_a_host_limit() {
    let error = sin_cos(&r("1e100"), 64).unwrap_err();
    assert!(error.quantity.contains("argument reduction"));
}
