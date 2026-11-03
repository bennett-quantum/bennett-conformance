//! Certificate checking (ISA section 12): strict schemas, hash bindings,
//! schedule bindings (section 11 as far as the certificate binds them),
//! claim dispatch, and the deterministic JSON verdict.

use crate::domain::{resolve_domain, ResolvedDomain};
use crate::eval;
use crate::fault::{HostLimit, Reject, ResolveError};
use crate::json::{jcs, n, parse, s, Value};
use crate::numeric;
use crate::object::{is_version, Object};
use crate::rat::{is_canonical_uint_str, parse_biguint_str, Rat};
use crate::{is_hash_hex, sha256_hex};
use num_bigint::BigUint;
use num_traits::Zero;
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const CHECKER_NAME: &str = "bennett-cert-checker";
pub const REF_09_HASH: &str = "5de4e4995350313e50217880ba1b95de39cd043283b726337d4894efb005d197";
pub(crate) const BORROW_PROOF_PENDING:&str="verification.borrow_frames has not been mapped to every maximal frame and its normative entry-state domain D_F (§5.6)";
pub(crate) const FRAME_STORAGE_PENDING:&str="frame descriptor lifetimes/storage widths have not been checked against the complete realised leaf expansion (§9.4)";

#[derive(Debug, Clone)]
pub struct Limits {
    pub max_elements: u64,
    pub max_wires: u32,
    /// Per relation-side replay, shared through nested oracle/action calls.
    pub max_reference_steps: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_elements: 1 << 20,
            max_wires: 10,
            max_reference_steps: 1 << 20,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Inputs {
    pub cert_bytes: Vec<u8>,
    pub object_text: String,
    pub target_bytes: Option<Vec<u8>>,
    pub schedule_bytes: Option<Vec<u8>>,
    pub libraries: Vec<Vec<u8>>,
    pub oracle_bin: Option<PathBuf>,
    pub limits: Limits,
}

#[derive(Debug, Clone, Default)]
pub struct ClaimOutcome {
    /// proof | proof_relative_to_image | evidence | accept | reject | unchecked
    pub outcome: String,
    pub establishes: String,
    pub elements: Option<String>,
    pub draws: Option<String>,
    pub distinct: Option<String>,
    pub covered_fraction: Option<String>,
    /// Checker-computed bound (rational "n/d" or decimal), numerical/channel.
    pub computed: Option<String>,
    pub contexts: Option<u64>,
    /// K85 image theorem scope, never a preparation assertion.
    pub image_bindings: Option<Value>,
    pub errors: Vec<Reject>,
    pub unchecked_reason: Option<String>,
    pub(crate) borrow_frame: Option<crate::structural::BorrowFrameEvidence>,
}

impl ClaimOutcome {
    pub fn unchecked(reason: impl Into<String>) -> ClaimOutcome {
        ClaimOutcome {
            outcome: "unchecked".into(),
            unchecked_reason: Some(reason.into()),
            ..Default::default()
        }
    }
    pub fn rejected(errors: Vec<Reject>) -> ClaimOutcome {
        ClaimOutcome {
            outcome: "reject".into(),
            errors,
            ..Default::default()
        }
    }
    fn to_value(&self, index: usize, kind: &str) -> Value {
        let mut m = BTreeMap::new();
        m.insert("index".into(), n(index));
        m.insert("kind".into(), s(kind));
        m.insert("outcome".into(), s(&self.outcome));
        m.insert("establishes".into(), s(&self.establishes));
        if let Some(v) = &self.elements {
            m.insert("elements".into(), s(v));
        }
        if let Some(v) = &self.draws {
            m.insert("draws".into(), s(v));
        }
        if let Some(v) = &self.distinct {
            m.insert("distinct".into(), s(v));
        }
        if let Some(v) = &self.covered_fraction {
            m.insert("covered_fraction".into(), s(v));
        }
        if let Some(v) = &self.computed {
            m.insert("computed_bound".into(), s(v));
        }
        if let Some(v) = self.contexts {
            m.insert("contexts".into(), n(v));
        }
        if let Some(v)=&self.image_bindings {m.insert("image_bindings".into(),v.clone());}
        if let Some(v) = &self.unchecked_reason {
            m.insert("unchecked_reason".into(), s(v));
        }
        m.insert(
            "errors".into(),
            Value::Arr(self.errors.iter().map(Reject::to_value).collect()),
        );
        Value::Obj(m)
    }
}

#[derive(Debug, Default)]
pub struct RegionReport {
    pub closure_hash: String,
    pub name: Option<String>,
    pub claims: Vec<(String, ClaimOutcome)>,
    pub errors: Vec<Reject>,
    pub unchecked_items: Vec<String>,
    pub establishes: Vec<String>,
}

#[derive(Debug, Default)]
pub struct Verdict {
    pub overall: String, // verified | rejected | unchecked
    pub establishes: Vec<String>,
    pub errors: Vec<Reject>,
    pub unchecked_items: Vec<String>,
    pub regions: Vec<RegionReport>,
    /// Recomputed hash bindings that were checked (name -> value).
    pub hashes: BTreeMap<String, String>,
}

impl Verdict {
    pub fn exit_code(&self) -> i32 {
        match self.overall.as_str() {
            "verified" => 0,
            "rejected" => 1,
            _ => 2,
        }
    }
    pub fn to_value(&self) -> Value {
        let mut m = BTreeMap::new();
        m.insert("checker".into(), s(CHECKER_NAME));
        m.insert("overall".into(), s(&self.overall));
        m.insert(
            "establishes".into(),
            Value::Arr(self.establishes.iter().map(|e| s(e)).collect()),
        );
        m.insert(
            "hashes".into(),
            Value::Obj(self.hashes.iter().map(|(k, v)| (k.clone(), s(v))).collect()),
        );
        m.insert(
            "errors".into(),
            Value::Arr(self.errors.iter().map(Reject::to_value).collect()),
        );
        m.insert(
            "unchecked_items".into(),
            Value::Arr(self.unchecked_items.iter().map(|e| s(e)).collect()),
        );
        let mut regions = BTreeMap::new();
        for r in &self.regions {
            let mut rm = BTreeMap::new();
            if !r.establishes.is_empty() {
                rm.insert(
                    "establishes".into(),
                    Value::Arr(r.establishes.iter().map(|s| crate::json::s(s)).collect()),
                );
            }
            if let Some(name) = &r.name {
                rm.insert("name".into(), s(name));
            }
            rm.insert(
                "claims".into(),
                Value::Arr(
                    r.claims
                        .iter()
                        .enumerate()
                        .map(|(i, (kind, o))| o.to_value(i, kind))
                        .collect(),
                ),
            );
            rm.insert(
                "errors".into(),
                Value::Arr(r.errors.iter().map(Reject::to_value).collect()),
            );
            rm.insert(
                "unchecked_items".into(),
                Value::Arr(r.unchecked_items.iter().map(|e| s(e)).collect()),
            );
            regions.insert(r.closure_hash.clone(), Value::Obj(rm));
        }
        m.insert("regions".into(), Value::Obj(regions));
        Value::Obj(m)
    }
    pub fn to_json_bytes(&self) -> Vec<u8> {
        jcs(&self.to_value()).expect("verdict value is always JCS-serialisable")
    }
}

// ---------- small schema helpers ----------

pub fn rej(
    code: &'static str,
    section: &'static str,
    path: impl AsRef<str>,
    msg: impl AsRef<str>,
) -> Reject {
    Reject::new(
        code,
        section,
        path.as_ref().to_string(),
        msg.as_ref().to_string(),
    )
}

pub fn schema_err(path: impl Into<String>, msg: impl Into<String>) -> Reject {
    Reject::new("E_CERT_SCHEMA", "§12.3", path, msg)
}

pub(crate) fn check_keys(
    v: &Value,
    allowed: &[&str],
    required: &[&str],
    path: &str,
    errs: &mut Vec<Reject>,
) -> bool {
    let Some(obj) = v.as_obj() else {
        errs.push(schema_err(
            path.to_string(),
            format!("expected object, got {}", v.kind()),
        ));
        return false;
    };
    let mut ok = true;
    for k in obj.keys() {
        if !allowed.contains(&k.as_str()) {
            errs.push(schema_err(
                format!("{path}.{k}"),
                format!("unknown field {k:?}"),
            ));
            ok = false;
        }
    }
    for r in required {
        if !obj.contains_key(*r) {
            errs.push(schema_err(
                format!("{path}.{r}"),
                format!("missing required field {r:?}"),
            ));
            ok = false;
        }
    }
    ok
}

/// Integer JSON number with |v| < 2^53 (section 9.1 metadata rule).
pub fn int_of(v: &Value) -> Option<i128> {
    let Value::Num(lex) = v else { return None };
    if lex.contains(['.', 'e', 'E']) {
        return None;
    }
    let i: i128 = lex.parse().ok()?;
    if i.unsigned_abs() >= (1u128 << 53) {
        return None;
    }
    Some(i)
}

pub fn u64_of(v: &Value) -> Option<u64> {
    match v {
        Value::Str(text) if is_canonical_uint_str(text) => text.parse().ok(),
        _ => u64::try_from(int_of(v)?).ok(),
    }
}

fn hash_field(v: &Value, path: &str, errs: &mut Vec<Reject>) -> Option<String> {
    match v.as_str() {
        Some(h) if is_hash_hex(h) => Some(h.to_string()),
        _ => {
            errs.push(schema_err(
                path.to_string(),
                "expected 64-lowercase-hex hash string",
            ));
            None
        }
    }
}

fn nonempty_str(v: &Value, path: &str, errs: &mut Vec<Reject>) -> Option<String> {
    match v.as_str() {
        Some(x) if !x.is_empty() => Some(x.to_string()),
        _ => {
            errs.push(schema_err(path.to_string(), "expected nonempty string"));
            None
        }
    }
}

// ---------- invocation (section 12.3, K13) ----------

#[derive(Debug, Clone)]
pub struct Invocation {
    pub args: Vec<Value>,
    pub budget: [BigUint; 4],
    pub params_hash: String,
}

pub fn validate_budget(budget: &[BigUint; 4], object: &Object, path: &str, errs: &mut Vec<Reject>) {
    // Section 5.4 bennett_budget checks that the object's layout decides.
    match &object.layout {
        None => {
            // Memoryless object: heap, stack, depth limits are 0.
            if !budget[1].is_zero() || !budget[2].is_zero() || !budget[3].is_zero() {
                errs.push(rej(
                    "E_CERT_INVOCATION",
                    "§5.4",
                    path.to_string(),
                    "memoryless object requires heap/stack/depth budget components to be 0",
                ));
            }
        }
        Some(layout) => {
            let lim = |key: &str| layout.get(key).and_then(u64_of).map(BigUint::from);
            if let Some(max) = lim("heap_limit_max") {
                if budget[1] > max {
                    errs.push(rej(
                        "E_CERT_INVOCATION",
                        "§5.4",
                        path.to_string(),
                        "heap_limit exceeds layout.heap_limit_max",
                    ));
                }
            }
            if let Some(s) = layout.get("S_cells").and_then(u64_of) {
                let bits = budget[2].bits();
                let exceeds = s
                    .checked_add(1)
                    .map(|b| bits > b || (bits == b && budget[2] > (BigUint::from(1u32) << s)))
                    .unwrap_or(false);
                if exceeds {
                    errs.push(rej(
                        "E_CERT_INVOCATION",
                        "§5.4",
                        path.to_string(),
                        "stack_limit exceeds 2^S_cells",
                    ));
                }
            }
            // depth_limit < 2^Dc - 1 needs section 3.7 counter widths not
            // derivable from the layout alone; see docs/checker.md.
        }
    }
}

/// K4 entry tuples use the metadata integer encoding (§9.1); unlike an
/// invocation's string-only encoding, both safe JSON integers and canonical
/// decimal strings represent exact nonnegative components here.
pub(crate) fn parse_entry_budget(
    v: &Value,
    object: &Object,
    path: &str,
) -> Result<[BigUint; 4], Vec<Reject>> {
    let mut errors = Vec::new();
    let Some(components) = v.as_arr().filter(|a| a.len() == 4) else {
        return Err(vec![rej(
            "E_CERT_INVOCATION",
            "§12.3",
            path,
            "entry budget must contain exactly four nonnegative integer components",
        )]);
    };
    let mut tuple: [BigUint; 4] = Default::default();
    for (i, v) in components.iter().enumerate() {
        match v.as_str().and_then(parse_biguint_str).or_else(|| {
            int_of(v)
                .filter(|i| *i >= 0)
                .map(|n| BigUint::from(n as u64))
        }) {
            Some(value) => tuple[i] = value,
            None => errors.push(rej(
                "E_CERT_INVOCATION",
                "§12.3",
                format!("{path}[{i}]"),
                "expected an exact nonnegative integer; no zero/default is inferred",
            )),
        }
    }
    validate_budget(&tuple, object, path, &mut errors);
    if errors.is_empty() {
        Ok(tuple)
    } else {
        Err(errors)
    }
}

pub fn parse_invocation(
    v: &Value,
    arg_types: &[String],
    object: &Object,
    path: &str,
) -> Result<Invocation, Vec<Reject>> {
    let mut errs = Vec::new();
    if !check_keys(v, &["args", "budget"], &["args", "budget"], path, &mut errs) {
        return Err(errs);
    }
    // Universal structural binding: {args:"all", budget:"all"} is handled by
    // the structural-claim path, never here.
    let args = match v.get("args") {
        Some(Value::Arr(a)) => a.clone(),
        _ => {
            errs.push(rej(
                "E_CERT_INVOCATION",
                "§12.3",
                format!("{path}.args"),
                "args must be an explicit array for this claim kind",
            ));
            return Err(errs);
        }
    };
    if args.len() != arg_types.len() {
        errs.push(rej(
            "E_CERT_INVOCATION",
            "§12.3",
            format!("{path}.args"),
            format!(
                "invocation binds {} arguments; signature has {} (no implicit zero/default)",
                args.len(),
                arg_types.len()
            ),
        ));
        return Err(errs);
    }
    for (i, (a, ty)) in args.iter().zip(arg_types.iter()).enumerate() {
        validate_arg_value(a, ty, object, &format!("{path}.args[{i}]"), &mut errs);
    }
    let budget = match v.get("budget") {
        Some(Value::Arr(b)) if b.len() == 4 => {
            let mut out: [BigUint; 4] = Default::default();
            for (i, c) in b.iter().enumerate() {
                match c.as_str().and_then(parse_biguint_str) {
                    Some(x) => out[i] = x,
                    None => errs.push(rej(
                        "E_CERT_INVOCATION",
                        "§12.3",
                        format!("{path}.budget[{i}]"),
                        "budget components are canonical nonnegative decimal strings",
                    )),
                }
            }
            out
        }
        _ => {
            errs.push(rej(
                "E_CERT_INVOCATION",
                "§12.3",
                format!("{path}.budget"),
                "budget must be four explicit components",
            ));
            return Err(errs);
        }
    };
    validate_budget(&budget, object, &format!("{path}.budget"), &mut errs);
    if !errs.is_empty() {
        return Err(errs);
    }
    let params_hash = sha256_hex(&jcs(&Value::Arr(args.clone())).unwrap_or_default());
    Ok(Invocation {
        args,
        budget,
        params_hash,
    })
}

pub fn validate_arg_value(
    a: &Value,
    ty: &str,
    object: &Object,
    path: &str,
    errs: &mut Vec<Reject>,
) {
    let bad = |errs: &mut Vec<Reject>, msg: &str| {
        errs.push(rej(
            "E_CERT_INVOCATION",
            "§12.3",
            path.to_string(),
            msg.to_string(),
        ));
    };
    if ty == "token" {
        match a {
            Value::Arr(t) if t.len() == 4 && t.iter().all(|c| c.as_str().map(is_canonical_uint_str).unwrap_or(false)) => {
                let tuple: [BigUint; 4] = [
                    parse_biguint_str(t[0].as_str().unwrap()).unwrap(),
                    parse_biguint_str(t[1].as_str().unwrap()).unwrap(),
                    parse_biguint_str(t[2].as_str().unwrap()).unwrap(),
                    parse_biguint_str(t[3].as_str().unwrap()).unwrap(),
                ];
                validate_budget(&tuple, object, path, errs);
            }
            _ => bad(errs, "token argument must be its four-component tuple of canonical decimal strings, not an id"),
        }
        return;
    }
    if ty == "f64" {
        match a.as_str() {
            Some(x)
                if x.len() == 18
                    && x.starts_with("0x")
                    && x[2..]
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) => {}
            _ => bad(
                errs,
                "f64 argument must be 0x followed by exactly 16 lowercase hex digits",
            ),
        }
        return;
    }
    let width: Option<u64> = if ty == "ptr" {
        Some(object.header.ptr_width)
    } else if let Some(rest) = ty.strip_prefix('i') {
        rest.parse().ok()
    } else {
        None
    };
    let Some(width) = width else {
        bad(errs, &format!("unsupported classical argument type {ty:?}"));
        return;
    };
    match a.as_str().and_then(parse_biguint_str) {
        Some(x) => {
            if width < 1_000_000 && x >= (BigUint::from(1u32) << width) {
                bad(
                    errs,
                    &format!("argument value does not fit {ty} ({width} bits)"),
                );
            } else if x.bits() > width {
                bad(
                    errs,
                    &format!("argument value does not fit {ty} ({width} bits)"),
                );
            }
        }
        None => bad(
            errs,
            &format!("{ty} argument must be a canonical unsigned decimal string"),
        ),
    }
}

