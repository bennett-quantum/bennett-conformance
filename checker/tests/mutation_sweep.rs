//! Every field and array position of the published positive certificate is
//! mutated, without sampling. Each mutation must produce its expected code.
mod common;
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
