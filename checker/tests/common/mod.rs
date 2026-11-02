#![allow(dead_code)]
use bennett_cert_checker::cert::{Inputs, Limits, Verdict};
use bennett_cert_checker::json::{jcs, parse, Value};
use bennett_cert_checker::object::Object;
use std::path::PathBuf;

pub fn json(s: &str) -> Value {
    parse(s).unwrap()
}
pub fn set(v: &mut Value, key: &str, x: Value) {
    let Value::Obj(m) = v else {
        panic!("expected object")
    };
    m.insert(key.to_string(), x);
}
pub fn controlled() -> Inputs {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../bennett-isa-spec/conformance/fixtures/controlled-action-09");
    Inputs {
        cert_bytes: std::fs::read(d.join("certificate.json")).unwrap(),
        object_text: std::fs::read_to_string(d.join("controlled.bisa")).unwrap(),
        target_bytes: Some(std::fs::read(d.join("target.json")).unwrap()),
        schedule_bytes: Some(std::fs::read(d.join("schedule.json")).unwrap()),
        libraries: vec![],
        oracle_bin: None,
        limits: Limits::default(),
    }
}
pub fn codes(v: &Verdict) -> Vec<&str> {
    v.errors
        .iter()
        .chain(v.regions.iter().flat_map(|r| {
            r.errors
                .iter()
                .chain(r.claims.iter().flat_map(|(_, c)| c.errors.iter()))
        }))
        .map(|e| e.code)
        .collect()
}
pub fn encode(v: &Value) -> String {
    String::from_utf8(jcs(v).unwrap()).unwrap()
}

/// Build a standalone reference certificate for independent test programs.
/// Hashes come from the checker's separately tested §9.2 implementation;
/// expected truth tables in the tests are specified independently.
pub fn standalone(
    functions: &str,
    signatures: &str,
    root: &str,
    claims: Vec<Value>,
) -> (Inputs, String) {
    let signatures = json(signatures);
    let mut symbols = json("{}");
    for (i, (name, sig)) in signatures.as_obj().unwrap().iter().enumerate() {
        let mut entry = json("{}");
        set(&mut entry, "name", Value::Str(name.clone()));
        set(&mut entry, "kind", sig.get("kind").unwrap().clone());
        set(&mut entry, "signature", sig.clone());
        set(&mut entry, "interface_hash", Value::Str("0".repeat(64)));
        set(&mut symbols, &i.to_string(), entry);
    }
    let make_object = |symbols: &Value| {
        format!(
        "bennett-object isa=0.9 object=0.9 launch=0.9 sched=0.9 cert=0.9 coverage=0.9 target=0.9 ptr_width=32\n\nsection symbols\n{}\n\nsection functions\n{}\n", encode(symbols), functions.trim_end())
    };
    let initial = Object::parse(&make_object(&symbols)).unwrap();
    for (i, s) in initial.symbols.iter().enumerate() {
        let Value::Obj(m) = &mut symbols else {
            unreachable!()
        };
        set(
            m.get_mut(&i.to_string()).unwrap(),
            "interface_hash",
            Value::Str(s.interface_hash.clone()),
        );
    }
    let object_text = make_object(&symbols);
    let object = Object::parse(&object_text).unwrap();
    assert!(object.stated_hashes_ok());
    let sym = object.symbol(root).unwrap();
    let h = sym.closure_hash.clone().unwrap();
    let iface = object.interface(root).unwrap();
    let mut region = json(
        r#"{"bounds":[],"claims":[],"cleanup":{},"definedness_assumptions":[],"error":{},"frame":{"ancillae":[],"borrow":[],"history":[],"scratch":[],"slots":[]},"gates_used":{},"resources":{},"verification":{"borrow_frames":[],"channel":[],"fault_rate":{"domain":{"borrowed":[],"bundles":[],"regions":[]},"faults_by_code":{},"layout_dependent_excluded":0,"undefined_excluded":0},"inverse_pairs":[],"oracle":null,"unitary":[]}}"#,
    );
    set(&mut region, "kind", Value::Str(sym.kind.clone()));
    set(&mut region, "signature", sym.signature.clone());
    let slots = iface
        .bundles
        .iter()
        .map(|b| {
            json(&format!(
                r#"{{"class":"fast","id":"{}:{}","role":"{}","width":{}}}"#,
                b.group, b.slot, b.group, b.width
            ))
        })
        .collect();
    let mut frame = region.get("frame").unwrap().clone();
    set(&mut frame, "slots", Value::Arr(slots));
    set(&mut region, "frame", frame);
    let claims = claims
        .into_iter()
        .map(|mut c| {
            set(
                &mut c,
                "subject",
                json(&format!(
                    r#"{{"closure_hash":"{h}","inherited_controls":0,"variant":"forward"}}"#
                )),
            );
            c
        })
        .collect();
    set(&mut region, "claims", Value::Arr(claims));
    let mut regions = json("{}");
    set(&mut regions, &h, region);
    let mut cert = json(
        r#"{"bindings":{},"composition":{"epsilon_total":{"diamond":0},"method":"sum_over_multiplicities","multiplicities":{}},"producer":{"materialisation":{"llvm_version":"none","normaliser_version":"1","pipeline":"independent-test"},"name":"checker-tests","version":"1"},"regions":{},"versions":{"cert":"0.9","coverage":"0.9","isa":"0.9","launch":"0.9","object":"0.9","sched":"0.9","target":"0.9"}}"#,
    );
    set(&mut cert, "regions", regions);
    set(
        &mut cert,
        "bindings",
        json(&format!(
            r#"{{"closure_hash":"{h}","interface_hash":"{}","object_hash":"{}","reference_program_hash":"{}"}}"#,
            sym.interface_hash, object.object_hash, sym.interface_hash
        )),
    );
    (
        Inputs {
            cert_bytes: jcs(&cert).unwrap(),
            object_text,
            target_bytes: None,
            schedule_bytes: None,
            libraries: vec![],
            oracle_bin: None,
            limits: Limits::default(),
        },
        h,
    )
}

