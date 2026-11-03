mod common;
use bennett_cert_checker::{cert::{check_certificate,Inputs},domain::{resolve_domain,ResolveError},json::{jcs,Value},object::Object,sampler::{Sampler,draw_elements},sha256_hex};
use common::*;
fn fixture(a:u32,w:u32,image:&str)->Inputs {
    let address=if a==0 {"()"}else{"c"};
    let source=format!("perm_action @a (t:wire[{w}] mut) (c:wire in) (r:region[{a},{w}] in) () ()\n^entry():\nd = qload r {address}\nqxor d t\nqunload r {address} d");
    let sig=format!(r#"{{"a":{{"abi":"0.9","args":[],"bounds":[],"in":[1],"kind":"perm_action","mem":[{{"A":{a},"W":{w},"role":"in"}}],"mut":[{w}],"tier":"q"}}}}"#);
    let bytes:Vec<u8>=(0..image.len()/2).map(|i|u8::from_str_radix(&image[2*i..2*i+2],16).unwrap()).collect();
    let hash=sha256_hex(&bytes);
    let domain=json(&format!(r#"{{"fixed_regions":[{{"name":"r","A":{a},"W":{w},"image":"{image}","sha256":"{hash}"}}],"bundles":[{{"name":"t","width":{w},"lo":0,"hi":{}}},{{"name":"c","width":1,"lo":0,"hi":1}}],"borrowed":[],"regions":[]}}"#,(1u32<<w)-1));
    let cells:Vec<u32>=(0..1u32<<a).map(|i|{let mut cell=0;for bit in 0..w {let j=i*w+bit;cell|=u32::from((bytes[(j/8)as usize]>>(j%8))&1)<<bit;}cell}).collect();
    let tuple=|t:u32,c:u32|json(&format!(r#"{{"bundles":["{t}","{c}"],"borrowed":[],"regions":[{}]}}"#,encode(&Value::Arr(cells.iter().map(|v|Value::Str(v.to_string())).collect()))));
    let mut rows=vec![];for t in 0..1u32<<w {for c in 0..2 {let mut row=json("{}");set(&mut row,"before",tuple(t,c));set(&mut row,"after",tuple(t^cells[if a==0 {0}else{c as usize}],c));rows.push(row);}}
    let mut claim=json(r#"{"kind":"exhaustive","label":"proof_relative_to_image","invocation":{"args":[],"budget":["0","0","0","0"]}}"#);set(&mut claim,"domain",domain);let mut expected=json("{}");set(&mut expected,"table",Value::Arr(rows));set(&mut claim,"expected",expected);
    let (mut inputs,h)=standalone(&source,&sig,"a",vec![claim]);
    inputs.object_text=rebind_object_symbols(&format!("{}\nsection layout\n{{\"A_tab\":0,\"H\":4,\"S_cells\":0,\"globals\":{{}},\"heap_base\":1,\"heap_limit_max\":0,\"interface_regions\":{{}},\"null_size\":1,\"ptr_width\":32}}\n",inputs.object_text));
    let object=Object::parse(&inputs.object_text).unwrap();let updated=object.symbol("a").unwrap().closure_hash.clone().unwrap();
    let mut cert=json(std::str::from_utf8(&inputs.cert_bytes).unwrap());let Value::Obj(top)=&mut cert else{panic!()};let Value::Obj(regions)=top.get_mut("regions").unwrap()else{panic!()};let mut record=regions.remove(&h).unwrap();
    let Value::Obj(fields)=&mut record else{panic!()};let Value::Obj(frame)=fields.get_mut("frame").unwrap()else{panic!()};let Value::Arr(slots)=frame.get_mut("slots").unwrap()else{panic!()};slots.push(json(&format!(r#"{{"class":"fast","id":"mem:0","role":"mem_in","width":{}}}"#,(1u32<<a)*w)));
    let Value::Arr(claims)=fields.get_mut("claims").unwrap()else{panic!()};let mut subject=claims[0].get("subject").unwrap().clone();set(&mut subject,"closure_hash",Value::Str(updated.clone()));set(&mut claims[0],"subject",subject);regions.insert(updated.clone(),record);
    set(top.get_mut("bindings").unwrap(),"object_hash",Value::Str(object.object_hash.clone()));for key in ["interface_hash","closure_hash","reference_program_hash"] {set(top.get_mut("bindings").unwrap(),key,Value::Str(updated.clone()));}
    inputs.cert_bytes=jcs(&cert).unwrap();inputs
}
fn claim_mut(inputs:&mut Inputs,f:impl FnOnce(&mut Value)) {let mut cert=json(std::str::from_utf8(&inputs.cert_bytes).unwrap());let Value::Obj(top)=&mut cert else{panic!()};let Value::Obj(regions)=top.get_mut("regions").unwrap()else{panic!()};let Value::Obj(r)=regions.values_mut().next().unwrap()else{panic!()};let Value::Arr(claims)=r.get_mut("claims").unwrap()else{panic!()};f(&mut claims[0]);inputs.cert_bytes=jcs(&cert).unwrap();}
#[test]
fn fixed_image_claims_replay_packed_cells_and_report_a_conditional_theorem() {
    for (a,w,image,fraction) in [(0,2,"03","1/4"),(1,2,"09","1/16")] {
        let inputs=fixture(a,w,image);let verdict=check_certificate(&inputs);assert!(codes(&verdict).is_empty(),"{verdict:?}");let outcome=&verdict.regions[0].claims[0].1;assert_eq!(outcome.outcome,"proof_relative_to_image","{verdict:?}");assert_eq!(outcome.covered_fraction.as_deref(),Some(fraction));assert_eq!(outcome.elements.as_deref(),Some("8"));assert!(outcome.establishes.contains("conditional"));
        let report=verdict.to_value();assert!(encode(&report).contains("fixed_regions"),"{report:?}");
    }
}
#[test]
fn conditional_and_sampled_results_cannot_satisfy_unrestricted_proof_requests() {
    for label in ["proof","proof_relative_to_image","evidence"] {
        for sampled in [false,true] {
            let mut inputs=fixture(0,2,"03");claim_mut(&mut inputs,|c|{set(c,"label",Value::Str(label.into()));if sampled {set(c,"kind",Value::Str("sampled".into()));set(c,"sampler",json(r#"{"algorithm":"xoshiro256**","seed":["1","0","0","0"],"count":1}"#));}});
            let verdict=check_certificate(&inputs);if label=="proof" ||(sampled&&label=="proof_relative_to_image"){assert!(codes(&verdict).contains(&"E_CERT_COVERAGE_LABEL"),"{label} sampled={sampled}: {verdict:?}");}else{assert!(codes(&verdict).is_empty(),"{verdict:?}");assert_eq!(verdict.regions[0].claims[0].1.outcome,if sampled{"evidence"}else{"proof_relative_to_image"});}
        }
    }
}
#[test]
fn every_fixed_region_field_type_and_removal_rejects_before_replay() {
    for key in ["name","A","W","image","sha256"] {
        for remove in [false,true] {
            let mut inputs=fixture(0,2,"03");claim_mut(&mut inputs,|c|{let Value::Obj(m)=c else{panic!()};let Value::Obj(d)=m.get_mut("domain").unwrap()else{panic!()};let Value::Arr(a)=d.get_mut("fixed_regions").unwrap()else{panic!()};let Value::Obj(r)=&mut a[0]else{panic!()};if remove{r.remove(key);}else{r.insert(key.into(),Value::Null);}});
            let verdict=check_certificate(&inputs);assert!(codes(&verdict).contains(&"E_CERT_DOMAIN"),"{key} remove={remove}: {verdict:?}");
        }
    }
}
#[test]
fn fixed_image_hash_size_padding_shape_identity_and_duplicates_reject() {
    for (key,value) in [("name",json("\"ghost\"")),("A",json("1")),("W",json("1")),("image",json("\"\"")),("image",json("\"0300\"")),("image",json("\"83\"")),("image",json("\"0A\"")),("sha256",json(&format!("\"{}\"","0".repeat(64))))] {
        let mut inputs=fixture(0,2,"03");claim_mut(&mut inputs,|c|{let Value::Obj(m)=c else{panic!()};let Value::Obj(d)=m.get_mut("domain").unwrap()else{panic!()};let Value::Arr(a)=d.get_mut("fixed_regions").unwrap()else{panic!()};set(&mut a[0],key,value);});let verdict=check_certificate(&inputs);assert!(codes(&verdict).contains(&"E_CERT_DOMAIN"),"{key}: {verdict:?}");
    }
    for overlap in [false,true] {let mut inputs=fixture(0,2,"03");claim_mut(&mut inputs,|c|{let Value::Obj(m)=c else{panic!()};let Value::Obj(d)=m.get_mut("domain").unwrap()else{panic!()};if overlap {d.insert("regions".into(),json(r#"[{"name":"r","cells":[]}]"#));}else{let Value::Arr(a)=d.get_mut("fixed_regions").unwrap()else{panic!()};a.push(a[0].clone());}});let verdict=check_certificate(&inputs);assert!(codes(&verdict).contains(&"E_CERT_DOMAIN"),"{verdict:?}");}
}
#[test]
fn fixed_cells_do_not_consume_sampler_words_or_enumeration_positions() {
    let inputs=fixture(1,2,"09");let object=Object::parse(&inputs.object_text).unwrap();let cert=json(std::str::from_utf8(&inputs.cert_bytes).unwrap());let c=cert.get("regions").unwrap().as_obj().unwrap().values().next().unwrap().get("claims").unwrap().as_arr().unwrap()[0].clone();let iface=object.interface("a").unwrap();let domain=resolve_domain(c.get("domain").unwrap(),&iface,0).unwrap();
    let draws=draw_elements(&domain,["1","2","3","4"],8).unwrap();let mut sampler=Sampler::new(["1","2","3","4"]).unwrap();for tuple in draws{assert_eq!(tuple.bundles[0],sampler.draw_component(0,3));assert_eq!(tuple.bundles[1],sampler.draw_component(0,1));assert_eq!(tuple.regions[0],vec![1u32.into(),2u32.into()]);}
    let mut mut_iface=iface.clone();mut_iface.regions[0].role="mut".into();assert!(matches!(resolve_domain(c.get("domain").unwrap(),&mut_iface,0),Err(ResolveError::Reject(_))));
}
