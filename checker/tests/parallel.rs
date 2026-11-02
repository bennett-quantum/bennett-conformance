mod common;
use common::*;
use bennett_cert_checker::cert::check_certificate;
use bennett_cert_checker::json::{jcs,n,Value};

fn program(first:u64,second:u64,third:bool,parallel:u64,time:u64)->bennett_cert_checker::cert::Inputs {
    let body=if third {"x t\nx u\nx u"}else{"x t\nx u"};
    let (mut inp,h)=standalone(&format!("perm_action @parallel (t:wire mut, u:wire mut) () () () ()\n^entry():\n{body}"),r#"{"parallel":{"abi":"0.9","args":[],"bounds":[],"in":[],"kind":"perm_action","mem":[],"mut":[1,1],"tier":"q"}}"#,"parallel",vec![json(r#"{"kind":"numerical","bound":0,"metric":"operator_norm","precision_bits":64}"#)]);
    let mut cert=json(std::str::from_utf8(&inp.cert_bytes).unwrap());let Value::Obj(top)=&mut cert else {unreachable!()};let Value::Obj(regions)=top.get_mut("regions").unwrap() else {unreachable!()};let region=regions.get_mut(&h).unwrap();
    set(region,"resources",json(&format!(r#"{{"peak_qubits":2,"time":{time},"qubit_second":{}}}"#,2*time)));
    set(region,"gates_used",json(&format!(r#"{{"x":{{"count":{}}}}}"#,if third{3}else{2})));
    set(region,"error",json(r#"{"forward:0":{"local":{"operator_norm":0},"inclusive":{"operator_norm":0}}}"#));let mut multiplicities=json("{}");set(&mut multiplicities,&format!("{h}:forward:0"),n(1));set(top.get_mut("composition").unwrap(),"multiplicities",multiplicities);inp.cert_bytes=jcs(&cert).unwrap();
    let base=controlled();let mut target=json(std::str::from_utf8(base.target_bytes.as_ref().unwrap()).unwrap());
    set(&mut target,"gates",json(r#"[{"op":"x","classes":["fast"],"controls":{"min":0,"max":0},"epsilon":0,"cost":{"time":2}}]"#));
    let Value::Obj(top)=&mut target else {unreachable!()};set(top.get_mut("capabilities").unwrap(),"max_parallel",n(parallel));
    let mut schedule=json(std::str::from_utf8(base.schedule_bytes.as_ref().unwrap()).unwrap());
    let mut entries=vec![];for (ordinal,start) in [(0,first),(1,second)] {entries.push(json(&format!(r#"{{"path":[{{"symbol":"{h}","point":"entry:{ordinal}","iteration":[]}}],"generated":0,"step":"*","phase":"*","kind":"start","start":{{"time":{start}}}}}"#)));}
    set(&mut schedule,"entries",Value::Arr(entries));bind_schedule(&mut inp,schedule,target);inp
}
#[test]
fn disjoint_overlap_and_out_of_order_ends_use_makespan(){
    for (first,second,third,max,time) in [(0,0,false,2,2),(10,0,false,1,12),(10,0,true,1,14),(0,2,false,1,4)] {
        let v=check_certificate(&program(first,second,third,max,time));assert_eq!(v.overall,"verified","{first}/{second}/{third}: {v:?}");
    }
}
#[test]
fn parallel_limit_is_checked_even_for_disjoint_wires(){
    let v=check_certificate(&program(0,0,false,1,2));assert!(codes(&v).contains(&"E_SCHEDULE_INFEASIBLE"),"{v:?}");
}
#[test]
fn overlapping_events_on_the_same_wire_are_infeasible(){
    let mut inp=program(0,0,true,3,2);let mut schedule=json(std::str::from_utf8(inp.schedule_bytes.as_ref().unwrap()).unwrap());
    let Value::Obj(s)=&mut schedule else {unreachable!()};let Value::Arr(entries)=s.get_mut("entries").unwrap() else {unreachable!()};let mut third=entries[1].clone();
    let Value::Obj(e)=&mut third else {unreachable!()};let Value::Arr(path)=e.get_mut("path").unwrap() else {unreachable!()};set(&mut path[0],"point",Value::Str("entry:2".into()));entries.push(third);
    let target=json(std::str::from_utf8(inp.target_bytes.as_ref().unwrap()).unwrap());bind_schedule(&mut inp,schedule,target);
    let v=check_certificate(&inp);assert!(codes(&v).contains(&"E_SCHEDULE_INFEASIBLE"),"{v:?}");
}