// ---------- subject ----------

#[derive(Debug, Clone)]
pub struct Subject {
    pub closure_hash: String,
    pub variant: String,
    pub inherited_controls: u64,
    pub schedule_hash: Option<String>,
}

fn parse_subject(v: &Value, path: &str, require_schedule: bool) -> Result<Subject, Vec<Reject>> {
    let mut errs = Vec::new();
    check_keys(
        v,
        &[
            "closure_hash",
            "variant",
            "inherited_controls",
            "schedule_hash",
        ],
        &["closure_hash", "variant", "inherited_controls"],
        path,
        &mut errs,
    );
    if require_schedule && v.get("schedule_hash").is_none() {
        errs.push(rej(
            "E_CERT_BINDING",
            "§12.1",
            format!("{path}.schedule_hash"),
            "this claim kind requires schedule_hash in its subject",
        ));
    }
    let closure_hash = v
        .get("closure_hash")
        .map(|x| hash_field(x, &format!("{path}.closure_hash"), &mut errs))
        .unwrap_or_default();
    let variant = v
        .get("variant")
        .map(|x| nonempty_str(x, &format!("{path}.variant"), &mut errs))
        .unwrap_or_default();
    let inherited_controls = v
        .get("inherited_controls")
        .and_then(u64_of)
        .unwrap_or_else(|| {
            errs.push(schema_err(
                format!("{path}.inherited_controls"),
                "expected nonnegative integer",
            ));
            0
        });
    let schedule_hash = v
        .get("schedule_hash")
        .map(|x| hash_field(x, &format!("{path}.schedule_hash"), &mut errs))
        .unwrap_or_default();
    if errs.is_empty() {
        Ok(Subject {
            closure_hash: closure_hash.unwrap(),
            variant: variant.unwrap(),
            inherited_controls,
            schedule_hash,
        })
    } else {
        Err(errs)
    }
}

