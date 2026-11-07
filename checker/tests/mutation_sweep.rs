//! Every field and array position of the published positive certificate is
//! mutated, without sampling. Each mutation must produce its expected code.
use crate::common;
use bennett_cert_checker::cert::check_certificate;
use bennett_cert_checker::json::{jcs, Value};
use common::*;

#[derive(Clone, Debug)]
enum Part {
    Key(String),
    Index(usize),
}
fn walk(v: &Value, path: &mut Vec<Part>, out: &mut Vec<Vec<Part>>) {
    match v {
        Value::Obj(m) => {
            for (k, v) in m {
                path.push(Part::Key(k.clone()));
                out.push(path.clone());
                walk(v, path, out);
                path.pop();
            }
        }
        Value::Arr(a) => {
            for (i, v) in a.iter().enumerate() {
                path.push(Part::Index(i));
                out.push(path.clone());
                walk(v, path, out);
                path.pop();
            }
        }
        _ => {}
    }
}
fn at<'a>(v: &'a mut Value, path: &[Part]) -> &'a mut Value {
    if path.is_empty() {
        return v;
    }
    match (&path[0], v) {
        (Part::Key(k), Value::Obj(m)) => at(m.get_mut(k).unwrap(), &path[1..]),
        (Part::Index(i), Value::Arr(a)) => at(&mut a[*i], &path[1..]),
        _ => unreachable!(),
    }
}
fn expected(path: &[Part]) -> &'static str {
    let keys: Vec<&str> = path
        .iter()
        .filter_map(|p| match p {
            Part::Key(k) => Some(k.as_str()),
            _ => None,
        })
        .collect();
    if keys.contains(&"domain") {
        return "E_CERT_DOMAIN";
    }
    // Signature and bounds bind the object directly; malformed supplied
    // signatures must fail the binding, whatever other checks also fail.
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
        if keys.last() == Some(&"expected") {
            return "E_CERT_BINDING";
        }
        return "E_CERT_TABLE";
    }
    if keys.contains(&"invocation") && keys.last() != Some(&"invocation") {
        return "E_CERT_INVOCATION";
    }
    "E_CERT_SCHEMA"
}
#[test]
fn published_certificate_every_field_type_mutation() {
    let base = controlled();
    let cert = json(std::str::from_utf8(&base.cert_bytes).unwrap());
    let mut paths = vec![];
    walk(&cert, &mut vec![], &mut paths);
    let mut failures = vec![];
    for path in &paths {
        let mut changed = cert.clone();
        let v = at(&mut changed, path);
        *v = if *v == Value::Null {
            Value::Bool(true)
        } else {
            Value::Null
        };
        let mut inp = base.clone();
        inp.cert_bytes = jcs(&changed).unwrap();
        let verdict = check_certificate(&inp);
        let actual = codes(&verdict);
        let want = expected(path);
        if verdict.overall != "rejected" || !actual.contains(&want) {
            failures.push(format!(
                "{path:?}: expected {want}, got {} {actual:?}",
                verdict.overall
            ));
        }
    }
    eprintln!("certificate mutation sweep: {} fields/positions, {} rejected with expected code, {} failures",paths.len(),paths.len()-failures.len(),failures.len());
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(
        paths.len(),
        250,
        "published certificate field inventory changed"
    );
}

