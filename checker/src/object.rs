//! Bennett object text (sections 9.1, 16) parser and section 9.2 identity
//! (semantic envelopes, SCC hashes, object_hash), implemented from the spec.
//! Only what certificate checking needs is validated; every rejection names
//! its spec section. Full producer well-formedness (suite F) is not re-run.

use crate::json::{jcs, parse, Value};
use crate::sha256::sha256_hex;
mod admission;
mod lexical;
mod signatures;
use std::collections::{BTreeMap, HashMap, HashSet};

const SECTION_ORDER: [&str; 9] = [
    "symbols",
    "imports",
    "functions",
    "data",
    "devices",
    "layout",
    "schedule",
    "certificate",
    "coverage",
];

#[derive(Debug, Clone)]
pub struct Header {
    pub versions: BTreeMap<String, String>, // isa, object, launch, sched, cert, coverage, target
    pub ptr_width: u64,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: String,
    pub suffix: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub name: String,
    pub params: Vec<Param>,
    /// Instruction lines in text order (block header and structured closing braces excluded).
    pub instructions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    /// "oracle" | "action" | "perm_action" | "lproc" | "cc" | "r"
    pub kind: String,
    pub groups: Vec<Vec<Param>>,
    /// Canonical function bytes: header line through last instruction line,
    /// LF-joined, no trailing LF.
    pub bytes: String,
    pub blocks: Vec<Block>,
    /// The `bounds { ... }` list of the regs line, if present.
    pub bounds: Vec<u64>,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: String,
    pub signature: Value,
    pub stated_interface_hash: String,
    pub interface_hash: String,
    /// None when the closure depends on an unresolved import.
    pub closure_hash: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BundleInfo {
    pub name: String,
    pub group: String,
    pub slot: usize,
    pub width: u64,
}

#[derive(Debug, Clone)]
pub struct RegionInfo {
    pub name: String,
    pub group: String,
    pub slot: usize,
    pub a: u64,
    pub w: u64,
    pub role: String,
}

#[derive(Debug, Clone)]
pub struct Interface {
    /// K13 tuple order: oracle = in.., tgt, fault; action = mut.., in...
    pub bundles: Vec<BundleInfo>,
    pub regions: Vec<RegionInfo>,
    /// Classical argument types in signature order ("i8", "ptr", "f64", "token", ...).
    pub args: Vec<String>,
    /// Widths of `qborrow` lifetimes in the body, text order (§12.2 borrowed k).
    pub borrowed_widths: Vec<u64>,
    pub bounds: Vec<u64>,
}

#[derive(Debug)]
pub struct Object {
    pub header: Header,
    pub symbols: Vec<Symbol>,
    pub functions: Vec<Function>,
    /// data name -> HEX (possibly empty)
    pub data: BTreeMap<String, String>,
    pub layout: Option<Value>,
    pub devices: Option<Value>,
    pub imports: BTreeMap<String, Value>,
    pub schedule: Option<Value>,
    pub schedule_raw: Option<String>,
    pub certificate: Option<Value>,
    pub certificate_raw: Option<String>,
    pub coverage: Option<Value>,
    pub coverage_raw: Option<String>,
    pub object_hash: String,
    /// §3.6: SCC order key, then member index. Kept from the closure hash pass
    /// because a cyclic member's hash does not reveal its SCC's ordering key.
    closure_order: HashMap<String, (String, usize)>,
}

fn sec_err(section: &str, msg: impl Into<String>) -> String {
    format!("{section}: {}", msg.into())
}

impl Object {
    pub fn parse(text: &str) -> Result<Object, String> {
        if !text.ends_with('\n') {
            return Err(sec_err("§16.1", "object text does not end with LF"));
        }
        let lines: Vec<&str> = text.split('\n').collect(); // last element is ""
        lexical::validate(&lines)?;
        let header = parse_header(lines.first().copied().unwrap_or(""))?;

        // Split into sections, enforcing order, uniqueness, blank-line rule.
        let mut sections: Vec<(&str, Vec<String>)> = Vec::new();
        let mut i = 1usize;
        let mut last_order: Option<usize> = None;
        while i < lines.len() - 1 {
            if lines[i].is_empty()
                && lines
                    .get(i + 1)
                    .map(|l| l.starts_with("section "))
                    .unwrap_or(false)
            {
                let name = &lines[i + 1]["section ".len()..];
                let order = SECTION_ORDER
                    .iter()
                    .position(|s| *s == name)
                    .ok_or_else(|| sec_err("§9.1", format!("unknown section {name:?}")))?;
                if last_order.map(|lo| order <= lo).unwrap_or(false) {
                    return Err(sec_err(
                        "§9.1",
                        format!("section {name:?} out of order or repeated"),
                    ));
                }
                last_order = Some(order);
                i += 2;
                let mut body = Vec::new();
                while i < lines.len() - 1
                    && !(lines[i].is_empty()
                        && lines
                            .get(i + 1)
                            .map(|l| l.starts_with("section "))
                            .unwrap_or(false))
                {
                    body.push(lines[i].to_string());
                    i += 1;
                }
                // A section body has no blank lines (§16.1).
                if body.iter().any(|l| l.is_empty()) {
                    return Err(sec_err(
                        "§16.1",
                        format!("blank line inside section {name:?}"),
                    ));
                }
                sections.push((SECTION_ORDER[order], body));
            } else if lines[i].is_empty() && i == lines.len() - 1 {
                i += 1;
            } else {
                return Err(sec_err(
                    "§16.1",
                    format!("expected blank line + section header at line {}", i + 1),
                ));
            }
        }
        let sec = |name: &str| sections.iter().find(|(n, _)| *n == name).map(|(_, b)| b);

        let symbols_json = parse_metadata(sec("symbols"), "symbols")?
            .ok_or_else(|| sec_err("§9.1", "missing symbols section"))?;
        let mut raw_symbols = Vec::new();
        let sym_obj = symbols_json
            .as_obj()
            .ok_or_else(|| sec_err("§9.1", "symbols section is not an object"))?;
        for (idx, entry) in sym_obj {
            let name = entry
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let kind = entry
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let signature = entry.get("signature").cloned().unwrap_or(Value::Null);
            let stated = entry
                .get("interface_hash")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let index: i128 = idx
                .parse()
                .map_err(|_| sec_err("§9.1", format!("bad symbol index {idx:?}")))?;
            raw_symbols.push((
                index,
                RawSymbol {
                    name,
                    kind,
                    signature,
                    stated,
                },
            ));
        }
        raw_symbols.sort_by_key(|(i, _)| *i);

        let mut imports = BTreeMap::new();
        if let Some(v) = parse_metadata(sec("imports"), "imports")? {
            let obj = v
                .as_obj()
                .ok_or_else(|| sec_err("§9.1", "imports is not an object"))?;
            for (name, entry) in obj {
                imports.insert(
                    name.clone(),
                    entry.get("signature").cloned().unwrap_or(Value::Null),
                );
            }
        }
        let devices = parse_metadata(sec("devices"), "devices")?;
        let layout = parse_metadata(sec("layout"), "layout")?;
        let schedule = parse_metadata(sec("schedule"), "schedule")?;
        let certificate = parse_metadata(sec("certificate"), "certificate")?;
        let coverage = parse_metadata(sec("coverage"), "coverage")?;
        let raw_line = |name: &str| sec(name).and_then(|b| b.first()).cloned();

        let mut data = BTreeMap::new();
        if let Some(body) = sec("data") {
            for line in body {
                let (name, hex) = line
                    .split_once(' ')
                    .ok_or_else(|| sec_err("§16.2", format!("bad data line {line:?}")))?;
                if hex.len() % 2 != 0
                    || !hex
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                {
                    return Err(sec_err("§16.1", format!("bad HEX in data line {line:?}")));
                }
                data.insert(name.to_string(), hex.to_string());
            }
        }
        let functions = match sec("functions") {
            Some(body) => parse_functions(body)?,
            None => Vec::new(),
        };

        let object_hash = sha256_hex(stripped_object_bytes(text).as_bytes());

        let mut obj = Object {
            header,
            symbols: raw_symbols
                .into_iter()
                .map(|(_, r)| Symbol {
                    name: r.name,
                    kind: r.kind,
                    signature: r.signature,
                    stated_interface_hash: r.stated,
                    interface_hash: String::new(),
                    closure_hash: None,
                })
                .collect(),
            functions,
            data,
            layout,
            devices,
            imports,
            schedule,
            schedule_raw: raw_line("schedule"),
            certificate,
            certificate_raw: raw_line("certificate"),
            coverage,
            coverage_raw: raw_line("coverage"),
            object_hash,
            closure_order: HashMap::new(),
        };
        admission::validate(&obj)?;
        obj.compute_hashes()?;
        Ok(obj)
    }