pub fn key_of(hash: &str, variant: &str, controls: u64) -> String {
    format!("{hash}:{variant}:{controls}")
}
pub fn inner_key(variant: &str, controls: u64) -> String {
    format!("{variant}:{controls}")
}

// ---------- document loading ----------

fn load_jcs_doc(bytes: &[u8], what: &str, verdict: &mut Verdict) -> Option<Value> {
    let text = match std::str::from_utf8(bytes) {
        Ok(t) => t,
        Err(_) => {
            verdict.errors.push(rej(
                "E_CERT_JSON",
                "§12",
                what.to_string(),
                format!("{what} is not UTF-8"),
            ));
            return None;
        }
    };
    let v = match parse(text) {
        Ok(v) => v,
        Err(e) => {
            verdict.errors.push(rej(
                "E_CERT_JSON",
                "§12",
                what.to_string(),
                format!("{what}: {e}"),
            ));
            return None;
        }
    };
    match jcs(&v) {
        Ok(canon) if canon == bytes => {
            if let Some(path) = crate::json::oversized_metadata_number(&v, what) {
                let code = match what {
                    "target" => "E_CERT_TARGET",
                    "schedule" => "E_CERT_SCHEDULE_BINDING",
                    _ => "E_CERT_SCHEMA",
                };
                verdict.errors.push(rej(
                    code,
                    "§9.1",
                    path,
                    "JSON metadata numbers >= 2^53 are forbidden; use exact decimal strings",
                ));
                return None;
            }
            if let Some(limit) = crate::decimal_limits::document(&v, what) {
                verdict.unchecked_items.push(format!(
                    "unchecked (host limit): {} (§12.2)",
                    limit.quantity
                ));
                None
            } else {
                Some(v)
            }
        }
        _ => {
            verdict.errors.push(rej(
                "E_CERT_JCS",
                "§12",
                what.to_string(),
                format!("{what} bytes are not RFC 8785 canonical (JCS)"),
            ));
            None
        }
    }
}