#[test]
fn published_target_every_field_type_mutation_with_hashes_rebound() {
    let base = controlled();
    let target = json(std::str::from_utf8(base.target_bytes.as_ref().unwrap()).unwrap());
    let schedule = json(std::str::from_utf8(base.schedule_bytes.as_ref().unwrap()).unwrap());
    let mut paths = vec![];
    walk(&target, &mut vec![], &mut paths);
    let mut failures = vec![];
    for path in &paths {
        let mut changed = target.clone();
        let v = at(&mut changed, path);
        *v = if *v == Value::Null {
            Value::Bool(true)
        } else {
            Value::Null
        };
        let mut inp = base.clone();
        bind_schedule(&mut inp, schedule.clone(), changed);
        let verdict = check_certificate(&inp);
        if verdict.overall != "rejected" || !codes(&verdict).contains(&"E_CERT_TARGET") {
            failures.push(format!(
                "{path:?}: {} {:?}",
                verdict.overall,
                codes(&verdict)
            ));
        }
    }
    eprintln!(
        "target field sweep: {} fields/positions; {} failures",
        paths.len(),
        failures.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(paths.len(), 43, "published target field inventory changed");
}

// Rebind only the schedule hash: normalising any mutated schedule field here
// would hide precisely the malformed metadata this sweep is meant to find.
fn rebind_schedule(inp: &mut bennett_cert_checker::cert::Inputs, schedule: &Value) {
    let bytes = jcs(schedule).unwrap();
    let hash = bennett_cert_checker::sha256::sha256_hex(&bytes);
    inp.schedule_bytes = Some(bytes);
    let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else {
        unreachable!()
    };
    set(
        top.get_mut("bindings").unwrap(),
        "schedule_hash",
        Value::Str(hash.clone()),
    );
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        unreachable!()
    };
    for report in regions.values_mut() {
        let Value::Obj(report) = report else {
            unreachable!()
        };
        let Value::Arr(claims) = report.get_mut("claims").unwrap() else {
            unreachable!()
        };
        for claim in claims {
            let Value::Obj(claim) = claim else {
                unreachable!()
            };
            if let Some(subject) = claim.get_mut("subject") {
                if subject.get("schedule_hash").is_some() {
                    set(subject, "schedule_hash", Value::Str(hash.clone()));
                }
            }
        }
    }
    inp.cert_bytes = jcs(&cert).unwrap();
}

