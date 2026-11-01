//! Section 9.2 identity tests. Expected values come from the spec text
//! (section 16.3, case derivations) and from black-box runs of the pinned
//! reference binary recorded as literals (`hash` subcommand outputs).

use bennett_cert_checker::object::Object;
use std::path::PathBuf;

fn isa_spec_dir() -> PathBuf {
    if let Ok(d) = std::env::var("BENNETT_ISA_SPEC_DIR") {
        return PathBuf::from(d);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../bennett-isa-spec")
}

fn load(rel: &str) -> Object {
    let text = std::fs::read_to_string(isa_spec_dir().join(rel))
        .unwrap_or_else(|e| panic!("read {rel}: {e}"));
    Object::parse(&text).unwrap_or_else(|e| panic!("parse {rel}: {e}"))
}

#[test]
fn inc8_08_hashes_match_case_c163() {
    let o = load("conformance/cases/objects/inc8.bisa");
    assert_eq!(o.object_hash, "8093da48bad04d6564325faf01ea1304a84b5ac5a120e685eee9951f5daec5d9");
    let s = o.symbol("inc8").unwrap();
    assert_eq!(s.interface_hash, "8ce5e64f15830d8e596ba9b4861def6565dd567b5804300f427d8b9566d5be93");
    assert_eq!(s.closure_hash.as_deref(), Some("8ce5e64f15830d8e596ba9b4861def6565dd567b5804300f427d8b9566d5be93"));
    assert!(o.stated_hashes_ok());
}

#[test]
fn object_hash_deletes_schedule_section() {
    // Case F-object-hash-schedule-deleted.
    let o = load("conformance/cases/objects/sched_inc8.bisa");
    assert_eq!(o.object_hash, "8093da48bad04d6564325faf01ea1304a84b5ac5a120e685eee9951f5daec5d9");
    assert!(o.schedule.is_some());
}

#[test]
fn example1_09_from_spec_text() {
    let text = "\
bennett-object isa=0.9 object=0.9 launch=0.9 sched=0.9 cert=0.9 coverage=0.9 target=0.9 ptr_width=32

section symbols
{\"0\":{\"interface_hash\":\"ed9865300b85a519c2379de98e36e635e72b5b78d8bdfca03dccf9a09aa3df5e\",\"kind\":\"oracle\",\"name\":\"inc8\",\"signature\":{\"abi\":\"0.9\",\"args\":[],\"fault\":4,\"in\":[8],\"kind\":\"oracle\",\"mem\":[],\"tgt\":8,\"tier\":\"q\"}}}

section functions
oracle @inc8 (x:wire[8] in) (t:wire[8]) (f:wire[4]) () ()
regs { one:reg[8], r:reg[8] }
^entry():
one = lift.const.i8 1
r = lift.add x one
lift.retval r
";
    let o = Object::parse(text).unwrap();
    assert_eq!(
        o.symbol("inc8").unwrap().interface_hash,
        "ed9865300b85a519c2379de98e36e635e72b5b78d8bdfca03dccf9a09aa3df5e"
    );
    assert!(o.stated_hashes_ok());
}

#[test]
fn controlled_09_hashes_and_sections() {
    let o = load("conformance/fixtures/controlled-action-09/controlled.bisa");
    assert_eq!(o.object_hash, "45bbb506541e3a8bb28a18e1bb0aef16ba01ee1e7cac74d5a190deb3ad36ef5a");
    assert_eq!(
        o.symbol("controlled").unwrap().closure_hash.as_deref(),
        Some("df131e972083b8d3b3b7d9341342b55252ca7a1963c4b7b6f702b9cdf3fbde12")
    );
    assert_eq!(
        o.symbol("flip").unwrap().closure_hash.as_deref(),
        Some("7ff7153a2ad3b857ec9eb86ca99b9bcbc605f60093d28e99fb749ed16d13274b")
    );
    assert!(o.certificate.is_some() && o.schedule.is_some());
    assert!(o.stated_hashes_ok());
    // Program points: both regions have block entry with one instruction.
    assert_eq!(o.instruction_count("controlled", "entry"), Some(1));
    assert_eq!(o.instruction_count("flip", "entry"), Some(1));
}

#[test]
fn mutual_scc_hashes_match_black_box() {
    // Recorded from `bennett-isa-ref hash mutual.bisa` (black box).
    let o = load("conformance/cases/objects/mutual.bisa");
    assert_eq!(o.object_hash, "ba0df453b2ada3a402fc6ab4f884c82311067f3c0df66e0249c83b29109accf8");
    assert_eq!(o.symbol("f").unwrap().interface_hash, "e4028cf7347c2779e19c8193ef1039837944acebd4784e9c3a42fbe236e737ec");
    assert_eq!(o.symbol("g").unwrap().interface_hash, "7dc9c25aff5230a46ae697edf187d34ba72a03f7a3c1a520c6fc6bf5462c6374");
    assert_eq!(o.symbol("main").unwrap().interface_hash, "598fdf171d0a201d52e8fbe2339c70d1a26b36a72dcd60503c87c51dff88039b");
    assert!(o.stated_hashes_ok());
}

#[test]
fn sum_db_self_scc_layout_and_data_hashes_match_black_box() {
    let o = load("conformance/cases/objects/sum_db.bisa");
    assert_eq!(o.object_hash, "ce1a10eb007786b5a1675892154ec4ac30663a051eeeeee41057596b1db26240");
    assert_eq!(o.symbol("sum").unwrap().interface_hash, "74f110bbafde68a4785025e7e571ef871ddd83a09465ab9c1604f653a0cfcd92");
    assert_eq!(o.symbol("sum_db").unwrap().interface_hash, "68225d213c44812fe844cd295d8821127c3cf31682be4eb19303cb53f9b147f7");
    assert_eq!(o.symbol("zero").unwrap().interface_hash, "7002ed5ea010907a0ae99065663f22fb7d18ca1d6cb171fed57e2594bd9367e6");
    assert!(o.stated_hashes_ok());
}

#[test]
fn registers_and_t3_domains_hashes_match_black_box() {
    let o = load("conformance/cases/objects/registers.bisa");
    assert_eq!(o.symbol("toggle").unwrap().interface_hash, "de3621c3dee0d970a86479e38ff4088730604a5b4d16429b71af2ec85ab0f42f");
    assert!(o.stated_hashes_ok());
    let o = load("conformance/cases/objects/t3_domains.bisa");
    assert_eq!(o.symbol("inc2").unwrap().interface_hash, "d42f3ddc94103b7c18a08340ae54ab5d98d173bae714c5f7477039b1c7b3543e");
    assert!(o.stated_hashes_ok());
}

#[test]
fn corrupt_stated_hash_is_detected() {
    let o = load("conformance/cases/objects/f_hash_corrupt.bisa");
    assert!(!o.stated_hashes_ok(), "stated interface_hash must not match recomputation");
    // The recomputed hash is still the true inc8 hash.
    assert_eq!(o.symbol("inc8").unwrap().interface_hash, "8ce5e64f15830d8e596ba9b4861def6565dd567b5804300f427d8b9566d5be93");
}

#[test]
fn section_order_and_unknown_section_rejected() {
    let text = std::fs::read_to_string(isa_spec_dir().join("conformance/cases/objects/f_section_order.bisa")).unwrap();
    assert!(Object::parse(&text).is_err());
    let text = std::fs::read_to_string(isa_spec_dir().join("conformance/cases/objects/f_unknown_section.bisa")).unwrap();
    assert!(Object::parse(&text).is_err());
}

#[test]
fn header_names_and_widths_resolve() {
    let o = load("conformance/cases/objects/t3_domains.bisa");
    let iface = o.interface("borrow_tuple").expect("action interface");
    // (t:wire mut, b:wire mut) (x:wire in): mut slots t,b width 1; in slot x width 1.
    assert_eq!(iface.bundles.len(), 3);
    assert_eq!(iface.bundles[0].name, "t");
    assert_eq!(iface.bundles[0].group, "mut");
    assert_eq!(iface.bundles[2].name, "x");
    assert_eq!(iface.bundles[2].group, "in");
    let oracle = o.interface("inc2").unwrap();
    let names: Vec<&str> = oracle.bundles.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, ["x", "t", "f"]);
    let widths: Vec<u64> = oracle.bundles.iter().map(|b| b.width).collect();
    assert_eq!(widths, [2, 2, 4]);
}
