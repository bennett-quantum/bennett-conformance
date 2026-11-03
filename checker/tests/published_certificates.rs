mod common;
use bennett_cert_checker::{
    cert::{check_certificate, Inputs, Limits},
    json::{jcs, Value},
    object::Object,
    sha256_hex,
};
use common::*;
#[test]
fn published_k84_certificates_replay_both_not_inputs_and_pin_each_negative_obligation() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../bennett-isa-spec/conformance/fixtures/accepted-k84-09");
    let manifest = json(&std::fs::read_to_string(root.join("k84.json")).unwrap());
    let mut count = 0;
    for case in manifest.get("cases").unwrap().as_arr().unwrap() {
        let bytes =
            std::fs::read(root.join(case.get("object").unwrap().as_str().unwrap())).unwrap();
        assert_eq!(
            sha256_hex(&bytes),
            case.get("sha256").unwrap().as_str().unwrap()
        );
        let object_text = String::from_utf8(bytes).unwrap();
        let object = Object::parse(&object_text).unwrap();
        assert!(object.stated_hashes_ok());
        let mut cert = json(object.certificate_raw.as_deref().unwrap());
        // These published admission fixtures predate K87. Port only the
        // resource spelling; source and every source/object hash remain exact.
        let Value::Obj(top) = &mut cert else { panic!() };
        let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
            panic!()
        };
        for r in regions.values_mut() {
            set(
                r,
                "resources",
                json(r#"{"peak_qubits[fast]":"1","qubit_second[fast]":"1","time":"1"}"#),
            );
        }
        let inputs = Inputs {
            cert_bytes: jcs(&cert).unwrap(),
            object_text,
            target_bytes: None,
            schedule_bytes: None,
            libraries: vec![],
            oracle_bin: None,
            limits: Limits::default(),
        };
        let verdict = check_certificate(&inputs);
        match case.get("name").unwrap().as_str().unwrap() {
            "baseline" => {
                assert!(codes(&verdict).is_empty(), "{verdict:?}");
                let result = &verdict.regions[0].claims[0].1;
                assert_eq!(result.outcome, "proof", "{verdict:?}");
                assert_eq!(result.elements.as_deref(), Some("2"));
            }
            "missing-expected" => {
                let result = &verdict.regions[0].claims[0].1;
                assert!(
                    result
                        .errors
                        .iter()
                        .any(|e| e.code == "E_CERT_SCHEMA" && e.path.ends_with(".expected")),
                    "{verdict:?}"
                );
            }
            "duplicate-component" => {
                assert!(codes(&verdict).contains(&"E_CERT_DOMAIN"), "{verdict:?}")
            }
            name => panic!("new K84 case requires an explicit assertion: {name}"),
        }
        count += 1;
    }
    assert_eq!(count, 3);
}