#[test]
fn published_schedule_every_field_type_mutation_with_hash_rebound() {
    let base = controlled();
    let schedule = json(std::str::from_utf8(base.schedule_bytes.as_ref().unwrap()).unwrap());
    let mut paths = vec![];
    walk(&schedule, &mut vec![], &mut paths);
    let mut failures = vec![];
    for path in &paths {
        let mut changed = schedule.clone();
        let v = at(&mut changed, path);
        *v = if *v == Value::Null {
            Value::Bool(true)
        } else {
            Value::Null
        };
        let mut inp = base.clone();
        rebind_schedule(&mut inp, &changed);
        let verdict = check_certificate(&inp);
        if verdict.overall != "rejected" || !codes(&verdict).contains(&"E_CERT_SCHEDULE_BINDING") {
            failures.push(format!(
                "{path:?}: {} {:?}",
                verdict.overall,
                codes(&verdict)
            ));
        }
    }
    eprintln!(
        "schedule field sweep: {} fields/positions; {} failures",
        paths.len(),
        failures.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(
        paths.len(),
        54,
        "published schedule field inventory changed"
    );
}

#[test]
fn schedule_binding_values_and_closed_records_are_checked() {
    let base = controlled();
    let schedule = json(std::str::from_utf8(base.schedule_bytes.as_ref().unwrap()).unwrap());
    let cases = [
        (
            vec![Part::Key("bindings".into()), Part::Key("epsilon".into())],
            json("-1"),
        ),
        (
            vec![Part::Key("bindings".into()), Part::Key("budget".into())],
            json("[0,0,0]"),
        ),
        (
            vec![
                Part::Key("bindings".into()),
                Part::Key("budget".into()),
                Part::Index(0),
            ],
            json("-1"),
        ),
        (
            vec![
                Part::Key("bindings".into()),
                Part::Key("budget".into()),
                Part::Index(0),
            ],
            json(r#""00""#),
        ),
        (
            vec![Part::Key("bindings".into()), Part::Key("scheduler".into())],
            json(r#"{"name":"x","version":"1","extra":0}"#),
        ),
        (
            vec![
                Part::Key("bindings".into()),
                Part::Key("scheduler".into()),
                Part::Key("name".into()),
            ],
            Value::Str(String::new()),
        ),
        (
            vec![Part::Key("bindings".into()), Part::Key("expansion".into())],
            json(
                r#"{"name":"ref","version":"0.9","hash":"5de4e4995350313e50217880ba1b95de39cd043283b726337d4894efb005d197","extra":0}"#,
            ),
        ),
        (
            vec![
                Part::Key("entries".into()),
                Part::Index(0),
                Part::Key("gate".into()),
                Part::Key("epsilon_share".into()),
            ],
            json("-1"),
        ),
    ];
    for (path, value) in cases {
        let mut changed = schedule.clone();
        *at(&mut changed, &path) = value;
        let mut inp = base.clone();
        rebind_schedule(&mut inp, &changed);
        let verdict = check_certificate(&inp);
        assert!(
            codes(&verdict).contains(&"E_CERT_SCHEDULE_BINDING"),
            "{path:?}: {verdict:?}"
        );
    }
    // §9.1 permits canonical decimal strings as exact metadata integers.
    let mut schedule = schedule;
    *at(
        &mut schedule,
        &[Part::Key("bindings".into()), Part::Key("budget".into())],
    ) = json(r#"["0","0","0","0"]"#);
    let mut inp = base;
    rebind_schedule(&mut inp, &schedule);
    let verdict = check_certificate(&inp);
    assert_eq!(verdict.overall, "verified", "{verdict:?}");
}

fn coverage_fixture() -> (bennett_cert_checker::cert::Inputs, Value) {
    let inp = controlled();
    let cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let interface = cert
        .get("bindings")
        .unwrap()
        .get("interface_hash")
        .unwrap()
        .as_str()
        .unwrap();
    let point = format!(r#"["{interface}","entry",0]"#);
    let coverage = json(&format!(
        r#"{{"entries":[{{"point":{point},"source":"DLT-example","classification":"lowered","reason":"literal gate","count":1}},{{"point":{point},"source":"DLT-example","classification":"trapped","reason":"unsupported source operation","count":1,"fault_code":4}},{{"point":{point},"source":"DLT-example","classification":"shimmed","reason":"shim binding","count":1,"shim_hash":"{}"}}]}}"#,
        "a".repeat(64)
    ));
    (inp, coverage)
}
fn attach_coverage(inp: &mut bennett_cert_checker::cert::Inputs, coverage: &Value) {
    inp.object_text
        .push_str(&format!("\nsection coverage\n{}\n", encode(coverage)));
}
#[test]
fn coverage_every_field_type_mutation_and_interface_point_binding() {
    let (base, coverage) = coverage_fixture();
    let mut paths = vec![];
    walk(&coverage, &mut vec![], &mut paths);
    let mut failures = vec![];
    for path in &paths {
        let mut changed = coverage.clone();
        *at(&mut changed, path) = Value::Null;
        let mut inp = base.clone();
        attach_coverage(&mut inp, &changed);
        let verdict = check_certificate(&inp);
        if verdict.overall != "rejected" || !codes(&verdict).contains(&"E_CERT_COVERAGE") {
            failures.push(format!(
                "{path:?}: {} {:?}",
                verdict.overall,
                codes(&verdict)
            ));
        }
    }
    eprintln!(
        "coverage field sweep: {} fields/positions; {} failures",
        paths.len(),
        failures.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert_eq!(paths.len(), 30, "coverage fixture inventory changed");
    let mut good = base.clone();
    attach_coverage(&mut good, &coverage);
    let v = check_certificate(&good);
    assert_eq!(v.overall, "unchecked", "{v:?}");
    assert!(codes(&v).is_empty(), "{v:?}");
    assert!(
        v.establishes
            .iter()
            .any(|s| s.contains("coverage") && s.contains("program-point")),
        "{v:?}"
    );
    for (path, value) in [
        (
            vec![
                Part::Key("entries".into()),
                Part::Index(0),
                Part::Key("point".into()),
                Part::Index(0),
            ],
            Value::Str("b".repeat(64)),
        ),
        (
            vec![
                Part::Key("entries".into()),
                Part::Index(0),
                Part::Key("point".into()),
                Part::Index(2),
            ],
            json("999"),
        ),
        (
            vec![
                Part::Key("entries".into()),
                Part::Index(1),
                Part::Key("classification".into()),
            ],
            Value::Str("lowered".into()),
        ),
        (
            vec![
                Part::Key("entries".into()),
                Part::Index(2),
                Part::Key("shim_hash".into()),
            ],
            Value::Str("BAD".into()),
        ),
    ] {
        let mut changed = coverage.clone();
        *at(&mut changed, &path) = value;
        let mut inp = base.clone();
        attach_coverage(&mut inp, &changed);
        let v = check_certificate(&inp);
        assert!(codes(&v).contains(&"E_CERT_COVERAGE"), "{path:?}: {v:?}");
    }
}
