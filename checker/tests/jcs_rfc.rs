//! RFC 8785 Appendix B, all 24 finite samples and both forbidden samples.
//! Source: https://www.rfc-editor.org/rfc/rfc8785.html#appendix-B
use bennett_cert_checker::json::{ecmascript_number, jcs, parse, Value};
#[test]
fn appendix_b_every_binary64_serialization_sample() {
    let cases: [(u64, &str); 24] = [
        (0x0000000000000000, "0"),
        (0x8000000000000000, "0"),
        (0x0000000000000001, "5e-324"),
        (0x8000000000000001, "-5e-324"),
        (0x7fefffffffffffff, "1.7976931348623157e+308"),
        (0xffefffffffffffff, "-1.7976931348623157e+308"),
        (0x4340000000000000, "9007199254740992"),
        (0xc340000000000000, "-9007199254740992"),
        (0x4430000000000000, "295147905179352830000"),
        (0x44b52d02c7e14af5, "9.999999999999997e+22"),
        (0x44b52d02c7e14af6, "1e+23"),
        (0x44b52d02c7e14af7, "1.0000000000000001e+23"),
        (0x444b1ae4d6e2ef4e, "999999999999999700000"),
        (0x444b1ae4d6e2ef4f, "999999999999999900000"),
        (0x444b1ae4d6e2ef50, "1e+21"),
        (0x3eb0c6f7a0b5ed8c, "9.999999999999997e-7"),
        (0x3eb0c6f7a0b5ed8d, "0.000001"),
        (0x41b3de4355555553, "333333333.3333332"),
        (0x41b3de4355555554, "333333333.33333325"),
        (0x41b3de4355555555, "333333333.3333333"),
        (0x41b3de4355555556, "333333333.3333334"),
        (0x41b3de4355555557, "333333333.33333343"),
        (0xbecbf647612f3696, "-0.0000033333333333333333"),
        (0x43143ff3c1cb0959, "1424953923781206.2"),
    ];
    for (bits, want) in cases {
        let v = f64::from_bits(bits);
        assert_eq!(ecmascript_number(v), want, "{bits:016x}");
        assert_eq!(
            jcs(&parse(want).unwrap()).unwrap(),
            want.as_bytes(),
            "round trip {bits:016x}"
        );
    }
    for text in ["1e9999", "-1e9999"] {
        assert!(jcs(&parse(text).unwrap()).is_err());
    }
}
#[test]
fn utf16_sorting_including_supplementary_plane_and_surrogates() {
    let input =
        parse(r#"{"\ufb33":7,"\ud83d\ude00":6,"\u20ac":5,"\u00f6":4,"\u0080":3,"1":2,"\r":1}"#)
            .unwrap();
    let encoded = String::from_utf8(jcs(&input).unwrap()).unwrap();
    assert_eq!(
        encoded,
        "{\"\\r\":1,\"1\":2,\"\u{80}\":3,\"ö\":4,\"€\":5,\"😀\":6,\"דּ\":7}"
    );
    for bad in [r#""\udead""#, r#""\ud800""#, r#""\ud800x""#] {
        assert!(parse(bad).is_err());
    }
    assert_eq!(
        jcs(&Value::Str("\u{2028}\u{2029}".into())).unwrap(),
        "\"\u{2028}\u{2029}\"".as_bytes()
    );
}