pub fn bind_schedule(inp: &mut Inputs, mut schedule: Value, target: Value) {
    let object = Object::parse(&inp.object_text).unwrap();
    let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let root = cert
        .get("bindings")
        .unwrap()
        .get("closure_hash")
        .unwrap()
        .as_str()
        .unwrap();
    let root = object.symbol_by_closure(root).unwrap();
    let th = bennett_cert_checker::sha256_hex(&jcs(&target).unwrap());
    let invocation = cert
        .get("bindings")
        .and_then(|b| b.get("invocation"))
        .cloned()
        .unwrap_or_else(|| json(r#"{"args":[],"budget":["0","0","0","0"]}"#));
    let mut sb = schedule.get("bindings").unwrap().clone();
    set(
        &mut sb,
        "closure_hash",
        Value::Str(root.closure_hash.clone().unwrap()),
    );
    set(&mut sb, "region", Value::Str(root.name.clone()));
    set(&mut sb, "target_hash", Value::Str(th.clone()));
    set(
        &mut sb,
        "params_hash",
        Value::Str(bennett_cert_checker::sha256_hex(
            &jcs(invocation.get("args").unwrap()).unwrap(),
        )),
    );
    set(&mut schedule, "bindings", sb);
    let sh = bennett_cert_checker::sha256_hex(&jcs(&schedule).unwrap());
    let mut bindings = cert.get("bindings").unwrap().clone();
    set(&mut bindings, "target_hash", Value::Str(th));
    set(&mut bindings, "schedule_hash", Value::Str(sh.clone()));
    set(&mut bindings, "invocation", invocation);
    set(&mut cert, "bindings", bindings);
    let Value::Obj(top) = &mut cert else {
        unreachable!()
    };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        unreachable!()
    };
    for region in regions.values_mut() {
        let Value::Obj(r) = region else {
            unreachable!()
        };
        let Value::Arr(claims) = r.get_mut("claims").unwrap() else {
            unreachable!()
        };
        for c in claims {
            if let Value::Obj(m) = c {
                if let Some(subject) = m.get_mut("subject") {
                    set(subject, "schedule_hash", Value::Str(sh.clone()));
                }
            }
        }
    }
    inp.cert_bytes = jcs(&cert).unwrap();
    inp.target_bytes = Some(jcs(&target).unwrap());
    inp.schedule_bytes = Some(jcs(&schedule).unwrap());
}