    pub fn stated_hashes_ok(&self) -> bool {
        self.symbols
            .iter()
            .all(|s| s.stated_interface_hash == s.interface_hash)
    }
    pub fn symbol(&self, name: &str) -> Option<&Symbol> {
        self.symbols.iter().find(|s| s.name == name)
    }
    pub fn symbol_by_closure(&self, hash: &str) -> Option<&Symbol> {
        self.symbols
            .iter()
            .find(|s| s.closure_hash.as_deref() == Some(hash))
    }
    pub fn function(&self, name: &str) -> Option<&Function> {
        self.functions.iter().find(|f| f.name == name)
    }
    pub fn instruction_count(&self, func: &str, block: &str) -> Option<usize> {
        self.function(func)?
            .blocks
            .iter()
            .find(|b| b.name == block)
            .map(|b| b.instructions.len())
    }
}

struct RawSymbol {
    name: String,
    kind: String,
    signature: Value,
    stated: String,
}

fn parse_header(line: &str) -> Result<Header, String> {
    let parts: Vec<&str> = line.split(' ').collect();
    let keys = [
        "isa=",
        "object=",
        "launch=",
        "sched=",
        "cert=",
        "coverage=",
        "target=",
        "ptr_width=",
    ];
    if parts.len() != 9 || parts[0] != "bennett-object" {
        return Err(sec_err("§16.2", format!("bad object header {line:?}")));
    }
    let mut versions = BTreeMap::new();
    for (kv, key) in parts[1..8].iter().zip(keys[..7].iter()) {
        let v = kv
            .strip_prefix(key)
            .ok_or_else(|| sec_err("§16.2", format!("bad object header field {kv:?}")))?;
        if !is_version(v) {
            return Err(sec_err("§16.2", format!("bad version {v:?} in header")));
        }
        versions.insert(key.trim_end_matches('=').to_string(), v.to_string());
    }
    let pw = parts[8]
        .strip_prefix("ptr_width=")
        .ok_or_else(|| sec_err("§16.2", "header lacks ptr_width"))?;
    let ptr_width: u64 = pw
        .parse()
        .map_err(|_| sec_err("§16.2", format!("bad ptr_width {pw:?}")))?;
    Ok(Header {
        versions,
        ptr_width,
    })
}

pub fn is_version(v: &str) -> bool {
    match v.split_once('.') {
        Some((a, b)) => {
            !a.is_empty()
                && !b.is_empty()
                && a.bytes().all(|c| c.is_ascii_digit())
                && b.bytes().all(|c| c.is_ascii_digit())
        }
        None => false,
    }
}

fn parse_metadata(body: Option<&Vec<String>>, name: &str) -> Result<Option<Value>, String> {
    match body {
        None => Ok(None),
        Some(body) => {
            if body.len() != 1 {
                return Err(sec_err(
                    "§9.1",
                    format!("{name} section must be one JCS line"),
                ));
            }
            let v = parse(&body[0]).map_err(|e| sec_err("§9.1", format!("{name}: {e}")))?;
            let canonical =
                jcs(&v).map_err(|e| sec_err("§9.1", format!("{name}: cannot encode JCS: {e}")))?;
            if canonical != body[0].as_bytes() {
                return Err(sec_err(
                    "§9.1",
                    format!("{name} metadata bytes are not RFC 8785 canonical"),
                ));
            }
            if let Some(path) = crate::json::oversized_metadata_number(&v, name) {
                return Err(sec_err("§9.1",format!("{path}: JSON metadata numbers >= 2^53 are forbidden; use exact decimal strings")));
            }
            Ok(Some(v))
        }
    }
}

/// object_hash input: canonical text with schedule/certificate/coverage
/// sections deleted together with the blank line preceding each (§9.2).
pub fn stripped_object_bytes(text: &str) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut out: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].is_empty()
            && lines
                .get(i + 1)
                .map(|l| {
                    l == &"section schedule"
                        || l == &"section certificate"
                        || l == &"section coverage"
                })
                .unwrap_or(false)
        {
            i += 2;
            while i < lines.len()
                && !(lines[i].is_empty()
                    && lines
                        .get(i + 1)
                        .map(|l| l.starts_with("section "))
                        .unwrap_or(false))
                && !(i == lines.len() - 1 && lines[i].is_empty())
            {
                i += 1;
            }
        } else {
            out.push(lines[i]);
            i += 1;
        }
    }
    let mut joined = out.join("\n");
    if !joined.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

