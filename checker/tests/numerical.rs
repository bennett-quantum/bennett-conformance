mod common;
use bennett_cert_checker::cert::check_certificate;
use common::*;
#[test]
fn controlled_fixture_numerical_claims_check_every_default_context() {
    let v = check_certificate(&controlled());
    assert!(codes(&v).is_empty(), "{v:?}");
    let mut checked = 0;
    for region in &v.regions {
        for (kind, claim) in &region.claims {
            if kind == "numerical" {
                assert_eq!(claim.outcome, "accept", "{region:?}");
                assert_eq!(claim.computed.as_deref(), Some("0"));
                assert_eq!(claim.contexts, Some(1));
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 2);
}

use bennett_cert_checker::json::{jcs, n, Value};
fn add_channel(inp: &mut bennett_cert_checker::cert::Inputs, zero_pivot: bool, missing: bool) {
    let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else {
        unreachable!()
    };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        unreachable!()
    };
    let root = regions
        .values_mut()
        .find(|r| r.get("claims").unwrap().as_arr().unwrap().len() == 2)
        .unwrap();
    let mut channel = root.get("claims").unwrap().as_arr().unwrap()[1].clone();
    set(&mut channel, "kind", Value::Str("channel".into()));
    set(&mut channel, "metric", Value::Str("diamond".into()));
    set(&mut channel, "bound", json("0.625"));
    let entries = (0..16)
        .flat_map(|i| {
            (0..16).map(move |j| {
                json(if i == j {
                    r#"["0.125","0.125","0","0"]"#
                } else {
                    r#"["0","0","0","0"]"#
                })
            })
        })
        .collect();
    let mut dual = json(r#"{"mu":0.625,"Y":[]}"#);
    set(&mut dual, "Y", Value::Arr(entries));
    if zero_pivot {
        set(&mut dual, "mu", json("0.5"));
    }
    set(
        &mut channel,
        "dual",
        Value::Arr(if missing { vec![] } else { vec![dual] }),
    );
    let Value::Obj(root) = root else {
        unreachable!()
    };
    let Value::Arr(claims) = root.get_mut("claims").unwrap() else {
        unreachable!()
    };
    claims.push(channel);
    let ver = root.get_mut("verification").unwrap();
    set(ver, "channel", Value::Arr(vec![n(2)]));
    inp.cert_bytes = jcs(&cert).unwrap();
}
#[test]
fn channel_strict_dual_witness_is_checked_at_the_resolved_context() {
    let mut inp = controlled();
    add_channel(&mut inp, false, false);
    let v = check_certificate(&inp);
    assert!(codes(&v).is_empty(), "{v:?}");
    let root = v
        .regions
        .iter()
        .find(|r| r.name.as_deref() == Some("controlled"))
        .unwrap();
    assert_eq!(root.claims[2].1.outcome, "accept", "{root:?}");
    assert_eq!(root.claims[2].1.contexts, Some(1));
}
#[test]
fn channel_rejects_zero_pivot_and_missing_context_witness() {
    for (zero, missing) in [(true, false), (false, true)] {
        let mut inp = controlled();
        add_channel(&mut inp, zero, missing);
        let v = check_certificate(&inp);
        assert!(codes(&v).contains(&"E_CERT_CHANNEL"), "{v:?}");
    }
}

#[test]
fn scheduled_decisions_are_not_inferred_from_unrelated_exact_target_entries() {
    for variant in [
        "unsupported-controls",
        "unimplemented-scheme",
        "tied-decisions",
    ] {
        let mut inp = controlled();
        let mut target = json(std::str::from_utf8(inp.target_bytes.as_ref().unwrap()).unwrap());
        let mut schedule = json(std::str::from_utf8(inp.schedule_bytes.as_ref().unwrap()).unwrap());
        if variant == "unsupported-controls" {
            let Value::Obj(t) = &mut target else {
                unreachable!()
            };
            let Value::Arr(gates) = t.get_mut("gates").unwrap() else {
                unreachable!()
            };
            set(&mut gates[0], "controls", json(r#"{"max":2,"min":2}"#));
        } else {
            let Value::Obj(s) = &mut schedule else {
                unreachable!()
            };
            let Value::Arr(entries) = s.get_mut("entries").unwrap() else {
                unreachable!()
            };
            if variant == "unimplemented-scheme" {
                set(
                    &mut entries[0],
                    "gate",
                    json(r#"{"epsilon_share":0,"realisation":"fake-exact-scheme"}"#),
                );
            } else {
                entries.push(entries[0].clone());
            }
        }
        bind_schedule(&mut inp, schedule, target);
        let v = check_certificate(&inp);
        for r in &v.regions {
            for (_, c) in &r.claims {
                assert!(
                    !matches!(c.outcome.as_str(), "proof" | "accept"),
                    "{variant}: {v:?}"
                );
            }
        }
        if variant == "tied-decisions" {
            assert!(codes(&v).contains(&"E_SCHEDULE_INFEASIBLE"), "{v:?}");
        }
    }
}
#[test]
fn default_only_schedule_still_enumerates_two_distinct_call_contexts() {
    let funcs="perm_action @controlled (t:wire mut) (c:wire in) () () ()\n^entry():\nctl c action_call @flip (t) () () ()\nctl c action_call @flip (t) () () ()\nperm_action @flip (t:wire mut) () () () ()\n^entry():\nx t";
    let sigs = r#"{"controlled":{"abi":"0.9","args":[],"bounds":[],"in":[1],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"},"flip":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1],"tier":"q"}}"#;
    let numerical =
        json(r#"{"bound":0,"kind":"numerical","metric":"operator_norm","precision_bits":128}"#);
    let (mut inp, _) = standalone(funcs, sigs, "controlled", vec![numerical.clone()]);
    let (child, ch) = standalone(funcs, sigs, "flip", vec![numerical]);
    let mut child_cert = json(std::str::from_utf8(&child.cert_bytes).unwrap());
    let Value::Obj(ctop) = &mut child_cert else {
        unreachable!()
    };
    let Value::Obj(cr) = ctop.get_mut("regions").unwrap() else {
        unreachable!()
    };
    let mut child_region = cr.remove(&ch).unwrap();
    let Value::Obj(r) = &mut child_region else {
        unreachable!()
    };
    let Value::Arr(cs) = r.get_mut("claims").unwrap() else {
        unreachable!()
    };
    let subject = cs[0].get("subject").unwrap().clone();
    let mut subject = subject;
    set(&mut subject, "inherited_controls", n(1));
    set(&mut cs[0], "subject", subject);
    let mut cert = json(std::str::from_utf8(&inp.cert_bytes).unwrap());
    let Value::Obj(top) = &mut cert else {
        unreachable!()
    };
    let Value::Obj(regions) = top.get_mut("regions").unwrap() else {
        unreachable!()
    };
    regions.insert(ch, child_region);
    inp.cert_bytes = jcs(&cert).unwrap();
    let base = controlled();
    let target = json(std::str::from_utf8(base.target_bytes.as_ref().unwrap()).unwrap());
    let mut schedule = json(std::str::from_utf8(base.schedule_bytes.as_ref().unwrap()).unwrap());
    set(&mut schedule, "entries", Value::Arr(vec![]));
    bind_schedule(&mut inp, schedule, target);
    let v = check_certificate(&inp);
    assert!(codes(&v).is_empty(), "{v:?}");
    let child = v
        .regions
        .iter()
        .find(|r| r.name.as_deref() == Some("flip"))
        .unwrap();
    assert_eq!(child.claims[0].1.outcome, "accept", "{v:?}");
    assert_eq!(child.claims[0].1.contexts, Some(2));
}
