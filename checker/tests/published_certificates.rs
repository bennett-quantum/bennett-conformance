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

#[derive(Clone, Debug)]
enum Part {
    Key(String),
    Index(usize),
}
fn positions(value: &Value, path: &mut Vec<Part>, out: &mut Vec<Vec<Part>>) {
    match value {
        Value::Obj(m) => {
            for (k, v) in m {
                path.push(Part::Key(k.clone()));
                out.push(path.clone());
                positions(v, path, out);
                path.pop();
            }
        }
        Value::Arr(a) => {
            for (i, v) in a.iter().enumerate() {
                path.push(Part::Index(i));
                out.push(path.clone());
                positions(v, path, out);
                path.pop();
            }
        }
        _ => (),
    }
}
fn at<'a>(v: &'a mut Value, path: &[Part]) -> &'a mut Value {
    if path.is_empty() {
        return v;
    }
    match (&path[0], v) {
        (Part::Key(k), Value::Obj(m)) => at(m.get_mut(k).unwrap(), &path[1..]),
        (Part::Index(i), Value::Arr(a)) => at(&mut a[*i], &path[1..]),
        _ => panic!(),
    }
}
fn expected(path: &[Part]) -> &'static str {
    let keys: Vec<_> = path
        .iter()
        .filter_map(|p| {
            if let Part::Key(k) = p {
                Some(k.as_str())
            } else {
                None
            }
        })
        .collect();
    if keys.contains(&"domain") {
        return "E_CERT_DOMAIN";
    }
    if keys.first() == Some(&"regions")
        && keys.len() >= 3
        && matches!(keys[2], "kind" | "signature" | "bounds")
    {
        return "E_CERT_BINDING";
    }
    if keys.contains(&"frame") {
        return "E_CERT_FRAME";
    }
    if keys.contains(&"expected") {
        return if keys.last() == Some(&"expected") {
            "E_CERT_BINDING"
        } else {
            "E_CERT_TABLE"
        };
    }
    if keys.contains(&"invocation") && keys.last() != Some(&"invocation") {
        return "E_CERT_INVOCATION";
    }
    "E_CERT_SCHEMA"
}
#[test]
fn both_published_k91_certificates_exhaust_every_field_and_array_position() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../bennett-isa-spec/conformance/fixtures/accepted-k91-09");
    let mut total = 0;
    let mut failures = vec![];
    for name in ["pair-same", "pair-distinct"] {
        let object_text = std::fs::read_to_string(root.join(format!("{name}.bisa"))).unwrap();
        let object = Object::parse(&object_text).unwrap();
        let cert = json(object.certificate_raw.as_deref().unwrap());
        let mut paths = vec![];
        positions(&cert, &mut vec![], &mut paths);
        assert_eq!(paths.len(), 1121, "published K91 field inventory changed");
        for path in paths {
            let mut changed = cert.clone();
            let value = at(&mut changed, &path);
            *value = if *value == Value::Null {
                Value::Bool(true)
            } else {
                Value::Null
            };
            let inputs = Inputs {
                cert_bytes: jcs(&changed).unwrap(),
                object_text: object_text.clone(),
                target_bytes: None,
                schedule_bytes: None,
                libraries: vec![],
                oracle_bin: None,
                limits: Limits::default(),
            };
            let verdict = check_certificate(&inputs);
            let wanted = expected(&path);
            if verdict.overall != "rejected" || !codes(&verdict).contains(&wanted) {
                failures.push(format!(
                    "{name} {path:?}: expected {wanted}, got {} {:?}",
                    verdict.overall,
                    codes(&verdict)
                ));
            }
            total += 1;
        }
    }
    eprintln!("K91 certificate sweep: {total} fields/positions, {} rejected with expected code, {} failures",total-failures.len(),failures.len());
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(total, 2242);
}