fn function_kind(line: &str) -> Option<(&'static str, usize)> {
    // Returns (kind, prefix length) for a function header line.
    for (prefix, kind) in [
        ("fn cc @", "cc"),
        ("fn r @", "r"),
        ("oracle @", "oracle"),
        ("action @", "action"),
        ("perm_action @", "perm_action"),
        ("lproc @", "lproc"),
    ] {
        if line.starts_with(prefix) {
            return Some((kind, prefix.len()));
        }
    }
    None
}

fn parse_functions(body: &[String]) -> Result<Vec<Function>, String> {
    let mut out = Vec::new();
    let mut cur: Option<(String, String, Vec<String>)> = None;
    let flush = |cur: &mut Option<(String, String, Vec<String>)>,
                 out: &mut Vec<Function>|
     -> Result<(), String> {
        if let Some((name, kind, lines)) = cur.take() {
            out.push(build_function(name, kind, lines)?);
        }
        Ok(())
    };
    for line in body {
        if let Some((kind, off)) = function_kind(line) {
            flush(&mut cur, &mut out)?;
            let rest = &line[off..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.')
                .collect();
            if name.is_empty() {
                return Err(sec_err(
                    "§16.2",
                    format!("function header without @NAME: {line:?}"),
                ));
            }
            cur = Some((name, kind.to_string(), vec![line.clone()]));
        } else {
            match cur.as_mut() {
                Some((_, _, lines)) => lines.push(line.clone()),
                None => {
                    return Err(sec_err(
                        "§16.2",
                        format!("functions section line outside any function: {line:?}"),
                    ))
                }
            }
        }
    }
    flush(&mut cur, &mut out)?;
    Ok(out)
}

fn build_function(name: String, kind: String, lines: Vec<String>) -> Result<Function, String> {
    let bytes = lines.join("\n");
    let header = &lines[0];
    let groups = parse_header_groups(header, &name)?;
    let mut bounds = Vec::new();
    let mut blocks: Vec<Block> = Vec::new();
    for line in &lines[1..] {
        if let Some(rest) = line.strip_prefix('^') {
            let (bname, plist) = rest
                .split_once('(')
                .ok_or_else(|| sec_err("§16.2", format!("bad block header {line:?}")))?;
            let inner = plist
                .strip_suffix("):")
                .ok_or_else(|| sec_err("§16.2", "block header must consume its complete line"))?;
            blocks.push(Block {
                name: bname.to_string(),
                params: parse_plist(inner)?,
                instructions: Vec::new(),
            });
        } else if line.starts_with("regs ") || line.starts_with("regs{") {
            if let Some(pos) = line.find("bounds") {
                let brace = &line[pos..];
                if let (Some(a), Some(b)) = (brace.find('{'), brace.rfind('}')) {
                    for tok in brace[a + 1..b].split(',') {
                        let tok = tok.trim();
                        if !tok.is_empty() {
                            bounds.push(tok.parse().map_err(|_| {
                                sec_err("§16.2", format!("bad bounds value {tok:?}"))
                            })?);
                        }
                    }
                }
            }
        } else if line != "}" && line != "} {" {
            match blocks.last_mut() {
                Some(b) => b.instructions.push(line.clone()),
                None => {
                    return Err(sec_err(
                        "§16.2",
                        format!("instruction before any block in @{name}: {line:?}"),
                    ))
                }
            }
        }
        // Structured closing braces are not instructions and have no ordinal.
    }
    Ok(Function {
        name,
        kind,
        groups,
        bytes,
        blocks,
        bounds,
    })
}

/// Split the five parenthesised groups of a function header.
fn parse_header_groups(header: &str, fname: &str) -> Result<Vec<Vec<Param>>, String> {
    let tokens = crate::eval::tokenise(header);
    let offset = if tokens.first().map(String::as_str) == Some("fn") {
        3
    } else {
        2
    };
    if tokens.len() != offset + 5
        || tokens.get(offset - 1).map(String::as_str) != Some(&format!("@{fname}"))
    {
        return Err(sec_err(
            "§16.2",
            "function header must consume exactly five parameter groups",
        ));
    }
    tokens[offset..]
        .iter()
        .map(|group| {
            let inner = group
                .strip_prefix('(')
                .and_then(|g| g.strip_suffix(')'))
                .ok_or_else(|| sec_err("§16.2", "expected parenthesised parameter group"))?;
            parse_plist(inner)
        })
        .collect()
}

fn parse_plist(text: &str) -> Result<Vec<Param>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for ch in text.chars() {
        match ch {
            '[' | '(' => {
                depth += 1;
                cur.push(ch);
            }
            ']' | ')' => {
                depth -= 1;
                cur.push(ch);
            }
            ',' if depth == 0 => {
                out.push(parse_param(cur.trim())?);
                cur = String::new();
            }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() {
        out.push(parse_param(cur.trim())?);
    }
    Ok(out)
}

fn parse_param(text: &str) -> Result<Param, String> {
    // NAME ":" type (SP suffix)?   -- the separator before the type is ":"
    // but canonical text writes "n:reg[8]" / "x:wire[8] in".
    let mut parts = text.splitn(2, ':');
    let name = parts.next().unwrap_or("").trim().to_string();
    let rest = parts
        .next()
        .ok_or_else(|| sec_err("§16.2", format!("parameter without type: {text:?}")))?;
    let rest = rest.trim();
    let (ty, suffix) = match rest.rsplit_once(' ') {
        Some((t, s)) if matches!(s, "in" | "mut" | "token" | "classical") => {
            (t.trim().to_string(), Some(s.to_string()))
        }
        _ => (rest.to_string(), None),
    };
    if name.is_empty() || ty.is_empty() {
        return Err(sec_err("§16.2", format!("bad parameter {text:?}")));
    }
    Ok(Param { name, ty, suffix })
}

/// Width of a wire type spelling: wire -> 1, wire[N] -> N.
pub fn wire_width_of(ty: &str) -> Option<u64> {
    if ty == "wire" {
        return Some(1);
    }
    ty.strip_prefix("wire[")?.strip_suffix(']')?.parse().ok()
}

pub(crate) fn region_aw_of(ty: &str) -> Option<(u64, u64)> {
    let inner = ty.strip_prefix("region[")?.strip_suffix(']')?;
    let (a, w) = inner.split_once(',')?;
    let parse = |text: &str| {
        if text.trim() == "-0" {
            Some(0)
        } else {
            text.trim().parse().ok()
        }
    };
    Some((parse(a)?, parse(w)?))
}

fn sig_widths(sig: &Value, key: &str) -> Option<Vec<u64>> {
    sig.get(key)?
        .as_arr()?
        .iter()
        .map(crate::cert::u64_of)
        .collect()
}

impl Object {
    /// Detect exact signature sizes beyond the interface's host integer type,
    /// checking their source spelling before classifying a legal host limit.
    pub(crate) fn interface_host_limit(
        &self,
        name: &str,
    ) -> Result<Option<crate::fault::HostLimit>, String> {
        let Some(symbol) = self.symbol(name) else {
            return Ok(None);
        };
        let Some(function) = self.function(name) else {
            return Ok(None);
        };
        if symbol.kind != function.kind {
            return Err(sec_err(
                "§9.1",
                format!("@{name} symbol kind does not match its function header"),
            ));
        }
        signatures::validate(
            symbol,
            function,
            self.header
                .versions
                .get("isa")
                .map(String::as_str)
                .unwrap_or(""),
        )?;
        let mut limit = None;
        let oversized = |value: &Value| {
            value
                .as_str()
                .map(|s| crate::rat::is_canonical_uint_str(s) && s.parse::<u64>().is_err())
                .unwrap_or(false)
        };
        let mut check = |value: Option<&Value>,
                         source: Option<&str>,
                         field: &str|
         -> Result<(), String> {
            let Some(text) = value
                .and_then(Value::as_str)
                .filter(|s| crate::rat::is_canonical_uint_str(s) && s.parse::<u64>().is_err())
            else {
                return Ok(());
            };
            if source.map(str::trim) != Some(text) {
                return Err(sec_err(
                    "§9.1",
                    format!("@{name} {field}: signature size differs from the source declaration"),
                ));
            }
            limit = Some(crate::fault::HostLimit {
                quantity: format!(
                    "@{name} {field} integer size exceeds the u64 interface host representation"
                ),
            });
            Ok(())
        };
        let raw_wire = |ty: &str| {
            ty.strip_prefix("wire[")
                .and_then(|s| s.strip_suffix(']'))
                .map(str::to_string)
        };
        let groups: &[(&str, usize)] = match symbol.kind.as_str() {
            "oracle" => &[("in", 0)],
            "action" | "perm_action" => &[("mut", 0), ("in", 1)],
            _ => &[],
        };
        for (key, group) in groups {
            if let Some(widths) = symbol.signature.get(key).and_then(Value::as_arr) {
                for (i, value) in widths.iter().enumerate() {
                    if !oversized(value) {
                        continue;
                    }
                    let raw = function.groups[*group].get(i).and_then(|p| raw_wire(&p.ty));
                    check(Some(value), raw.as_deref(), key)?;
                }
            }
        }
        if symbol.kind == "oracle" {
            for (key, group) in [("tgt", 1), ("fault", 2)] {
                let raw = function.groups[group].first().and_then(|p| raw_wire(&p.ty));
                check(symbol.signature.get(key), raw.as_deref(), key)?;
            }
        }
        let group = if symbol.kind == "oracle" { 3 } else { 2 };
        if let Some(regions) = symbol.signature.get("mem").and_then(Value::as_arr) {
            for (i, region) in regions.iter().enumerate() {
                let raw = function
                    .groups
                    .get(group)
                    .and_then(|g| g.get(i))
                    .and_then(|p| p.ty.strip_prefix("region["))
                    .and_then(|s| s.strip_suffix(']'))
                    .and_then(|s| s.split_once(','));
                check(region.get("A"), raw.map(|p| p.0), "mem.A")?;
                check(region.get("W"), raw.map(|p| p.1), "mem.W")?;
            }
        }
        if let Some(bounds) = symbol.signature.get("bounds").and_then(Value::as_arr) {
            if bounds.iter().any(oversized) {
                let declared = function
                    .bytes
                    .lines()
                    .find_map(|line| {
                        line.split_once("bounds {")
                            .map(|(_, s)| s.trim_end_matches('}').trim())
                    })
                    .unwrap_or("");
                let source_bounds: Vec<_> = declared.split(',').map(str::trim).collect();
                for (i, value) in bounds.iter().enumerate() {
                    check(Some(value), source_bounds.get(i).copied(), "bounds")?;
                }
            }
        }
        Ok(limit)
    }
    /// The section 12 interface of an oracle/action/perm_action symbol:
    /// header names + signature widths, cross-checked.
    pub fn interface(&self, name: &str) -> Result<Interface, String> {
        if let Some(limit) = self.interface_host_limit(name)? {
            return Err(format!("host limit: {} (§12.2)", limit.quantity));
        }
        let sym = self
            .symbol(name)
            .ok_or_else(|| format!("no symbol @{name}"))?;
        let func = self
            .function(name)
            .ok_or_else(|| sec_err("§9.1", format!("symbol @{name} has no function body")))?;
        let sig = &sym.signature;
        let mut bundles = Vec::new();
        let add = |params: &[Param],
                   widths: &[u64],
                   group: &str,
                   bundles: &mut Vec<BundleInfo>|
         -> Result<(), String> {
            if params.len() != widths.len() {
                return Err(sec_err(
                    "§9.1",
                    format!(
                        "@{name} {group}: header lists {} params, signature {} widths",
                        params.len(),
                        widths.len()
                    ),
                ));
            }
            for (slot, (p, w)) in params.iter().zip(widths.iter()).enumerate() {
                let hw = wire_width_of(&p.ty).ok_or_else(|| {
                    sec_err(
                        "§16.2",
                        format!("@{name} param {} has non-wire type {:?}", p.name, p.ty),
                    )
                })?;
                if hw != *w || *w == 0 {
                    return Err(sec_err(
                        "§9.1",
                        format!("@{name} param {} width {hw} != signature {w}", p.name),
                    ));
                }
                bundles.push(BundleInfo {
                    name: p.name.clone(),
                    group: group.to_string(),
                    slot,
                    width: *w,
                });
            }
            Ok(())
        };
        let args: Vec<String> = sig
            .get("args")
            .and_then(Value::as_arr)
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let regions = match sym.kind.as_str() {
            "oracle" => {
                add(
                    &func.groups[0],
                    &sig_widths(sig, "in").unwrap_or_default(),
                    "in",
                    &mut bundles,
                )?;
                let tgt = sig
                    .get("tgt")
                    .and_then(crate::cert::u64_of)
                    .ok_or_else(|| sec_err("§9.1", "invalid or unmaterialisable target width"))?;
                let fault = sig
                    .get("fault")
                    .and_then(crate::cert::u64_of)
                    .ok_or_else(|| sec_err("§9.1", "invalid or unmaterialisable fault width"))?;
                if fault < 4 {
                    return Err(sec_err(
                        "§5.4",
                        "an oracle fault width must be at least four",
                    ));
                }
                add(&func.groups[1], &[tgt], "tgt", &mut bundles)?;
                add(&func.groups[2], &[fault], "fault", &mut bundles)?;
                self.interface_regions(name, func, sig, 3)?
            }
            "action" | "perm_action" => {
                add(
                    &func.groups[0],
                    &sig_widths(sig, "mut").unwrap_or_default(),
                    "mut",
                    &mut bundles,
                )?;
                add(
                    &func.groups[1],
                    &sig_widths(sig, "in").unwrap_or_default(),
                    "in",
                    &mut bundles,
                )?;
                self.interface_regions(name, func, sig, 2)?
            }
            other => {
                return Err(sec_err(
                    "§12",
                    format!("@{name} kind {other} is not a certificate region"),
                ))
            }
        };
        // Borrowed lifetimes: qborrow instructions in text order.
        let mut borrowed_widths = Vec::new();
        for block in &func.blocks {
            for line in &block.instructions {
                if let Some(pos) = line.find("qborrow ") {
                    let toks: Vec<&str> = line[pos..].split(' ').collect();
                    // qborrow cls n
                    if toks.len() >= 3 {
                        if let Ok(n) = toks[2].parse::<u64>() {
                            borrowed_widths.push(n);
                        }
                    }
                }
            }
        }
        let bounds = sig_widths(sig, "bounds").unwrap_or_else(|| func.bounds.clone());
        let _ = args.len();
        Ok(Interface {
            bundles,
            regions,
            args,
            borrowed_widths,
            bounds,
        })
    }

    fn interface_regions(
        &self,
        fname: &str,
        func: &Function,
        sig: &Value,
        group_index: usize,
    ) -> Result<Vec<RegionInfo>, String> {
        let params = &func.groups[group_index];
        let mems: Vec<Value> = sig
            .get("mem")
            .and_then(Value::as_arr)
            .map(|a| a.to_vec())
            .unwrap_or_default();
        if params.len() != mems.len() {
            return Err(sec_err(
                "§9.1",
                format!(
                    "@{fname}: header lists {} regions, signature {}",
                    params.len(),
                    mems.len()
                ),
            ));
        }
        let mut out = Vec::new();
        let mut role_slots: HashMap<String, usize> = HashMap::new();
        for (p, m) in params.iter().zip(mems.iter()) {
            let (a, w) = region_aw_of(&p.ty).ok_or_else(|| {
                sec_err(
                    "§16.2",
                    format!("@{fname} param {} is not a region", p.name),
                )
            })?;
            let role = m
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let sa = m.get("A").and_then(crate::cert::u64_of);
            let sw = m.get("W").and_then(crate::cert::u64_of);
            if sa != Some(a) || sw != Some(w) {
                return Err(sec_err(
                    "§9.1",
                    format!("@{fname} region {} shape mismatch", p.name),
                ));
            }
            let slot = *role_slots.entry(role.clone()).or_insert(0);
            role_slots.insert(role.clone(), slot + 1);
            out.push(RegionInfo {
                name: p.name.clone(),
                group: role.clone(),
                slot,
                a,
                w,
                role,
            });
        }
        Ok(out)
    }
}

// ---------- section 9.2 identity ----------

#[derive(PartialEq, Clone, Copy)]
enum HashMode {
    Interface,
    Closure,
}

struct LayoutLeaf {
    /// Key components from the layout root, e.g. ["globals", "zero"].
    components: Vec<String>,
    value: Value,
    /// Rendered path for this object's ISA version (0.8: dotted; 0.9: JCS array),
    /// also the bytewise sort key of part (5b).
    path_rendered: String,
    /// data symbol names this leaf names as an image
    images: Vec<String>,
}

impl Object {
    fn layout_leaves(&self) -> Result<Vec<LayoutLeaf>, String> {
        let isa = self
            .header
            .versions
            .get("isa")
            .map(String::as_str)
            .unwrap_or("");
        let mut out = Vec::new();
        let Some(layout) = &self.layout else {
            return Ok(out);
        };
        let obj = layout
            .as_obj()
            .ok_or_else(|| sec_err("§9.1", "layout section is not an object"))?;
        for (key, val) in obj {
            match key.as_str() {
                "globals" => {
                    let g = val
                        .as_obj()
                        .ok_or_else(|| sec_err("§9.1", "layout.globals is not an object"))?;
                    for (name, gv) in g {
                        let image = gv
                            .get("image")
                            .and_then(Value::as_str)
                            .unwrap_or(name)
                            .to_string();
                        out.push(make_leaf(
                            vec!["globals".into(), name.clone()],
                            gv.clone(),
                            vec![image],
                            isa,
                        )?);
                    }
                }
                "interface_regions" => {
                    let g = val.as_obj().ok_or_else(|| {
                        sec_err("§9.1", "layout.interface_regions is not an object")
                    })?;
                    for (oracle, regions) in g {
                        let r = regions.as_obj().ok_or_else(|| {
                            sec_err("§9.1", "layout.interface_regions entry is not an object")
                        })?;
                        for (region, rv) in r {
                            out.push(make_leaf(
                                vec!["interface_regions".into(), oracle.clone(), region.clone()],
                                rv.clone(),
                                vec![],
                                isa,
                            )?);
                        }
                    }
                }
                _ => {
                    let images = if key == "block_image" {
                        val.as_str()
                            .map(|s| vec![s.to_string()])
                            .unwrap_or_default()
                    } else {
                        vec![]
                    };
                    out.push(make_leaf(vec![key.clone()], val.clone(), images, isa)?);
                }
            }
        }
        Ok(out)
    }

    /// (data entries sorted by name, layout leaves selected) for a symbol kind.
    fn closure_set(
        &self,
        sym: &Symbol,
        func: Option<&Function>,
        leaves: &[LayoutLeaf],
    ) -> Result<(Vec<String>, Vec<LayoutLeaf>), String> {
        let chosen: Vec<&LayoutLeaf> = match sym.kind.as_str() {
            "data" | "action" | "perm_action" => Vec::new(),
            "oracle" => leaves
                .iter()
                .filter(|l| {
                    // all leaves except other oracles' interface_regions entries
                    !(l.components.first().map(String::as_str) == Some("interface_regions")
                        && l.components.get(1).map(String::as_str) != Some(sym.name.as_str()))
                })
                .collect(),
            "lproc" => {
                let mut globals = HashSet::new();
                if let Some(f) = func {
                    for tok in addr_targets(&f.bytes) {
                        globals.insert(tok);
                    }
                }
                leaves
                    .iter()
                    .filter(|l| {
                        l.components.first().map(String::as_str) == Some("globals")
                            && l.components
                                .get(1)
                                .map(|g| globals.contains(g.as_str()))
                                .unwrap_or(false)
                    })
                    .collect()
            }
            "cc" | "r" => leaves.iter().collect(),
            other => {
                return Err(sec_err(
                    "§9.1",
                    format!("unknown symbol kind {other:?} for @{}", sym.name),
                ))
            }
        };
        let mut data_names: Vec<String> = Vec::new();
        for l in &chosen {
            for img in &l.images {
                if !data_names.contains(img) {
                    data_names.push(img.clone());
                }
            }
        }
        data_names.sort();
        let mut chosen: Vec<LayoutLeaf> = chosen.into_iter().cloned().collect();
        chosen.sort_by(|a, b| a.path_rendered.cmp(&b.path_rendered));
        Ok((data_names, chosen))
    }

    fn device_lines(&self, sym: &Symbol, func: Option<&Function>) -> Result<Vec<String>, String> {
        if !matches!(sym.kind.as_str(), "cc" | "r") {
            return Ok(Vec::new());
        }
        let Some(f) = func else { return Ok(Vec::new()) };
        let mut used: Vec<String> = Vec::new();
        for block in &f.blocks {
            for line in &block.instructions {
                let toks: Vec<&str> = line.split(' ').collect();
                if toks.first() == Some(&"effect") && toks.len() >= 2 {
                    used.push(toks[1].split('.').next().unwrap_or(toks[1]).to_string());
                }
                if let Some(pos) = toks.iter().position(|t| *t == "hostcall") {
                    if let Some(svc) = toks.get(pos + 1) {
                        used.push(svc.trim_matches('"').to_string());
                    }
                }
            }
        }
        used.sort();
        used.dedup();
        let mut out = Vec::new();
        for name in used {
            let entry = self
                .devices
                .as_ref()
                .and_then(|d| {
                    d.get("models")
                        .and_then(|m| m.get(&name))
                        .or_else(|| d.get("services").and_then(|m| m.get(&name)))
                })
                .ok_or_else(|| {
                    sec_err(
                        "§9.1",
                        format!("@{} uses unlisted device/service {name:?}", sym.name),
                    )
                })?;
            out.push(format!(
                "device {name} {}",
                String::from_utf8(jcs(entry)?).unwrap()
            ));
        }
        Ok(out)
    }

    fn compute_hashes(&mut self) -> Result<(), String> {
        // Validate definitions exist and collect function bytes per symbol.
        for sym in &self.symbols {
            if sym.kind == "data" {
                if !self.data.contains_key(&sym.name) {
                    return Err(sec_err(
                        "§9.1",
                        format!("data symbol {} has no data line", sym.name),
                    ));
                }
            } else if self.function(&sym.name).is_none() {
                return Err(sec_err(
                    "§9.1",
                    format!("symbol @{} ({}) has no function", sym.name, sym.kind),
                ));
            }
        }
        let (ifaces, _) = self.hash_pass(HashMode::Interface)?;
        let (closures, order) = self.hash_pass(HashMode::Closure)?;
        self.closure_order = order;
        for sym in &mut self.symbols {
            sym.interface_hash = ifaces.get(&sym.name).cloned().unwrap_or_default();
            sym.closure_hash = closures.get(&sym.name).cloned();
        }
        Ok(())
    }

    pub(crate) fn closure_order(&self, name: &str) -> Option<&(String, usize)> {
        self.closure_order.get(name)
    }

    /// One section 9.2 pass, retaining §3.6 placement keys as well as hashes.
    /// The closure pass omits symbols depending on unresolved imports.
    fn hash_pass(
        &self,
        mode: HashMode,
    ) -> Result<(HashMap<String, String>, HashMap<String, (String, usize)>), String> {
        let names: HashSet<String> = self.symbols.iter().map(|s| s.name.clone()).collect();
        let import_hash = |name: &str| -> Option<String> {
            let sig = self.imports.get(name)?;
            let mut bytes = name.as_bytes().to_vec();
            bytes.push(0);
            bytes.extend_from_slice(&jcs(sig).ok()?);
            Some(sha256_hex(&bytes))
        };
        // Reference graph over defined symbols (edges to imports end there).
        let mut refs_of: HashMap<String, Vec<String>> = HashMap::new();
        let mut region_names_of: HashMap<String, HashSet<String>> = HashMap::new();
        for sym in &self.symbols {
            if sym.kind == "oracle" {
                if let Some(f) = self.function(&sym.name) {
                    let regs: HashSet<String> =
                        f.groups[3].iter().map(|p| p.name.clone()).collect();
                    region_names_of.insert(sym.name.clone(), regs);
                }
            }
            let body = self.part4_text(sym);
            let regions = region_names_of.get(&sym.name).cloned().unwrap_or_default();
            let mut refs = Vec::new();
            for tok in ref_tokens(&body, &sym.name) {
                if regions.contains(&tok) {
                    continue; // interface-region spelling, not a symbol reference
                }
                if !names.contains(&tok) && !self.imports.contains_key(&tok) {
                    return Err(sec_err(
                        "§9.2",
                        format!("@{} references undefined @{tok}", sym.name),
                    ));
                }
                if !refs.contains(&tok) {
                    refs.push(tok);
                }
            }
            refs_of.insert(sym.name.clone(), refs);
        }
        // SCCs via Kosaraju over defined-symbol edges.
        let sccs = kosaraju(&names, &refs_of);
        let mut scc_of: HashMap<String, usize> = HashMap::new();
        for (i, scc) in sccs.iter().enumerate() {
            for m in scc {
                scc_of.insert(m.clone(), i);
            }
        }
        // Process SCCs in dependency order (Kahn over the condensation).
        let mut result: HashMap<String, String> = HashMap::new();
        let mut order = HashMap::new();
        let mut remaining: Vec<usize> = (0..sccs.len()).collect();
        let mut done_scc: HashSet<usize> = HashSet::new();
        while !remaining.is_empty() {
            let mut progressed = false;
            let mut still = Vec::new();
            for &si in &remaining {
                let members = &sccs[si];
                let deps_ready = members.iter().all(|m| {
                    refs_of[m].iter().all(|r| {
                        if names.contains(r) {
                            *scc_of.get(r).unwrap() == si || result.contains_key(r)
                        } else {
                            true // import: handled at envelope time
                        }
                    })
                });
                if !deps_ready {
                    still.push(si);
                    continue;
                }
                progressed = true;
                done_scc.insert(si);
                // Resolve external references to hashes (or mark unresolved in closure mode).
                let mut ext_hash: HashMap<String, String> = HashMap::new();
                let mut unresolved = false;
                for m in members {
                    for r in &refs_of[m] {
                        if *scc_of.get(r).unwrap_or(&usize::MAX) == si && names.contains(r) {
                            continue;
                        }
                        if let Some(h) = result.get(r) {
                            ext_hash.insert(r.clone(), h.clone());
                        } else if self.imports.contains_key(r) {
                            match mode {
                                HashMode::Interface => {
                                    if let Some(h) = import_hash(r) {
                                        ext_hash.insert(r.clone(), h);
                                    } else {
                                        unresolved = true;
                                    }
                                }
                                HashMode::Closure => unresolved = true,
                            }
                        } else {
                            unresolved = true; // defined but its own closure unresolved
                        }
                    }
                }
                if unresolved {
                    continue;
                }
                let cyclic = members.len() > 1 || refs_of[&members[0]].contains(&members[0]);
                let mut sorted = members.clone();
                sorted.sort();
                let mut envelopes = Vec::new();
                for (idx, m) in sorted.iter().enumerate() {
                    let sym = self.symbol(m).unwrap();
                    let intra: HashMap<String, String> = if cyclic {
                        sorted
                            .iter()
                            .enumerate()
                            .map(|(j, n)| (n.clone(), format!("${j}")))
                            .collect()
                    } else {
                        HashMap::new()
                    };
                    let _ = idx;
                    envelopes.push(self.envelope(
                        sym,
                        &refs_of[m],
                        &ext_hash,
                        &intra,
                        cyclic,
                        &scc_of,
                        si,
                    )?);
                }
                if !cyclic {
                    let hash = sha256_hex(envelopes[0].as_bytes());
                    order.insert(sorted[0].clone(), (hash.clone(), 0));
                    result.insert(sorted[0].clone(), hash);
                } else {
                    let scc_hash = sha256_hex(envelopes.concat().as_bytes());
                    for (idx, m) in sorted.iter().enumerate() {
                        let hash = sha256_hex(format!("{scc_hash}:{idx}").as_bytes());
                        let key = if sorted.len() == 1 { &hash } else { &scc_hash };
                        order.insert(m.clone(), (key.clone(), idx));
                        result.insert(m.clone(), hash);
                    }
                }
            }
            if !progressed {
                return Err(sec_err(
                    "§9.2",
                    "reference graph could not be ordered (internal)",
                ));
            }
            remaining = still;
        }
        let _ = done_scc;
        Ok((result, order))
    }

    fn part4_text(&self, sym: &Symbol) -> String {
        if sym.kind == "data" {
            format!(
                "{} {}",
                sym.name,
                self.data.get(&sym.name).cloned().unwrap_or_default()
            )
        } else {
            self.function(&sym.name)
                .map(|f| f.bytes.clone())
                .unwrap_or_default()
        }
    }

    fn envelope(
        &self,
        sym: &Symbol,
        refs: &[String],
        ext_hash: &HashMap<String, String>,
        intra: &HashMap<String, String>,
        cyclic: bool,
        scc_of: &HashMap<String, usize>,
        my_scc: usize,
    ) -> Result<String, String> {
        let func = self.function(&sym.name);
        let leaves = self.layout_leaves()?;
        let (data_names, chosen) = self.closure_set(sym, func, &leaves)?;
        // Reference replacement in part (4): outside string literals, the
        // header's own definition excluded (handled inside replace_refs by
        // skipping the first token when it is the definition).
        let regions: HashSet<String> = if sym.kind == "oracle" {
            func.map(|f| f.groups[3].iter().map(|p| p.name.clone()).collect())
                .unwrap_or_default()
        } else {
            HashSet::new()
        };
        let part4 = replace_refs(&self.part4_text(sym), &sym.name, ext_hash, intra, &regions);
        let mut env = String::new();
        env.push_str(&format!(
            "isa={}\n",
            self.header.versions.get("isa").cloned().unwrap_or_default()
        ));
        env.push_str(&format!("ptr_width={}\n", self.header.ptr_width));
        env.push_str(&String::from_utf8(jcs(&sym.signature)?).unwrap());
        env.push('\n');
        env.push_str(&part4);
        env.push('\n');
        for dn in &data_names {
            let hex = self.data.get(dn).ok_or_else(|| {
                sec_err(
                    "§9.2",
                    format!("closure set of @{} names missing data {dn:?}", sym.name),
                )
            })?;
            env.push_str(&format!("data {dn} {hex}\n"));
        }
        for leaf in &chosen {
            env.push_str(&format!(
                "layout {} {}\n",
                leaf.path_rendered,
                String::from_utf8(jcs(&leaf.value)?).unwrap()
            ));
        }
        for line in self.device_lines(sym, func)? {
            env.push_str(&line);
            env.push('\n');
        }
        // Part (6): first-reference order, outside-SCC references only.
        for r in refs {
            if cyclic && scc_of.get(r).map(|s| *s == my_scc).unwrap_or(false) {
                continue;
            }
            if let Some(h) = ext_hash.get(r) {
                env.push_str(h);
                env.push('\n');
            }
        }
        Ok(env)
    }
}

fn make_leaf(
    components: Vec<String>,
    value: Value,
    images: Vec<String>,
    isa: &str,
) -> Result<LayoutLeaf, String> {
    let path_rendered = match isa {
        // 0.8 section 9.2 (5b): dotted path.
        "0.8" => components.join("."),
        // 0.9 section 9.2 (5b) as amended by K46/K47: JCS key-component array.
        "0.9" => String::from_utf8(jcs(&Value::Arr(
            components.iter().cloned().map(Value::Str).collect(),
        ))?)
        .unwrap(),
        other => {
            return Err(sec_err(
                "§9.2",
                format!("unsupported isa version {other:?} for identity"),
            ))
        }
    };
    Ok(LayoutLeaf {
        components,
        value,
        path_rendered,
        images,
    })
}

impl Clone for LayoutLeaf {
    fn clone(&self) -> Self {
        LayoutLeaf {
            components: self.components.clone(),
            value: self.value.clone(),
            path_rendered: self.path_rendered.clone(),
            images: self.images.clone(),
        }
    }
}

/// `@name` tokens outside string literals, in order of appearance,
/// excluding the defining occurrence of `definition` (first `@definition`
/// token at a header position is still returned; callers that replace use
/// `replace_refs`, which handles the definition separately). Here every
/// token after the first `@definition` on the first line is a reference.
fn ref_tokens(text: &str, definition: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut lines = text.split('\n');
    if let Some(first) = lines.next() {
        out.extend(at_tokens(first, Some(definition)));
    }
    for line in lines {
        out.extend(at_tokens(line, None));
    }
    out
}

fn at_tokens(line: &str, skip_first_def: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0;
    let mut skipped = skip_first_def.is_none();
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                i += 1;
            }
            b'@' => {
                let start = i + 1;
                let mut j = start;
                while j < bytes.len()
                    && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_' || bytes[j] == b'.')
                {
                    j += 1;
                }
                if j > start {
                    let name = &line[start..j];
                    if !skipped && Some(name) == skip_first_def {
                        skipped = true;
                    } else {
                        out.push(name.to_string());
                    }
                }
                i = j;
            }
            _ => i += 1,
        }
    }
    out
}