pub struct LoadedDocs {
    pub root_closure: String,
    pub root_invocation: Option<Invocation>,
    pub object: Object,
    pub target: Option<Value>,
    pub schedule: Option<Value>,
    pub schedule_hash: Option<String>,
    pub expansion: Option<(String, String, String)>, // name, version, hash (effective)
    pub expansion_unknown: bool,
}

/// Check one certificate against its object (and target/schedule when bound).
pub fn check_certificate(inputs: &Inputs) -> Verdict {
    let mut verdict = Verdict::default();
    let Some(cert) = load_jcs_doc(&inputs.cert_bytes, "certificate", &mut verdict) else {
        return finish(verdict);
    };
    let object = match Object::parse(&inputs.object_text) {
        Ok(o) => o,
        Err(e) => {
            let (section, msg) = match e.split_once(": ") {
                Some((sec, rest)) if sec.starts_with('§') => (sec.to_string(), rest.to_string()),
                _ => ("§9.1".to_string(), e),
            };
            verdict.errors.push(Reject {
                code: "E_CERT_OBJECT",
                section: Box::leak(section.into_boxed_str()),
                path: "object".into(),
                message: msg,
            });
            return finish(verdict);
        }
    };
    verdict
        .hashes
        .insert("object_hash".into(), object.object_hash.clone());
    if !object.stated_hashes_ok() {
        verdict.errors.push(rej(
            "E_CERT_HASH",
            "§9.2",
            "object.symbols",
            "object's stated interface_hash values do not match recomputation; the object is mislabelled",
        ));
        return finish(verdict);
    }

    crate::coverage::validate(&object, &mut verdict.errors);
    if !verdict.errors.is_empty() {
        return finish(verdict);
    }
    if object.coverage.is_some() {
        verdict.establishes.push("coverage closed schema, conditional fields, exact counts and program-point bindings to interface hashes were checked (§9.1/§9.2/§13); source classification correspondence remains unchecked".into());
    }
    // Certificate top-level schema (section 12.3), strict fields.
    let mut schema_errs = Vec::new();
    check_keys(
        &cert,
        &["versions", "bindings", "producer", "regions", "composition"],
        &["versions", "bindings", "producer", "regions", "composition"],
        "",
        &mut schema_errs,
    );
    if let Some(v) = cert.get("versions") {
        let mut e = Vec::new();
        if check_keys(
            v,
            &[
                "isa", "object", "launch", "sched", "cert", "coverage", "target",
            ],
            &[
                "isa", "object", "launch", "sched", "cert", "coverage", "target",
            ],
            "versions",
            &mut e,
        ) {
            for k in [
                "isa", "object", "launch", "sched", "cert", "coverage", "target",
            ] {
                match v.get(k).and_then(Value::as_str) {
                    Some(x) if is_version(x) => {
                        if object.header.versions.get(k).map(String::as_str) != Some(x) {
                            e.push(rej(
                                "E_CERT_VERSION",
                                "§12.3",
                                format!("versions.{k}"),
                                format!(
                                    "certificate {k} version {x} != object header {:?}",
                                    object.header.versions.get(k)
                                ),
                            ));
                        }
                    }
                    _ => e.push(schema_err(
                        format!("versions.{k}"),
                        "expected version string INT.INT",
                    )),
                }
            }
        }
        schema_errs.extend(e);
    }
    if let Some(p) = cert.get("producer") {
        let mut e = Vec::new();
        if check_keys(
            p,
            &["name", "version", "materialisation"],
            &["name", "version", "materialisation"],
            "producer",
            &mut e,
        ) {
            for k in ["name", "version"] {
                nonempty_str(
                    p.get(k).unwrap_or(&Value::Null),
                    &format!("producer.{k}"),
                    &mut e,
                );
            }
            if let Some(m) = p.get("materialisation") {
                if check_keys(
                    m,
                    &["llvm_version", "pipeline", "normaliser_version"],
                    &["llvm_version", "pipeline", "normaliser_version"],
                    "producer.materialisation",
                    &mut e,
                ) {
                    for k in ["llvm_version", "pipeline", "normaliser_version"] {
                        nonempty_str(
                            m.get(k).unwrap_or(&Value::Null),
                            &format!("producer.materialisation.{k}"),
                            &mut e,
                        );
                    }
                }
            }
        }
        schema_errs.extend(e);
    }
    if cert.get("regions").and_then(Value::as_obj).is_none() {
        schema_errs.push(schema_err(
            "regions",
            "expected an object keyed by region closure hash",
        ));
    }
    // Bindings schema.
    let bindings = cert.get("bindings").cloned().unwrap_or(Value::Null);
    {
        let mut e = Vec::new();
        check_keys(
            &bindings,
            &[
                "object_hash",
                "interface_hash",
                "closure_hash",
                "reference_program_hash",
                "dialect_mapping_version",
                "target_hash",
                "schedule_hash",
                "invocation",
            ],
            &[
                "object_hash",
                "interface_hash",
                "closure_hash",
                "reference_program_hash",
            ],
            "bindings",
            &mut e,
        );
        for k in [
            "object_hash",
            "interface_hash",
            "closure_hash",
            "reference_program_hash",
            "target_hash",
            "schedule_hash",
        ] {
            if let Some(v) = bindings.get(k) {
                hash_field(v, &format!("bindings.{k}"), &mut e);
            }
        }
        if let Some(v) = bindings.get("dialect_mapping_version") {
            nonempty_str(v, "bindings.dialect_mapping_version", &mut e);
        }
        schema_errs.extend(e);
    }
    verdict.errors.extend(schema_errs);
    if !verdict.errors.is_empty() {
        return finish(verdict);
    }

    if cert
        .get("versions")
        .and_then(|v| v.get("cert"))
        .and_then(Value::as_str)
        != Some("0.9")
    {
        verdict.unchecked_items.push("certificate schema/replay for this legacy version has not been implemented; only cert 0.9 obligations are currently checked".into());
        return finish(verdict);
    }

    // Root binding cross-checks.
    let bget = |k: &str| {
        bindings
            .get(k)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    if bget("object_hash") != object.object_hash {
        verdict.errors.push(rej(
            "E_CERT_BINDING",
            "§12.3",
            "bindings.object_hash",
            "does not equal the recomputed §9.2 object_hash of the supplied object",
        ));
    }
    let root = object.symbol_by_closure(&bget("closure_hash"));
    match root {
        None => verdict.errors.push(rej(
            "E_CERT_BINDING",
            "§12.3",
            "bindings.closure_hash",
            "root closure_hash is not a symbol of the supplied object",
        )),
        Some(r) => {
            if bget("interface_hash") != r.interface_hash {
                verdict.errors.push(rej(
                    "E_CERT_BINDING",
                    "§12.3",
                    "bindings.interface_hash",
                    "does not equal the root symbol's interface hash",
                ));
            }
            if bget("reference_program_hash") != r.interface_hash {
                verdict.errors.push(rej(
                    "E_CERT_BINDING",
                    "§9.2",
                    "bindings.reference_program_hash",
                    "reference_program_hash is the root symbol's interface hash",
                ));
            }
        }
    }
    if !verdict.errors.is_empty() {
        return finish(verdict);
    }
    let root = root.unwrap().clone();

    if let Some(limit) = eval::source_nesting_limit(&object) {
        verdict.unchecked_items.push(format!(
            "unchecked (host limit): {} (§12.2)",
            limit.quantity
        ));
        return finish(verdict);
    }

    // Target document.
    let mut target_doc: Option<Value> = None;
    if let Some(tb) = &inputs.target_bytes {
        let h = sha256_hex(tb);
        verdict.hashes.insert("target_hash".into(), h.clone());
        if let Some(want) = bindings.get("target_hash").and_then(Value::as_str) {
            if want != h {
                verdict.errors.push(rej(
                    "E_CERT_BINDING",
                    "§8.1",
                    "bindings.target_hash",
                    "does not equal SHA-256 of the supplied target document",
                ));
            }
        }
        target_doc = load_jcs_doc(tb, "target", &mut verdict);
        if !verdict.unchecked_items.is_empty() {
            return finish(verdict);
        }
        if let Some(t) = &target_doc {
            numeric::validate_target_shape(t, &mut verdict.errors);
        }
    }

    // Schedule document: explicit input wins, else the object's section.
    let (schedule_doc, schedule_hash_val): (Option<Value>, Option<String>) =
        match &inputs.schedule_bytes {
            Some(sb) => (
                load_jcs_doc(sb, "schedule", &mut verdict),
                Some(sha256_hex(sb)),
            ),
            None => match (&object.schedule, &object.schedule_raw) {
                (Some(v), Some(raw)) => (Some(v.clone()), Some(sha256_hex(raw.as_bytes()))),
                _ => (None, None),
            },
        };
    if !verdict.unchecked_items.is_empty() {
        return finish(verdict);
    }
    if let Some(schedule) = &schedule_doc {
        // Embedded schedules use the same decimal preflight as detached ones.
        if let Some(limit) = crate::decimal_limits::document(schedule, "schedule") {
            verdict.unchecked_items.push(format!(
                "unchecked (host limit): {} (§12.2)",
                limit.quantity
            ));
            return finish(verdict);
        }
    }
    if let Some(h) = &schedule_hash_val {
        verdict.hashes.insert("schedule_hash".into(), h.clone());
        if let Some(want) = bindings.get("schedule_hash").and_then(Value::as_str) {
            if want != h {
                verdict.errors.push(rej(
                    "E_CERT_BINDING",
                    "§9.2",
                    "bindings.schedule_hash",
                    "does not equal SHA-256 of the supplied schedule document",
                ));
            }
        }
    }
    let (expansion, expansion_unknown) = match &schedule_doc {
        Some(sched) => crate::numeric::check_schedule_bindings(
            sched,
            &object,
            &root,
            &bindings,
            schedule_hash_val.as_deref(),
            &mut verdict.errors,
        ),
        None => (None, false),
    };
    if bindings.get("schedule_hash").is_some() && schedule_doc.is_none() {
        verdict.unchecked_items.push("schedule document was not supplied (and the object embeds none); schedule binding is unchecked".into());
    }

    // Root invocation (required for a scheduled certificate, K13).
    let root_iface = match object.interface_host_limit(&root.name) {
        Ok(Some(limit)) => {
            verdict.unchecked_items.push(format!(
                "unchecked (host limit): {} (§12.2)",
                limit.quantity
            ));
            return finish(verdict);
        }
        Err(error) => Err(error),
        Ok(None) => object.interface(&root.name),
    };
    if let Err(error) = &root_iface {
        verdict
            .errors
            .push(rej("E_CERT_OBJECT", "§9.1", "object", error));
        return finish(verdict);
    }
    let mut root_invocation: Option<Invocation> = None;
    match bindings.get("invocation") {
        Some(inv) => match &root_iface {
            Ok(iface) => match parse_invocation(inv, &iface.args, &object, "bindings.invocation") {
                Ok(parsed) => root_invocation = Some(parsed),
                Err(e) => verdict.errors.extend(e),
            },
            Err(e) => verdict.errors.push(rej(
                "E_CERT_OBJECT",
                "§9.1",
                "bindings.closure_hash",
                e.clone(),
            )),
        },
        None => {
            if bindings.get("schedule_hash").is_some() {
                verdict.errors.push(rej(
                    "E_CERT_BINDING",
                    "§12.3",
                    "bindings.invocation",
                    "a scheduled certificate must carry the root bindings.invocation",
                ));
            }
        }
    }
    // Schedule/invocation agreement.
    if let (Some(sched), Some(inv)) = (&schedule_doc, &root_invocation) {
        if let Some(sb) = sched.get("bindings") {
            if sb.get("params_hash").and_then(Value::as_str) != Some(inv.params_hash.as_str()) {
                verdict.errors.push(rej(
                    "E_CERT_BINDING",
                    "§12.3",
                    "schedule.bindings.params_hash",
                    "does not equal SHA-256 of the JCS root invocation args array",
                ));
            }
            let budget = sb
                .get("budget")
                .and_then(|b| parse_entry_budget(b, &object, "schedule.bindings.budget").ok());
            if budget.as_ref() != Some(&inv.budget) {
                verdict.errors.push(rej(
                    "E_CERT_BINDING",
                    "§12.3",
                    "schedule.bindings.budget",
                    "does not equal the root invocation budget",
                ));
            }
        }
    }
    if !verdict.errors.is_empty() {
        return finish(verdict);
    }

    let docs = LoadedDocs {
        root_closure: bget("closure_hash"),
        root_invocation: root_invocation.clone(),
        object,
        target: target_doc,
        schedule: schedule_doc,
        schedule_hash: schedule_hash_val,
        expansion,
        expansion_unknown,
    };
    let cert_schedule_hash = bindings
        .get("schedule_hash")
        .and_then(Value::as_str)
        .map(str::to_string);
    let mut libraries = Vec::new();
    for (i, bytes) in inputs.libraries.iter().enumerate() {
        if let Some(doc) = load_jcs_doc(bytes, &format!("libraries[{i}]"), &mut verdict) {
            libraries.push(doc);
        }
    }
    if !verdict.errors.is_empty() {
        return finish(verdict);
    }

    // Regions and claims.
    let regions = cert
        .get("regions")
        .and_then(Value::as_obj)
        .cloned()
        .unwrap_or_default();
    if !regions.contains_key(&docs.root_closure) {
        verdict.errors.push(rej(
            "E_CERT_BINDING",
            "§12.3",
            "regions",
            "certificate omits its bound root region record",
        ));
    }
    for (hash, region) in &regions {
        let report = check_region(
            hash,
            region,
            &docs,
            &root_invocation,
            cert_schedule_hash.as_deref(),
            &libraries,
            inputs,
        );
        verdict.regions.push(report);
    }
    verdict
        .regions
        .sort_by(|a, b| a.closure_hash.cmp(&b.closure_hash));

    // Composition (section 12.3) across the checked regions.
    if let Some(comp) = cert.get("composition") {
        numeric::check_composition(comp, &cert, &docs, &verdict.regions, &mut verdict.errors);
        if docs.schedule.is_some() {
            verdict.unchecked_items.push("composition multiplicities and inclusive error recurrence have not been recomputed from the realised invocation tree (§12.3)".into());
        } else if comp
            .get("multiplicities")
            .and_then(Value::as_obj)
            .map(|m| !m.is_empty())
            == Some(true)
        {
            verdict.unchecked_items.push(
                "composition reports multiplicities without a supplied realised schedule (§12.3)"
                    .into(),
            );
        }
    }

    crate::accounting::check(&cert, &docs, inputs, &mut verdict);

    verdict.establishes.push(
        "hash bindings were recomputed from the supplied documents (§9.2 object/symbol identity, SHA-256 of JCS target/schedule bytes, params_hash); producer-stated hashes were never trusted".into(),
    );
    if inputs.target_bytes.is_none() && bindings.get("target_hash").is_some() {
        verdict
            .unchecked_items
            .push("target document was not supplied; its hash binding is unchecked".into());
    }
    if inputs.oracle_bin.is_some() {
        verdict
            .unchecked_items
            .push("optional --oracle-bin differential replay has not been performed".into());
    }
    finish(verdict)
}

fn finish(mut verdict: Verdict) -> Verdict {
    let any_reject = !verdict.errors.is_empty()
        || verdict
            .regions
            .iter()
            .any(|r| !r.errors.is_empty() || r.claims.iter().any(|(_, o)| o.outcome == "reject"));
    let any_unchecked = verdict.regions.iter().any(|r| {
        !r.unchecked_items.is_empty() || r.claims.iter().any(|(_, o)| o.outcome == "unchecked")
    });
    verdict.overall = if any_reject {
        "rejected"
    } else if any_unchecked || !verdict.unchecked_items.is_empty() {
        "unchecked"
    } else {
        "verified"
    }
    .into();
    verdict
}

// ---------- regions and claims ----------

fn check_region(
    hash: &str,
    region: &Value,
    docs: &LoadedDocs,
    root_invocation: &Option<Invocation>,
    cert_schedule_hash: Option<&str>,
    libraries: &[Value],
    inputs: &Inputs,
) -> RegionReport {
    let base = format!("regions.{hash}");
    let mut report = RegionReport {
        closure_hash: hash.to_string(),
        ..Default::default()
    };
    if !is_hash_hex(hash) {
        report.errors.push(schema_err(
            base.clone(),
            "region key is not a 64-lowercase-hex closure hash",
        ));
        return report;
    }
    let mut errs = Vec::new();
    let shape_ok = check_keys(
        region,
        &[
            "kind",
            "signature",
            "bounds",
            "estimate",
            "frame",
            "frame_contexts",
            "resources",
            "resource_contexts",
            "gates_used",
            "claims",
            "verification",
            "cleanup",
            "error",
            "definedness_assumptions",
        ],
        &[
            "kind",
            "signature",
            "bounds",
            "resources",
            "gates_used",
            "claims",
            "verification",
            "cleanup",
            "error",
            "definedness_assumptions",
        ],
        &base,
        &mut errs,
    );
    if !shape_ok {
        report.errors = errs;
        return report;
    }
    if region.get("claims").and_then(Value::as_arr).is_none() {
        report.errors.push(schema_err(
            format!("{base}.claims"),
            "expected an array of claims",
        ));
    }
    let symbol = docs.object.symbol_by_closure(hash).cloned();
    match &symbol {
        None => {
            report.unchecked_items.push(
                "region is not a symbol of the supplied object (possibly a linked library region); its claims were not replayed".into(),
            );
        }
        Some(sym) => {
            report.name = Some(sym.name.clone());
            if region.get("kind").and_then(Value::as_str) != Some(sym.kind.as_str()) {
                report.errors.push(rej(
                    "E_CERT_BINDING",
                    "§12.3",
                    format!("{base}.kind"),
                    "does not equal the object symbol kind",
                ));
            }
            let sig_ok = region
                .get("signature")
                .map(|s| jcs(s).unwrap_or_default() == jcs(&sym.signature).unwrap_or_default())
                .unwrap_or(false);
            if !sig_ok {
                report.errors.push(rej(
                    "E_CERT_BINDING",
                    "§12.3",
                    format!("{base}.signature"),
                    "does not equal the object symbol's signature",
                ));
            }
            if let Some(b) = region.get("bounds") {
                let want = sym
                    .signature
                    .get("bounds")
                    .cloned()
                    .unwrap_or(Value::Arr(vec![]));
                if jcs(b).unwrap_or_default() != jcs(&want).unwrap_or_default() {
                    report.errors.push(rej("E_CERT_BINDING", "§12.3", format!("{base}.bounds"), "does not equal the signature bounds (§5.3 padded unitary is part of identity)"));
                }
            }
        }
    }
    crate::frames::check(region, symbol.as_ref(), docs, inputs, &base, &mut report);
    // definedness_assumptions: array (of dialect-spec ids; strings).
    if let Some(v) = region.get("definedness_assumptions") {
        if v.as_arr().map(|a| {
            a.iter()
                .all(|x| x.as_str().map(|t| !t.is_empty()).unwrap_or(false))
        }) != Some(true)
        {
            report.errors.push(schema_err(
                format!("{base}.definedness_assumptions"),
                "expected array of nonempty id strings",
            ));
        }
    }
    // resources / gates_used shapes.
    numeric::check_region_reports(region, docs, &base, &mut report);
    // cleanup entries shape (paths checked with the schedule machinery).
    numeric::check_cleanup_entries(region, docs, &base, &mut report);
    // error map shape.
    numeric::check_error_map(region, &base, &mut report.errors);
    if region
        .get("error")
        .and_then(Value::as_obj)
        .map(|m| !m.is_empty())
        == Some(true)
    {
        report.unchecked_items.push("local/inclusive error entries have not been tied to checked common context bounds and the callee recurrence (§12.3)".into());
    }
    if let Some(sym) = &symbol {
        if let Ok(iface) = docs.object.interface(&sym.name) {
            if !iface.borrowed_widths.is_empty() {
                report.unchecked_items.push(BORROW_PROOF_PENDING.into());
                if !report
                    .unchecked_items
                    .iter()
                    .any(|s| s == FRAME_STORAGE_PENDING)
                {
                    report.unchecked_items.push(FRAME_STORAGE_PENDING.into());
                }
            }
        }
        let mut seen = std::collections::HashSet::new();
        let mut visit = vec![sym.name.clone()];
        while let Some(name) = visit.pop() {
            if !seen.insert(name.clone()) {
                continue;
            }
            if let Some(function) = docs.object.function(&name) {
                for tokens in eval::find_opcodes(function) {
                    if tokens.iter().any(|t| t == "qinplace") {
                        report.unchecked_items.push("qinplace uses require covering inverse-pair entries at every resolved call's budget, arguments and token tuples; entry applicability has not been checked (§4.5)".into());
                    }
                    for token in tokens {
                        if let Some(name) = token.strip_prefix('@') {
                            if docs.object.symbol(name).is_some() {
                                visit.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    // estimate (K12): exact recomputation needs the bound realisation.
    if region.get("estimate").is_some() {
        report.unchecked_items.push(
            "estimate exact_resources (§10.1) not recomputed: rebuilding the bound realisation requires the §11.1 expansion, which this checker does not yet re-derive".into(),
        );
    }

    let claims = region
        .get("claims")
        .and_then(Value::as_arr)
        .map(|a| a.to_vec())
        .unwrap_or_default();
    // Non-structural claims first (index order preserved in the report);
    // structural claims second so every subclaim outcome is available.
    let mut outcomes: Vec<(String, ClaimOutcome)> = claims
        .iter()
        .map(|c| {
            (
                c.get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                ClaimOutcome::unchecked("not yet evaluated"),
            )
        })
        .collect();
    for pass_structural in [false, true] {
        for (i, claim) in claims.iter().enumerate() {
            let kind = claim.get("kind").and_then(Value::as_str).unwrap_or("");
            if (kind == "structural") != pass_structural {
                continue;
            }
            let outcome = check_claim(
                claim,
                i,
                &format!("{base}.claims[{i}]"),
                hash,
                symbol.as_ref(),
                region,
                docs,
                root_invocation,
                cert_schedule_hash,
                libraries,
                inputs,
                &claims,
                &outcomes,
            );
            outcomes[i].1 = outcome;
        }
    }
    report.claims = outcomes;
    if let Some(coverage) = &docs.object.coverage {
        let _ = coverage;
        report.unchecked_items.push(
            "object coverage/source-classification obligations have not been checked (§13/§15)"
                .into(),
        );
    }

    // Verification index cross-checks (section 12.3).
    if let Some(ver) = region.get("verification") {
        let mut e = Vec::new();
        check_keys(
            ver,
            &[
                "oracle",
                "borrow_frames",
                "inverse_pairs",
                "unitary",
                "channel",
                "fault_rate",
            ],
            &[
                "oracle",
                "borrow_frames",
                "inverse_pairs",
                "unitary",
                "channel",
                "fault_rate",
            ],
            &format!("{base}.verification"),
            &mut e,
        );
        report.errors.extend(e);
        let idx = |v: &Value| -> Option<usize> { u64_of(v).map(|i| i as usize) };
        let kind_at = |i: usize| {
            claims
                .get(i)
                .and_then(|c| c.get("kind"))
                .and_then(Value::as_str)
        };
        let check_index =
            |report: &mut RegionReport, field: &str, i: usize, want: &[&str]| match kind_at(i) {
                None => report.errors.push(rej(
                    "E_CERT_VERIFICATION_INDEX",
                    "§12.3",
                    format!("{base}.verification.{field}"),
                    format!("claim index {i} out of range"),
                )),
                Some(k) if !want.contains(&k) => report.errors.push(rej(
                    "E_CERT_VERIFICATION_INDEX",
                    "§12.3",
                    format!("{base}.verification.{field}"),
                    format!("claim {i} has kind {k}, expected {}", want.join("/")),
                )),
                _ => {}
            };
        match ver.get("oracle") {
            Some(Value::Null) => {}
            Some(v) => match idx(v) {
                Some(i) => check_index(&mut report, "oracle", i, &["exhaustive"]),
                None => report.errors.push(schema_err(
                    format!("{base}.verification.oracle"),
                    "expected claim index or null",
                )),
            },
            None => {}
        }
        for (field, want) in [
            ("unitary", &["numerical"][..]),
            ("channel", &["channel"][..]),
            ("borrow_frames", &["exhaustive", "structural"][..]),
        ] {
            if let Some(Value::Arr(list)) = ver.get(field) {
                for v in list {
                    match idx(v) {
                        Some(i) => check_index(&mut report, field, i, want),
                        None => report.errors.push(schema_err(
                            format!("{base}.verification.{field}"),
                            "expected claim indices",
                        )),
                    }
                }
            } else {
                report.errors.push(schema_err(
                    format!("{base}.verification.{field}"),
                    "expected an array of claim indices",
                ));
            }
        }
        // fault_rate domain validity (semantic recount: see docs/checker.md).
        if let Some(fr) = ver.get("fault_rate") {
            let mut e = Vec::new();
            if check_keys(
                fr,
                &[
                    "domain",
                    "faults_by_code",
                    "undefined_excluded",
                    "layout_dependent_excluded",
                ],
                &[
                    "domain",
                    "faults_by_code",
                    "undefined_excluded",
                    "layout_dependent_excluded",
                ],
                &format!("{base}.verification.fault_rate"),
                &mut e,
            ) {
                match fr.get("faults_by_code").and_then(Value::as_obj) {
                    Some(counts) => {
                        for (code, count) in counts {
                            if !is_canonical_uint_str(code) || u64_of(count).is_none() {
                                e.push(schema_err(
                                    format!("{base}.verification.fault_rate.faults_by_code.{code}"),
                                    "expected a nonnegative fault-code count",
                                ));
                            }
                        }
                    }
                    None => e.push(schema_err(
                        format!("{base}.verification.fault_rate.faults_by_code"),
                        "expected an object of fault-code counts",
                    )),
                }
                for k in ["undefined_excluded", "layout_dependent_excluded"] {
                    if fr.get(k).and_then(u64_of).is_none() {
                        e.push(schema_err(
                            format!("{base}.verification.fault_rate.{k}"),
                            "expected a nonnegative integer count",
                        ));
                    }
                }
                if let Some(sym) = &symbol {
                    if let (Some(d), Ok(iface)) =
                        (fr.get("domain"), docs.object.interface(&sym.name))
                    {
                        if let Err(err) = resolve_domain(d, &iface, 0) {
                            match err {
                                ResolveError::Reject(r) => report
                                    .errors
                                    .push(r.at(&format!("{base}.verification.fault_rate.domain"))),
                                ResolveError::HostLimit(h) => report.unchecked_items.push(format!(
                                    "fault_rate domain: host limit ({})",
                                    h.quantity
                                )),
                            }
                        }
                    }
                }
                if symbol.as_ref().map(|x| x.kind.as_str()) == Some("oracle") {
                    report.unchecked_items.push(
                        "fault_rate counts not recounted: §12 does not state the budget/arguments a fault-rate recount uses".into(),
                    );
                }
            }
            report.errors.extend(e);
        }
        // inverse_pairs entries: structure + claim linkage; pair replay runs
        // through the linked claim itself.
        if let Some(Value::Arr(pairs)) = ver.get("inverse_pairs") {
            for (pi, pair) in pairs.iter().enumerate() {
                let pbase = format!("{base}.verification.inverse_pairs[{pi}]");
                let mut e = Vec::new();
                if check_keys(
                    pair,
                    &["g", "ginv", "claim", "budget", "arg_tokens"],
                    &["g", "ginv", "claim", "budget"],
                    &pbase,
                    &mut e,
                ) {
                    for k in ["g", "ginv"] {
                        match pair.get(k).and_then(Value::as_str) {
                            Some(h) if is_hash_hex(h) => {
                                if docs.object.symbol_by_closure(h).is_none() {
                                    report.errors.push(rej(
                                        "E_CERT_BINDING",
                                        "§12.3",
                                        format!("{pbase}.{k}"),
                                        "names no symbol of the supplied object",
                                    ));
                                }
                            }
                            _ => e.push(schema_err(
                                format!("{pbase}.{k}"),
                                "expected closure hash of an oracle",
                            )),
                        }
                    }
                    match pair.get("claim").and_then(|v| idx(v)) {
                        Some(i) => {
                            let ok = claims.get(i).map(|c| {
                                let k = c.get("kind").and_then(Value::as_str).unwrap_or("");
                                (k == "exhaustive")
                                    || (k == "structural"
                                        && c.get("rule").and_then(Value::as_str)
                                            == Some("inverse_pair_library"))
                            });
                            if ok != Some(true) {
                                report.errors.push(rej("E_CERT_VERIFICATION_INDEX", "§4.5", format!("{pbase}.claim"), "must name an exhaustive inverse_pair claim or an inverse_pair_library structural claim; a sampled claim never covers"));
                            }
                        }
                        None => {
                            e.push(schema_err(format!("{pbase}.claim"), "expected claim index"))
                        }
                    }
                }
                report.errors.extend(e);
            }
        } else {
            report.errors.push(schema_err(
                format!("{base}.verification.inverse_pairs"),
                "expected an array of inverse-pair records",
            ));
        }
    }
    crate::structural::discharge_borrow_frames(region, docs, &mut report);
    report
}

#[allow(clippy::too_many_arguments)]
fn check_claim(
    claim: &Value,
    claim_index: usize,
    path: &str,
    region_hash: &str,
    region_symbol: Option<&crate::object::Symbol>,
    region: &Value,
    docs: &LoadedDocs,
    root_invocation: &Option<Invocation>,
    cert_schedule_hash: Option<&str>,
    libraries: &[Value],
    inputs: &Inputs,
    all_claims: &[Value],
    prior: &[(String, ClaimOutcome)],
) -> ClaimOutcome {
    let kind = claim.get("kind").and_then(Value::as_str).unwrap_or("");
    match kind {
        "exhaustive" | "sampled" => eval::check_replay_claim(
            claim,
            claim_index,
            path,
            region_hash,
            region_symbol,
            region,
            docs,
            root_invocation,
            cert_schedule_hash,
            inputs,
        ),
        "numerical" => numeric::check_numerical_claim(claim, path, region_hash, docs, cert_schedule_hash, inputs),
        "channel" => numeric::check_channel_claim(claim, path, region_hash, docs, cert_schedule_hash, inputs),
        "structural" => numeric::check_structural_claim(
            claim,
            path,
            region_hash,
            region_symbol,
            region,
            docs,
            root_invocation,
            libraries,
            all_claims,
            prior,
            inputs,
        ),
        other => ClaimOutcome::rejected(vec![schema_err(
            format!("{path}.kind"),
            format!("unknown claim kind {other:?}; cert 0.9 defines exhaustive, sampled, structural, numerical, channel (minimality claims must not appear, §1.3)"),
        )]),
    }
}

/// Resolve + validate the domain/expected/subject/invocation shell shared by
/// replay claims. Returns Err(outcome) when the claim is already decided.
pub struct ReplayShell {
    pub subject: Subject,
    pub subject_symbol: crate::object::Symbol,
    pub domain: ResolvedDomain,
    pub invocation: Invocation,
    pub requested_label: Option<String>,
}

pub fn claim_subject(
    claim: &Value,
    path: &str,
    region_hash: &str,
    require_schedule: bool,
) -> Result<Subject, ClaimOutcome> {
    let Some(subj) = claim.get("subject") else {
        return Err(ClaimOutcome::rejected(vec![rej(
            "E_CERT_BINDING",
            "§12.1",
            format!("{path}.subject"),
            "claim must name its subject (no implicit self-comparison)",
        )]));
    };
    let subject = parse_subject(subj, &format!("{path}.subject"), require_schedule)
        .map_err(ClaimOutcome::rejected)?;
    let _ = region_hash;
    Ok(subject)
}

pub fn decimal_of(v: &Value) -> Option<Rat> {
    crate::rat::rat_of_json(v).ok()
}

pub fn host_limit_outcome(h: HostLimit) -> ClaimOutcome {
    let mut o = ClaimOutcome::unchecked(format!("unchecked (host limit): {} (§12.2)", h.quantity));
    o.establishes =
        "nothing: the claim is legal but exceeds this host; no label is assigned".into();
    o
}
