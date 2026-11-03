mod common;
use bennett_cert_checker::{
    cert::{check_certificate, Inputs},
    json::{jcs, n, s, Value},
};
use common::*;

fn keyed() -> Inputs {
    let mut inputs = controlled();
    let mut cert = json(std::str::from_utf8(&inputs.cert_bytes).unwrap());
    let root = cert
        .get("bindings")
        .unwrap()
        .get("closure_hash")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    let Value::Obj(top) = &mut cert else { panic!() };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        panic!()
    };
    for (hash, region) in regions {
        let Value::Obj(fields) = region else { panic!() };
        let frame = fields.remove("frame").unwrap();
        let key = if hash == &root {
            json("[]")
        } else {
            json(&format!(
                r#"[{{"symbol":"{root}","point":"entry:0","iteration":[],"instance":[],"call_index":0}}]"#
            ))
        };
        let mut entry = json("{}");
        set(&mut entry, "context_key", key);
        set(&mut entry, "frame", frame);
        fields.insert("frame_contexts".into(), Value::Arr(vec![entry]));
    }
    inputs.cert_bytes = jcs(&cert).unwrap();
    inputs
}
fn change_child(inputs: &mut Inputs, f: impl FnOnce(&mut Value)) {
    let mut cert = json(std::str::from_utf8(&inputs.cert_bytes).unwrap());
    let root = cert
        .get("bindings")
        .unwrap()
        .get("closure_hash")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    let Value::Obj(top) = &mut cert else { panic!() };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        panic!()
    };
    f(regions
        .iter_mut()
        .find(|(h, _)| h.as_str() != root)
        .unwrap()
        .1);
    inputs.cert_bytes = jcs(&cert).unwrap();
}
#[test]
fn singleton_frame_contexts_select_root_and_predicated_off_child_with_full_keys() {
    let verdict = check_certificate(&keyed());
    assert_eq!(verdict.overall, "verified", "{verdict:?}");
}
#[test]
fn every_frame_context_binding_dimension_and_alternative_is_checked() {
    for field in ["symbol", "point", "iteration", "instance", "call_index"] {
        let mut inputs = keyed();
        change_child(&mut inputs, |r| {
            let Value::Obj(m) = r else { panic!() };
            let Value::Arr(a) = m.get_mut("frame_contexts").unwrap() else {
                panic!()
            };
            let Value::Obj(e) = &mut a[0] else { panic!() };
            let Value::Arr(key) = e.get_mut("context_key").unwrap() else {
                panic!()
            };
            set(
                &mut key[0],
                field,
                match field {
                    "symbol" => s(&"0".repeat(64)),
                    "point" => s("entry:1"),
                    "iteration" | "instance" => json("[1]"),
                    _ => n(1),
                },
            );
        });
        let verdict = check_certificate(&inputs);
        assert!(
            codes(&verdict).contains(&"E_CERT_FRAME"),
            "{field}: {verdict:?}"
        );
    }
    for case in 0..5 {
        let mut inputs = keyed();
        change_child(&mut inputs, |r| {
            let array = r.get("frame_contexts").unwrap().as_arr().unwrap().to_vec();
            match case {
                0 => set(r, "frame_contexts", json("[]")),
                1 => set(
                    r,
                    "frame_contexts",
                    Value::Arr(vec![array[0].clone(), array[0].clone()]),
                ),
                2 => set(r, "frame", array[0].get("frame").unwrap().clone()),
                3 => {
                    let mut e = array[0].clone();
                    set(&mut e, "context_key", json("[]"));
                    set(r, "frame_contexts", Value::Arr(vec![e]));
                }
                _ => {
                    let mut e = array[0].clone();
                    set(&mut e, "unknown", n(0));
                    set(r, "frame_contexts", Value::Arr(vec![e]));
                }
            }
        });
        let verdict = check_certificate(&inputs);
        assert!(
            codes(&verdict).contains(&"E_CERT_FRAME"),
            "case {case}: {verdict:?}"
        );
    }
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
#[test]
fn frame_contexts_exhaust_every_field_type_and_required_removal() {
    let base = keyed();
    let mut paths = vec![];
    let cert = json(std::str::from_utf8(&base.cert_bytes).unwrap());
    let regions = cert.get("regions").unwrap().as_obj().unwrap();
    for (hash, region) in regions {
        let mut path = vec![
            Part::Key("regions".into()),
            Part::Key(hash.clone()),
            Part::Key("frame_contexts".into()),
        ];
        paths.push(path.clone());
        positions(region.get("frame_contexts").unwrap(), &mut path, &mut paths);
    }
    let mut removals = 0;
    for path in &paths {
        let mut changed = cert.clone();
        *at(&mut changed, path) = Value::Null;
        let mut inputs = base.clone();
        inputs.cert_bytes = jcs(&changed).unwrap();
        let verdict = check_certificate(&inputs);
        assert!(
            codes(&verdict).contains(&"E_CERT_FRAME"),
            "{path:?}: {verdict:?}"
        );
        if let Some(Part::Key(key)) = path.last() {
            if key == "frame_contexts" {
                continue;
            }
            let mut changed = cert.clone();
            let Value::Obj(m) = at(&mut changed, &path[..path.len() - 1]) else {
                panic!()
            };
            m.remove(key);
            inputs.cert_bytes = jcs(&changed).unwrap();
            let verdict = check_certificate(&inputs);
            assert!(
                codes(&verdict).contains(&"E_CERT_FRAME"),
                "remove {path:?}: {verdict:?}"
            );
            removals += 1;
        }
    }
    assert_eq!((paths.len(), removals), (39, 31));
}