fn addr_targets(bytes: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in bytes.split('\n') {
        if let Some(pos) = line.find("lift.addr @") {
            let rest = &line[pos + "lift.addr @".len()..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.')
                .collect();
            if !name.is_empty() {
                out.push(name);
            }
        }
    }
    out
}

fn replace_refs(
    text: &str,
    definition: &str,
    ext_hash: &HashMap<String, String>,
    intra: &HashMap<String, String>,
    regions: &HashSet<String>,
) -> String {
    let mut out = String::with_capacity(text.len());
    let mut line_index = 0usize;
    for line in text.split('\n') {
        if line_index > 0 {
            out.push('\n');
        }
        let is_first_line = line_index == 0;
        line_index += 1;
        let bytes = line.as_bytes();
        let mut i = 0;
        let mut skipped_def = false;
        while i < bytes.len() {
            match bytes[i] {
                b'"' => {
                    let start = i;
                    i += 1;
                    while i < bytes.len() && bytes[i] != b'"' {
                        if bytes[i] == b'\\' {
                            i += 1;
                        }
                        i += 1;
                    }
                    i = (i + 1).min(bytes.len());
                    out.push_str(&line[start..i]);
                }
                b'@' => {
                    let start = i + 1;
                    let mut j = start;
                    while j < bytes.len()
                        && (bytes[j].is_ascii_alphanumeric()
                            || bytes[j] == b'_'
                            || bytes[j] == b'.')
                    {
                        j += 1;
                    }
                    if j == start {
                        out.push('@');
                        i = j;
                        continue;
                    }
                    let name = &line[start..j];
                    if is_first_line
                        && !skipped_def
                        && name == definition
                        && is_header_def_position(line, i)
                    {
                        // The symbol's own name in its header line is a definition.
                        skipped_def = true;
                        out.push('@');
                        out.push_str(name);
                    } else if regions.contains(name) {
                        out.push('@');
                        out.push_str(name);
                    } else if let Some(repl) = intra.get(name) {
                        out.push('@');
                        out.push_str(repl);
                    } else if let Some(h) = ext_hash.get(name) {
                        out.push('@');
                        out.push_str(h);
                    } else {
                        out.push('@');
                        out.push_str(name);
                    }
                    i = j;
                }
                c => {
                    out.push(c as char);
                    i += 1;
                }
            }
        }
    }
    out
}

fn is_header_def_position(line: &str, at: usize) -> bool {
    // Definition = the first @token of the first line; replace_refs calls
    // this only while scanning that line from its start.
    !line[..at].contains('@')
}

fn kosaraju(nodes: &HashSet<String>, edges: &HashMap<String, Vec<String>>) -> Vec<Vec<String>> {
    let defined = |n: &str| nodes.contains(n);
    let mut visited = HashSet::new();
    let mut order = Vec::new();
    fn visit(
        n: &str,
        edges: &HashMap<String, Vec<String>>,
        nodes: &HashSet<String>,
        visited: &mut HashSet<String>,
        order: &mut Vec<String>,
    ) {
        if !visited.insert(n.to_string()) {
            return;
        }
        if let Some(es) = edges.get(n) {
            for e in es {
                if nodes.contains(e) {
                    visit(e, edges, nodes, visited, order);
                }
            }
        }
        order.push(n.to_string());
    }
    let mut sorted_nodes: Vec<&String> = nodes.iter().collect();
    sorted_nodes.sort();
    for n in sorted_nodes {
        visit(n, edges, nodes, &mut visited, &mut order);
    }
    let mut rev: HashMap<String, Vec<String>> = HashMap::new();
    for (n, es) in edges {
        for e in es {
            if defined(e) {
                rev.entry(e.clone()).or_default().push(n.clone());
            }
        }
    }
    let mut assigned = HashSet::new();
    let mut sccs = Vec::new();
    fn assign(
        n: &str,
        rev: &HashMap<String, Vec<String>>,
        assigned: &mut HashSet<String>,
        comp: &mut Vec<String>,
    ) {
        if !assigned.insert(n.to_string()) {
            return;
        }
        comp.push(n.to_string());
        if let Some(es) = rev.get(n) {
            for e in es {
                assign(e, rev, assigned, comp);
            }
        }
    }
    for n in order.iter().rev() {
        if !assigned.contains(n) {
            let mut comp = Vec::new();
            assign(n, &rev, &mut assigned, &mut comp);
            sccs.push(comp);
        }
    }
    sccs
}
