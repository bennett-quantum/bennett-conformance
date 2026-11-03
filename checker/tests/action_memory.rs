mod common;
use bennett_cert_checker::{
    domain::Tuple,
    eval::{eval_action_tuple, EvalError},
    object::Object,
};
use common::*;

fn object(body: &str, a: u32, w: u32, t: u32) -> Object {
    object_with_image(body, a, w, t, None)
}
fn object_with_image(body: &str, a: u32, w: u32, t: u32, image: Option<&str>) -> Object {
    let data = image
        .map(|hex| format!("\n\nsection data\nimg {hex}\n\nsection devices\n{{\"models\":{{}},\"services\":{{}}}}"))
        .unwrap_or_default();
    let data_sig = image
        .map(|hex| {
            format!(
                r#", "img":{{"abi":"0.9","kind":"data","size":{},"tier":"-"}}"#,
                hex.len() / 2
            )
        })
        .unwrap_or_default();
    let source=format!("perm_action @a (t:wire[{t}] mut) (c:wire in) (r:region[{a},{w}] mut) () ()\n^entry():\n{body}{data}");
    let sig = format!(
        r#"{{"a":{{"abi":"0.9","args":[],"bounds":[],"in":[1],"kind":"perm_action","mem":[{{"A":{a},"W":{w},"role":"mut"}}],"mut":[{t}],"tier":"q"}}{data_sig}}}"#
    );
    let (inputs, _) = standalone(&source, &sig, "a", vec![]);
    let text=rebind_object_symbols(&format!("{}\nsection layout\n{{\"A_tab\":0,\"H\":4,\"S_cells\":0,\"globals\":{{}},\"heap_base\":1,\"heap_limit_max\":0,\"interface_regions\":{{}},\"null_size\":1,\"ptr_width\":32}}\n",inputs.object_text));
    Object::parse(&text).unwrap()
}
fn run(object: &Object, before: &Tuple, adjoint: bool) -> Result<Tuple, EvalError> {
    eval_action_tuple(
        object,
        object.symbol("a").unwrap(),
        &object.interface("a").unwrap(),
        before,
        &[],
        0,
        adjoint,
        0,
    )
}
fn tuple(t: u32, c: u32, cells: &[u32]) -> Tuple {
    Tuple {
        bundles: vec![t.into(), c.into()],
        borrowed: vec![],
        regions: vec![cells.iter().copied().map(Into::into).collect()],
        args: vec![],
    }
}

#[test]
fn memory_width_address_ownership_and_arity_are_checked_before_false_predicates() {
    for body in [
        "qpred c\nqstore r () t\nqunpred c",
        "qpred c\nqexchm r () t\nqunpred c",
        "qpred c\nqunload r () t\nqunpred c",
        "qpred c\nqunload r () ghost\nqunpred c",
        "qpred c\nd = qload r c\nqunload r c d\nqunpred c",
        "qpred c\nd = qload r () ignored\nqunload r () d\nqunpred c",
        "qpred c\nqstore r () c ignored\nqunpred c",
    ] {
        let object = object(body, 0, 1, 2);
        let result = run(&object, &tuple(0, 0, &[0]), false);
        assert!(
            matches!(&result,Err(EvalError::Reject(e)) if e.code=="E_CERT_OBJECT"),
            "{body}: {result:?}"
        );
    }
    for body in [
        "d = qload r ()\nqunload r () d",
        "qexchm r t t",
        "qstore r t t",
    ] {
        let object = object(body, 1, 1, 1);
        let result = run(&object, &tuple(0, 0, &[0, 0]), false);
        assert!(
            matches!(&result,Err(EvalError::Reject(e)) if e.code=="E_CERT_OBJECT"),
            "{body}: {result:?}"
        );
    }
}

