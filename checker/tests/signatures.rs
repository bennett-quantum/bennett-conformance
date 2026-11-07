use crate::common;
use bennett_cert_checker::{
    cert::check_certificate,
    json::{jcs, Value},
    object::Object,
};
use common::*;
fn sources() -> Vec<(String, String)> {
    let declarations=[
        ("a","perm_action @a (t:wire[2] mut) (c:wire in) (r:region[1,2] in) (n:i2, tk:token) ()\n^entry():\nx t[0]\nx t[0]",r#"{"a":{"abi":"0.9","args":["i2","token"],"bounds":[],"in":[1],"kind":"perm_action","mem":[{"A":1,"W":2,"role":"in"}],"mut":[2],"tier":"q"}}"#),
        ("o","oracle @o (x:wire in) (t:wire[2]) (f:wire[4]) (r:region[1,2] in) (p:ptr, angle:f64, tk:token, n:i128)\nregs { result:reg[2] }\n^entry():\nresult = lift.const.i2 0\nlift.retval result",r#"{"o":{"abi":"0.9","args":["ptr","f64","token","i128"],"fault":4,"in":[1],"kind":"oracle","mem":[{"A":1,"W":2,"role":"in"}],"tgt":2,"tier":"q"}}"#),
    ];
    declarations
        .into_iter()
        .map(|(name, source, sig)| {
            let (inputs, _) = standalone(source, sig, name, vec![]);
            (name.into(), inputs.object_text)
        })
        .collect()
}
fn signature_text(source: &str, change: impl FnOnce(&mut Value)) -> String {
    let (prefix, rest) = source.split_once("section symbols\n").unwrap();
    let (text, suffix) = rest.split_once("\n\nsection functions\n").unwrap();
    let mut symbols = json(text);
    let Value::Obj(entries) = &mut symbols else {
        panic!()
    };
    let Value::Obj(symbol) = entries.values_mut().next().unwrap() else {
        panic!()
    };
    change(symbol.get_mut("signature").unwrap());
    format!(
        "{prefix}section symbols\n{}\n\nsection functions\n{suffix}",
        encode(&symbols)
    )
}
fn rejects(source: &str, name: &str) {
    match Object::parse(source) {
        Err(_) => (),
        Ok(object) => assert!(
            object.interface(name).is_err(),
            "ill-formed source interface was accepted: {source}"
        ),
    }
    // Rebind every document identity so a stale hash cannot mask the source
    // interface diagnostic in the public certificate entry point.
    let mut inputs = controlled();
    inputs.object_text = if Object::parse(source).is_ok() {
        rebind_object_symbols(source)
    } else {
        source.into()
    };
    if let Ok(object) = Object::parse(&inputs.object_text) {
        let symbol = object.symbol(name).unwrap();
        let hash = symbol.closure_hash.as_ref().unwrap();
        let mut cert = json(std::str::from_utf8(&inputs.cert_bytes).unwrap());
        set(
            &mut cert,
            "bindings",
            json(&format!(
                r#"{{"closure_hash":"{hash}","interface_hash":"{}","object_hash":"{}","reference_program_hash":"{}"}}"#,
                symbol.interface_hash, object.object_hash, symbol.interface_hash
            )),
        );
        inputs.cert_bytes = jcs(&cert).unwrap();
    }
    inputs.target_bytes = None;
    inputs.schedule_bytes = None;
    let verdict = check_certificate(&inputs);
    assert!(codes(&verdict).contains(&"E_CERT_OBJECT"), "{verdict:?}");
}
#[derive(Clone)]
enum Part {
    Key(String),
    Index(usize),
}
fn positions(v: &Value, p: &mut Vec<Part>, out: &mut Vec<Vec<Part>>) {
    match v {
        Value::Obj(m) => {
            for (k, v) in m {
                p.push(Part::Key(k.clone()));
                out.push(p.clone());
                positions(v, p, out);
                p.pop();
            }
        }
        Value::Arr(a) => {
            for (i, v) in a.iter().enumerate() {
                p.push(Part::Index(i));
                out.push(p.clone());
                positions(v, p, out);
                p.pop();
            }
        }
        _ => (),
    }
}
fn at<'a>(v: &'a mut Value, p: &[Part]) -> &'a mut Value {
    if p.is_empty() {
        return v;
    }
    match (&p[0], v) {
        (Part::Key(k), Value::Obj(m)) => at(m.get_mut(k).unwrap(), &p[1..]),
        (Part::Index(i), Value::Arr(a)) => at(&mut a[*i], &p[1..]),
        _ => panic!(),
    }
}
#[test]
fn quantum_signatures_exhaust_every_field_type_and_required_field_removal() {
    let mut types = 0;
    let mut removals = 0;
    for (name, source) in sources() {
        let object = Object::parse(&source).unwrap();
        let signature = &object.symbol(&name).unwrap().signature;
        let mut paths = vec![];
        positions(signature, &mut vec![], &mut paths);
        for path in paths {
            let mutated = signature_text(&source, |sig| *at(sig, &path) = Value::Null);
            rejects(&mutated, &name);
            types += 1;
            if let Some(Part::Key(key)) = path.last() {
                let mutated = signature_text(&source, |sig| {
                    let Value::Obj(m) = at(sig, &path[..path.len() - 1]) else {
                        panic!()
                    };
                    m.remove(key);
                });
                rejects(&mutated, &name);
                removals += 1;
            }
        }
    }
    assert_eq!((types, removals), (33, 22));
}
#[test]
fn source_roles_argument_types_names_and_unused_groups_match_signatures() {
    let sources = sources();
    let action = &sources[0].1;
    let oracle = &sources[1].1;
    for (source, from, to, name) in [
        (action, "t:wire[2] mut", "t:wire[2] in", "a"),
        (action, "c:wire in", "c:wire mut", "a"),
        (action, "c:wire in", "t:wire in", "a"),
        (action, "n:i2", "n:i3", "a"),
        (action, "n:i2", "n:i2 classical", "a"),
        (action, "r:region[1,2] in", "r:region[1,2] mut", "a"),
        (action, "r:region[1,2] in", "r:region[1,2]", "a"),
        (
            action,
            "(n:i2, tk:token) ()",
            "(n:i2, tk:token) (extra:wire)",
            "a",
        ),
        (oracle, "t:wire[2]", "t:wire[2] mut", "o"),
        (oracle, "p:ptr", "p:i32", "o"),
    ] {
        rejects(&source.replacen(from, to, 1), name);
    }
}
#[test]
fn unknown_signature_fields_and_invalid_arrays_reject_before_a_width_host_limit() {
    let source = &sources()[0].1;
    let oversized = source.replace("t:wire[2]", "t:wire[18446744073709551616]");
    let oversized = signature_text(&oversized, |sig| {
        set(sig, "mut", json(r#"["18446744073709551616"]"#));
        set(sig, "extra", Value::Bool(true));
    });
    rejects(&oversized, "a");
    for field in ["mut", "in", "mem", "args", "bounds"] {
        let source = signature_text(source, |sig| set(sig, field, Value::Bool(false)));
        rejects(&source, "a");
    }
}

#[test]
fn optional_wire_roles_and_zero_region_widths_require_canonical_spelling() {
    let source = &sources()[0].1;
    let source = source
        .replacen("t:wire[2] mut", "t:wire[2]", 1)
        .replacen("c:wire in", "c:wire", 1)
        .replacen("region[1,2]", "region[0,2]", 1);
    let source = signature_text(&source, |sig| {
        let Value::Obj(fields) = sig else { panic!() };
        let Value::Arr(mem) = fields.get_mut("mem").unwrap() else {
            panic!()
        };
        set(&mut mem[0], "A", json("0"));
    });
    assert!(
        Object::parse(&source).unwrap().interface("a").is_ok(),
        "{source}"
    );
    for invalid in ["region[-0,2]", "region[00,2]"] {
        let error = Object::parse(&source.replacen("region[0,2]", invalid, 1)).unwrap_err();
        assert!(error.starts_with("§16.1:"), "{error}");
    }
}
