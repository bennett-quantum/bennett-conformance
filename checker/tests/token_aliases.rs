mod common;
use bennett_cert_checker::{cert::{check_certificate,parse_invocation,Inputs,Limits},json::jcs,object::Object,sha256_hex};
use common::*;
fn object()->Object {Object::parse(&controlled().object_text).unwrap()}
#[test]
fn equal_tuple_action_tokens_admit_every_canonical_partition_without_tuple_interning() {
    let object=object();let types=vec!["token".to_string();3];
    for aliases in ["[0,0,0]","[0,0,1]","[0,1,0]","[0,1,1]","[0,1,2]"] {
        let invocation=json(&format!(r#"{{"args":[["1","0","0","0"],["1","0","0","0"],["1","0","0","0"]],"budget":["0","0","0","0"],"token_aliases":{aliases}}}"#));
        let parsed=parse_invocation(&invocation,&types,&object,"test");assert!(parsed.is_ok(),"{aliases}: {parsed:?}");
    }
}
#[test]
fn token_partitions_reject_bad_labels_arity_missing_values_and_inconsistent_equalities() {
    let object=object();let types=vec!["token".to_string();2];
    for aliases in ["null","[1,1]","[0,2]","[0,-1]","[]","[0]","[0,1,2]","[0,true]"] {
        let invocation=json(&format!(r#"{{"args":[["1","0","0","0"],["1","0","0","0"]],"budget":["0","0","0","0"],"token_aliases":{aliases}}}"#));let errors=parse_invocation(&invocation,&types,&object,"test").unwrap_err();assert!(errors.iter().any(|e|e.code=="E_CERT_INVOCATION"),"{aliases}: {errors:?}");
    }
    for invocation in [r#"{"args":[["1","0","0","0"],["1","0","0","0"]],"budget":["0","0","0","0"]}"#,r#"{"args":[["1","0","0","0"],["2","0","0","0"]],"budget":["0","0","0","0"],"token_aliases":[0,0]}"#] {
        let errors=parse_invocation(&json(invocation),&types,&object,"test").unwrap_err();assert!(errors.iter().any(|e|e.code=="E_CERT_INVOCATION"),"{errors:?}");
    }
}
#[test]
fn published_k91_reference_certificates_bind_identity_partitions_and_replay_all_sixty_four_tuples() {
    let root=std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../bennett-isa-spec/conformance/fixtures/accepted-k91-09");let manifest=json(&std::fs::read_to_string(root.join("k91.json")).unwrap());let mut count=0;
    for name in ["pair-same","pair-distinct","wrong-partition-certificate"] {
        let case=manifest.get("fixtures").unwrap().as_arr().unwrap().iter().find(|c|c.get("name").unwrap().as_str()==Some(name)).unwrap();let bytes=std::fs::read(root.join(case.get("object").unwrap().as_str().unwrap())).unwrap();assert_eq!(sha256_hex(&bytes),case.get("sha256").unwrap().as_str().unwrap());let object_text=String::from_utf8(bytes).unwrap();let object=Object::parse(&object_text).unwrap();assert!(object.stated_hashes_ok());let cert=json(object.certificate_raw.as_deref().unwrap());
        let inputs=Inputs{cert_bytes:jcs(&cert).unwrap(),object_text,target_bytes:None,schedule_bytes:None,libraries:vec![],oracle_bin:None,limits:Limits::default()};let verdict=check_certificate(&inputs);
        if name=="wrong-partition-certificate" {assert!(codes(&verdict).contains(&"E_CERT_BINDING"),"{verdict:?}");}
        else {assert!(codes(&verdict).is_empty(),"{name}: {verdict:?}");let outcome=&verdict.regions[0].claims[0].1;assert_eq!(outcome.outcome,"proof","{name}: {verdict:?}");assert_eq!(outcome.elements.as_deref(),Some("64"));}
        count+=1;
    }
    assert_eq!(count,3);
}