#[test]
fn empty_and_indexed_addresses_and_values_replay_without_truncation() {
    let mut checked = 0;
    for a in 0..2 {
        let address = if a == 0 { "()" } else { "t[0]" };
        for op in ["qstore", "qexchm", "load"] {
            let body = if op == "load" {
                format!(
                    "qpred c\nd = qload r {address}\nqxor d t[1]\nqunload r {address} d\nqunpred c"
                )
            } else {
                format!("qpred c\n{op} r {address} t[1]\nqunpred c")
            };
            let object = object(&body, a, 1, 2);
            for t in 0..4u32 {
                for c in 0..2u32 {
                    for mem in 0..1 << (1 << a) {
                        let cells: Vec<u32> = (0..1 << a).map(|i| (mem >> i) & 1).collect();
                        let before = tuple(t, c, &cells);
                        let mut expected = before.clone();
                        if c != 0 {
                            let index = if a == 0 { 0 } else { (t & 1) as usize };
                            let cell = cells[index];
                            if op == "qstore" {
                                expected.regions[0][index] = ((t >> 1) ^ cell).into();
                            } else if op == "load" {
                                expected.bundles[0] = (t ^ (cell << 1)).into();
                            } else {
                                expected.bundles[0] = (t ^ (((t >> 1) ^ cell) << 1)).into();
                                expected.regions[0][index] = (t >> 1).into();
                            }
                        }
                        let after = run(&object, &before, false).unwrap();
                        assert_eq!(after, expected, "A={a} {op}, t={t}, c={c}, cells={cells:?}");
                        assert_eq!(run(&object, &after, true).unwrap(), before);
                        checked += 1;
                    }
                }
            }
        }
    }
    assert_eq!(checked, 144);
}

#[test]
fn local_regions_exist_at_entry_in_zero_trip_loops_and_are_created_once_per_invocation() {
    for body in [
        "d = qload local ()\nqunload local () d\nqloop 0 {\nqregion local 0 1\n}",
        "qloop 2 {\nqregion local 0 1\nqstore local () t[0]\n}",
        "qif c {\nqregion local 0 1\n} {\n}\nd = qload local ()\nqunload local () d",
    ] {
        let object = object(body, 0, 1, 1);
        for t in 0..2 {
            for c in 0..2 {
                let before = tuple(t, c, &[0]);
                assert_eq!(run(&object, &before, false).unwrap(), before, "{body}");
                assert_eq!(
                    run(&object, &before, true).unwrap(),
                    before,
                    "adjoint {body}"
                );
            }
        }
    }
}

#[test]
fn duplicate_local_regions_cannot_erase_dirty_storage_and_overlapping_names_reject() {
    for body in [
        "qregion local 0 1\nqstore local () t\nqregion local 0 1",
        "qregion r 0 1",
        "qregion t 0 1",
        "qregion local 0 0",
    ] {
        let object = object(body, 0, 1, 1);
        let result = run(&object, &tuple(1, 0, &[0]), false);
        assert!(
            matches!(&result,Err(EvalError::Reject(e)) if e.code=="E_CERT_OBJECT"),
            "{body}: {result:?}"
        );
    }
}

#[test]
fn local_images_bind_exact_bytes_and_little_endian_cells_across_adjoint_replay() {
    let mut checked = 0;
    for (a, w, image) in [(0, 8, "a5"), (1, 4, "a5")] {
        let address = if a == 0 { "()" } else { "t[0]" };
        let xor = if a == 0 {
            "qxor d t".to_string()
        } else {
            (0..4)
                .map(|i| format!("cx d[{i}] t[{}]", i + 4))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let body=format!("qpred c\nd = qload local {address}\n{xor}\nqunload local {address} d\nqunpred c\nqloop 0 {{\nqregion local {a} {w} @img\n}}");
        let object = object_with_image(&body, 0, 1, 8, Some(image));
        for t in 0..256 {
            for c in 0..2 {
                let before = tuple(t, c, &[0]);
                let mut expected = before.clone();
                if c != 0 {
                    expected.bundles[0] = (t ^ if a == 0 {
                        0xa5
                    } else {
                        if t & 1 == 0 {
                            5 << 4
                        } else {
                            10 << 4
                        }
                    })
                    .into();
                }
                let after = run(&object, &before, false).unwrap();
                assert_eq!(after, expected, "A={a} W={w} t={t} c={c}");
                assert_eq!(run(&object, &after, true).unwrap(), before);
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 1024);
    for image in ["", "00", "000000"] {
        let object = object_with_image("qregion local 1 8 @img", 0, 1, 1, Some(image));
        let result = run(&object, &tuple(0, 0, &[0]), false);
        assert!(
            matches!(&result,Err(EvalError::Reject(e))if e.code=="E_CERT_OBJECT"),
            "{image}: {result:?}"
        );
    }
}
