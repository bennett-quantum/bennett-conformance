//! ISA-12.2: rejection records and legal host limits are distinct.
use bennett_cert_checker::{
    fault::{HostLimit, Reject, ResolveError},
    json::parse,
};

#[test]
fn rejection_paths_json_and_display_preserve_every_field() {
    let error = Reject::new("E_CERT_DOMAIN", "§12.2", "domain.width", "width mismatch");
    let expected = parse(r#"{"code":"E_CERT_DOMAIN","section":"§12.2","path":"regions.root.domain.width","message":"width mismatch"}"#).unwrap();
    assert_eq!(error.at("regions.root").to_value(), expected);
    assert_eq!(error.at("").path, "domain.width");
    assert_eq!(Reject::new("X", "§12.2", "", "m").at("claim").path, "claim");
    assert_eq!(
        error.to_string(),
        "E_CERT_DOMAIN (§12.2) at domain.width: width mismatch"
    );
    assert_eq!(error.path, "domain.width");
    assert_eq!(
        ResolveError::from(error.clone()),
        ResolveError::Reject(error)
    );
}

#[test]
fn host_limits_remain_non_rejections_with_the_original_quantity() {
    let limit = HostLimit {
        quantity: "domain element count 18446744073709551616".into(),
    };
    assert_eq!(
        ResolveError::from(limit.clone()),
        ResolveError::HostLimit(limit)
    );
}
