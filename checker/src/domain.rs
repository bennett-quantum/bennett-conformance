//! Section 12.2 domains: component resolution by identity (never by name
//! sets), validity (K71/K72), coverage of D (K69/K74), enumeration with
//! omitted components held at zero, and honest host limits (K75).

pub use crate::fault::{HostLimit, Reject, ResolveError};

use crate::json::Value;
use crate::object::Interface;
use crate::rat::{fraction_string, is_canonical_uint_str};
use num_bigint::BigUint;
use num_traits::{One, Zero};
use std::collections::HashSet;
use std::str::FromStr;

/// Host cap on bits per domain element / full-domain bit size. Larger legal
/// domains resolve as HostLimit, never as ill-formed (K75).
pub const MAX_ELEMENT_BITS: u64 = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ComponentId {
    Control(usize),
    Bundle { group: String, slot: usize },
    Borrowed(usize),
    Cell { group: String, slot: usize, index: BigUint },
    Arg(usize),
}

#[derive(Debug, Clone)]
pub struct Component {
    pub id: ComponentId,
    pub width: u64,
    pub lo: BigUint,
    pub size: BigUint, // hi - lo + 1
}

/// One full interface tuple (section 12.1, K13) plus quantified classical
/// argument values for inverse-pair domains.
#[derive(Debug, Clone, PartialEq)]
pub struct Tuple {
    pub bundles: Vec<BigUint>, // inherited controls first, then interface bundles
    pub borrowed: Vec<BigUint>,
    pub regions: Vec<Vec<BigUint>>, // per interface region, cells in index order
    pub args: Vec<BigUint>,
}

#[derive(Debug, Clone)]
pub struct ResolvedDomain {
    /// Resolved components in enumeration order (listed order; last fastest).
    pub components: Vec<Component>,
    /// Widths of every full-D component in tuple order, for table validation.
    pub bundle_widths: Vec<u64>,
    pub borrowed_widths: Vec<u64>,
    /// (cell count, cell width) per interface region.
    pub region_shapes: Vec<(BigUint, u64)>,
    pub arg_widths: Vec<u64>,
    pub n: BigUint,
    pub full_n: BigUint,
    pub covers_full: bool,
    /// Tuple layout: bundle position i holds this component identity.
    pub bundle_positions: Vec<ComponentId>,
    /// Region position i identifies its (group, slot) via a probe cell id.
    pub region_positions: Vec<ComponentId>,
}

impl ResolvedDomain {
    pub fn covered_fraction(&self) -> String {
        fraction_string(&self.n, &self.full_n)
    }

    /// Enumerate every element; caller must have bounded `n` (see cert layer).
    pub fn elements(&self) -> Elements<'_> {
        Elements { d: self, next: BigUint::zero() }
    }

    pub fn element_at(&self, mut index: BigUint) -> Tuple {
        let mut tuple = self.zero_tuple();
        // Last listed component fastest: peel digits from the last component.
        let mut values: Vec<BigUint> = vec![BigUint::zero(); self.components.len()];
        for i in (0..self.components.len()).rev() {
            let c = &self.components[i];
            let digit = &index % &c.size;
            index /= &c.size;
            values[i] = &c.lo + digit;
        }
        for (c, v) in self.components.iter().zip(values) {
            self.set_component(&mut tuple, c, v);
        }
        tuple
    }

    pub fn zero_tuple(&self) -> Tuple {
        Tuple {
            bundles: vec![BigUint::zero(); self.bundle_widths.len()],
            borrowed: vec![BigUint::zero(); self.borrowed_widths.len()],
            regions: self
                .region_shapes
                .iter()
                .map(|(count, _)| vec![BigUint::zero(); count.to_string().parse::<usize>().unwrap_or(0)])
                .collect(),
            args: vec![BigUint::zero(); self.arg_widths.len()],
        }
    }

    pub fn set_component(&self, tuple: &mut Tuple, c: &Component, v: BigUint) {
        match &c.id {
            ComponentId::Control(i) => tuple.bundles[*i] = v,
            ComponentId::Bundle { .. } => {
                let pos = self.bundle_pos(c);
                tuple.bundles[pos] = v;
            }
            ComponentId::Borrowed(k) => tuple.borrowed[*k] = v,
            ComponentId::Cell { slot, index, .. } => {
                // Interface regions are stored in interface order; cell lists
                // in the tuple are per region position. `slot` is per-role,
                // so route through the region position table.
                let pos = self.region_pos_of(c);
                tuple.regions[pos][index.to_string().parse::<usize>().unwrap_or(0)] = v;
            }
            ComponentId::Arg(k) => tuple.args[*k] = v,
        }
    }

    fn bundle_pos(&self, c: &Component) -> usize {
        self.bundle_positions
            .iter()
            .position(|id| id == &c.id)
            .expect("resolved bundle component must have a tuple position")
    }
    fn region_pos_of(&self, c: &Component) -> usize {
        self.region_positions
            .iter()
            .position(|id| match (id, &c.id) {
                (ComponentId::Cell { group: g1, slot: s1, .. }, ComponentId::Cell { group: g2, slot: s2, .. }) => {
                    g1 == g2 && s1 == s2
                }
                _ => false,
            })
            .expect("resolved cell component must have a region position")
    }

}

pub struct Elements<'a> {
    d: &'a ResolvedDomain,
    next: BigUint,
}

impl Iterator for Elements<'_> {
    type Item = Tuple;
    fn next(&mut self) -> Option<Tuple> {
        if self.next >= self.d.n {
            return None;
        }
        let t = self.d.element_at(self.next.clone());
        self.next += BigUint::one();
        Some(t)
    }
}

fn reject(code: &'static str, section: &'static str, msg: String) -> ResolveError {
    ResolveError::Reject(Reject::new(code, section, "", msg))
}

fn domain_reject(msg: String) -> ResolveError {
    reject("E_CERT_DOMAIN", "§12.2", msg)
}

/// Parse a nonnegative integer given as a JSON number or canonical decimal
/// string (section 9.1: numbers >= 2^53 are errors in metadata).
pub fn biguint_of_json(v: &Value, what: &str) -> Result<BigUint, ResolveError> {
    match v {
        Value::Num(lex) => {
            let i: i128 = lex
                .parse()
                .map_err(|_| domain_reject(format!("{what} is not an integer: {lex:?}")))?;
            if i < 0 {
                return Err(domain_reject(format!("{what} is negative: {lex:?}")));
            }
            if i >= (1i128 << 53) {
                return Err(reject(
                    "E_CERT_SCHEMA",
                    "§9.1",
                    format!("{what}: JSON number >= 2^53 in metadata ({lex})"),
                ));
            }
            Ok(BigUint::from(i as u64))
        }
        Value::Str(s) => {
            if is_canonical_uint_str(s) {
                BigUint::from_str(s).map_err(|_| domain_reject(format!("{what}: bad decimal string")))
            } else {
                Err(domain_reject(format!("{what}: non-canonical decimal string {s:?}")))
            }
        }
        other => Err(domain_reject(format!("{what}: expected integer, got {}", other.kind()))),
    }
}

fn u64_of(v: &BigUint, what: &str) -> Result<u64, ResolveError> {
    v.to_string()
        .parse::<u64>()
        .map_err(|_| ResolveError::HostLimit(HostLimit { quantity: format!("{what} = {v} exceeds host u64") }))
}

struct Entry {
    lo: BigUint,
    hi: BigUint,
    width: u64,
}

fn parse_range_entry(e: &Value, what: &str) -> Result<(String, Entry), ResolveError> {
    let obj = e.as_obj().ok_or_else(|| domain_reject(format!("{what} entry is not an object")))?;
    for k in obj.keys() {
        if !matches!(k.as_str(), "name" | "width" | "lo" | "hi") {
            return Err(reject("E_CERT_SCHEMA", "§12.2", format!("{what} entry has unknown field {k:?}")));
        }
    }
    let name = e
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| domain_reject(format!("{what} entry lacks a string name")))?
        .to_string();
    let width = u64_of(&biguint_of_json(e.get("width").unwrap_or(&Value::Null), "width")?, "width")?;
    let lo = biguint_of_json(e.get("lo").unwrap_or(&Value::Null), "lo")?;
    let hi = biguint_of_json(e.get("hi").unwrap_or(&Value::Null), "hi")?;
    Ok((name, Entry { lo, hi, width }))
}

fn check_range(e: &Entry, width: u64, what: &str) -> Result<BigUint, ResolveError> {
    if e.width != width {
        return Err(domain_reject(format!("{what}: supplied width {} != component width {width}", e.width)));
    }
    if e.lo > e.hi {
        return Err(domain_reject(format!("{what}: lo > hi")));
    }
    // hi < 2^width, decided by bit length when width is huge.
    if width < 1_000_000 {
        if e.hi >= (BigUint::one() << width) {
            return Err(domain_reject(format!("{what}: hi >= 2^{width}")));
        }
    } else if e.hi.bits() > width {
        return Err(domain_reject(format!("{what}: hi >= 2^{width}")));
    }
    Ok(&e.hi - &e.lo + BigUint::one())
}

fn full_range(e: &Entry, width: u64) -> bool {
    e.lo.is_zero() && width < 1_000_000 && e.hi == (BigUint::one() << width) - BigUint::one()
}

/// Resolve a `domain` object against the subject interface with
/// `inherited_controls` prepended control wires (sections 12.1/12.2).
pub fn resolve_domain(
    domain: &Value,
    iface: &Interface,
    inherited_controls: u64,
) -> Result<ResolvedDomain, ResolveError> {
    let obj = domain
        .as_obj()
        .ok_or_else(|| domain_reject("domain is not an object".into()))?;
    for k in obj.keys() {
        if !matches!(k.as_str(), "bundles" | "borrowed" | "regions" | "args") {
            return Err(reject("E_CERT_SCHEMA", "§12.2", format!("domain has unknown field {k:?}")));
        }
    }
    let get_arr = |key: &str| -> Result<Vec<Value>, ResolveError> {
        match domain.get(key) {
            None => Err(domain_reject(format!("domain lacks required field {key:?}"))),
            Some(Value::Arr(a)) => Ok(a.clone()),
            Some(o) => Err(domain_reject(format!("domain.{key} is {}, expected array", o.kind()))),
        }
    };
    let c = inherited_controls as usize;
    let mut components: Vec<Component> = Vec::new();
    let mut seen: HashSet<ComponentId> = HashSet::new();
    let mut push = |comp: Component, components: &mut Vec<Component>, seen: &mut HashSet<ComponentId>| -> Result<(), ResolveError> {
        if !seen.insert(comp.id.clone()) {
            return Err(domain_reject(format!("component {:?} occurs twice in domain", comp.id)));
        }
        components.push(comp);
        Ok(())
    };

    // Full-D identity sets.
    let full_bundles: HashSet<ComponentId> = iface
        .bundles
        .iter()
        .map(|b| ComponentId::Bundle { group: b.group.clone(), slot: b.slot })
        .collect();
    let mut covered: HashSet<ComponentId> = HashSet::new(); // present at full range

    // Bundles: first c entries denote inherited controls (position only).
    let bundle_entries = get_arr("bundles")?;
    if bundle_entries.len() < c {
        return Err(domain_reject(format!(
            "domain has {} bundle entries but subject has {c} inherited controls (all c entries required)",
            bundle_entries.len()
        )));
    }
    for (i, e) in bundle_entries.iter().enumerate() {
        let (name, entry) = parse_range_entry(e, "bundles")?;
        if i < c {
            let size = check_range(&entry, 1, &format!("inherited control {i}"))?;
            let id = ComponentId::Control(i);
            if full_range(&entry, 1) {
                covered.insert(id.clone());
            }
            push(Component { id, width: 1, lo: entry.lo, size }, &mut components, &mut seen)?;
        } else {
            let matches: Vec<_> = iface.bundles.iter().filter(|b| b.name == name).collect();
            let b = match matches.len() {
                0 => {
                    return Err(domain_reject(format!(
                        "bundles entry name {name:?} is not an interface bundle of the subject"
                    )))
                }
                1 => matches[0],
                _ => {
                    return Err(domain_reject(format!(
                        "bundles entry name {name:?} is ambiguous in the subject header"
                    )))
                }
            };
            let size = check_range(&entry, b.width, &format!("bundle {name:?}"))?;
            let id = ComponentId::Bundle { group: b.group.clone(), slot: b.slot };
            if full_range(&entry, b.width) {
                covered.insert(id.clone());
            }
            push(Component { id, width: b.width, lo: entry.lo, size }, &mut components, &mut seen)?;
        }
    }

    // Borrowed: entry k is borrow lifetime k; the name is a label only.
    for (k, e) in get_arr("borrowed")?.iter().enumerate() {
        let (_label, entry) = parse_range_entry(e, "borrowed")?;
        let width = *iface.borrowed_widths.get(k).ok_or_else(|| {
            domain_reject(format!(
                "borrowed entry {k} exceeds the subject's checked borrow requirement ({})",
                iface.borrowed_widths.len()
            ))
        })?;
        let size = check_range(&entry, width, &format!("borrowed {k}"))?;
        let id = ComponentId::Borrowed(k);
        if full_range(&entry, width) {
            covered.insert(id.clone());
        }
        push(Component { id, width, lo: entry.lo, size }, &mut components, &mut seen)?;
    }

    // Regions: name resolves to an interface region; cells as listed.
    let mut region_full: Vec<bool> = vec![false; iface.regions.len()];
    for e in get_arr("regions")? {
        let eobj = e.as_obj().ok_or_else(|| domain_reject("regions entry is not an object".into()))?;
        for k in eobj.keys() {
            if !matches!(k.as_str(), "name" | "cells") {
                return Err(reject("E_CERT_SCHEMA", "§12.2", format!("regions entry has unknown field {k:?}")));
            }
        }
        let name = e
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| domain_reject("regions entry lacks a string name".into()))?;
        let rpos = iface
            .regions
            .iter()
            .position(|r| r.name == name)
            .ok_or_else(|| domain_reject(format!("regions entry name {name:?} is not an interface region")))?;
        let region = &iface.regions[rpos];
        match e.get("cells") {
            Some(Value::Str(s)) if s == "all" => {
                // Every cell at full range. Materialise components lazily by
                // count arithmetic: one component per cell is required for
                // enumeration, bounded by the host bit cap below.
                let cell_count = if region.a >= 64 {
                    return Err(ResolveError::HostLimit(HostLimit {
                        quantity: format!("region {name:?} cell count 2^{} (§12.2 host limit)", region.a),
                    }));
                } else {
                    1u64 << region.a
                };
                if cell_count as u128 * region.w as u128 > MAX_ELEMENT_BITS as u128 {
                    return Err(ResolveError::HostLimit(HostLimit {
                        quantity: format!(
                            "region {name:?} element bits 2^{}*{} (§12.2 host limit)",
                            region.a, region.w
                        ),
                    }));
                }
                for index in 0..cell_count {
                    let id = ComponentId::Cell {
                        group: region.group.clone(),
                        slot: region.slot,
                        index: BigUint::from(index),
                    };
                    push(
                        Component {
                            id,
                            width: region.w,
                            lo: BigUint::zero(),
                            size: BigUint::one() << region.w,
                        },
                        &mut components,
                        &mut seen,
                    )?;
                }
                region_full[rpos] = true;
            }
            Some(Value::Arr(cells)) => {
                let mut all_full = true;
                for cell in cells {
                    let cobj = cell.as_obj().ok_or_else(|| domain_reject("region cell entry is not an object".into()))?;
                    for k in cobj.keys() {
                        if !matches!(k.as_str(), "index" | "lo" | "hi") {
                            return Err(reject("E_CERT_SCHEMA", "§12.2", format!("region cell has unknown field {k:?}")));
                        }
                    }
                    let index = biguint_of_json(cell.get("index").unwrap_or(&Value::Null), "cell index")?;
                    if !(region.a >= 64 || index < (BigUint::one() << region.a)) && index.bits() > region.a {
                        return Err(domain_reject(format!("cell index {index} >= 2^{} in region {name:?}", region.a)));
                    }
                    if region.a < 64 && index >= (BigUint::one() << region.a) {
                        return Err(domain_reject(format!("cell index {index} >= 2^{} in region {name:?}", region.a)));
                    }
                    let lo = biguint_of_json(cell.get("lo").unwrap_or(&Value::Null), "cell lo")?;
                    let hi = biguint_of_json(cell.get("hi").unwrap_or(&Value::Null), "cell hi")?;
                    let entry = Entry { lo, hi, width: region.w };
                    let size = check_range(&entry, region.w, &format!("region {name:?} cell {index}"))?;
                    if !full_range(&entry, region.w) {
                        all_full = false;
                    }
                    let id = ComponentId::Cell { group: region.group.clone(), slot: region.slot, index };
                    push(Component { id, width: region.w, lo: entry.lo, size }, &mut components, &mut seen)?;
                }
                // Explicit list covers the region iff it lists every cell at
                // full range (distinctness already enforced).
                let covers = all_full
                    && region.a < 64
                    && (cells.len() as u64) == (1u64 << region.a);
                if covers {
                    region_full[rpos] = true;
                }
            }
            Some(o) => return Err(domain_reject(format!("regions.cells is {}, expected \"all\" or array", o.kind()))),
            None => return Err(domain_reject(format!("regions entry {name:?} lacks cells"))),
        }
    }

    // Args (inverse-pair domains): same entry shape; widths cross-checked
    // against signature types by the cert layer (needs ptr_width).
    let mut arg_widths: Vec<u64> = Vec::new();
    if let Some(Value::Arr(args)) = domain.get("args") {
        for (k, e) in args.iter().enumerate() {
            let (_name, entry) = parse_range_entry(e, "args")?;
            let size = check_range(&entry, entry.width, &format!("args {k}"))?;
            if entry.width == 0 {
                return Err(domain_reject(format!("args {k} has width 0")));
            }
            arg_widths.push(entry.width);
            push(
                Component { id: ComponentId::Arg(k), width: entry.width, lo: entry.lo, size },
                &mut components,
                &mut seen,
            )?;
        }
    } else if let Some(o) = domain.get("args") {
        return Err(domain_reject(format!("domain.args is {}, expected array", o.kind())));
    }

    // Counts. Full D bits decide host materialisability.
    let mut full_bits: BigUint = BigUint::from(inherited_controls);
    for b in &iface.bundles {
        full_bits += BigUint::from(b.width);
    }
    for w in &iface.borrowed_widths {
        full_bits += BigUint::from(*w);
    }
    let mut region_shapes: Vec<(BigUint, u64)> = Vec::new();
    for r in &iface.regions {
        if r.a > 1_000_000 {
            return Err(ResolveError::HostLimit(HostLimit {
                quantity: format!("full domain bits: region {:?} has 2^{} cells (§12.2 host limit)", r.name, r.a),
            }));
        }
        full_bits += (BigUint::one() << r.a) * BigUint::from(r.w);
        region_shapes.push((BigUint::one() << r.a, r.w));
    }
    if full_bits > BigUint::from(MAX_ELEMENT_BITS) {
        return Err(ResolveError::HostLimit(HostLimit {
            quantity: format!("domain element bits ({full_bits}) (§12.2 host limit, cap {MAX_ELEMENT_BITS})"),
        }));
    }
    let full_bits_u = full_bits.to_string().parse::<usize>().unwrap_or(0);
    let full_n = BigUint::one() << full_bits_u;
    let mut n = BigUint::one();
    for comp in &components {
        n *= &comp.size;
    }
    let covers_full = covered
        .iter()
        .filter(|id| matches!(id, ComponentId::Control(_)))
        .count()
        == c
        && full_bundles.iter().all(|id| covered.contains(id))
        && (0..iface.borrowed_widths.len()).all(|k| covered.contains(&ComponentId::Borrowed(k)))
        && region_full.iter().all(|f| *f);

    let bundle_widths: Vec<u64> = std::iter::repeat(1u64)
        .take(c)
        .chain(iface.bundles.iter().map(|b| b.width))
        .collect();
    let bundle_positions: Vec<ComponentId> = (0..c)
        .map(ComponentId::Control)
        .chain(
            iface
                .bundles
                .iter()
                .map(|b| ComponentId::Bundle { group: b.group.clone(), slot: b.slot }),
        )
        .collect();
    let region_positions: Vec<ComponentId> = iface
        .regions
        .iter()
        .map(|r| ComponentId::Cell { group: r.group.clone(), slot: r.slot, index: BigUint::zero() })
        .collect();

    Ok(ResolvedDomain {
        components,
        bundle_widths,
        borrowed_widths: iface.borrowed_widths.clone(),
        region_shapes,
        arg_widths,
        n,
        full_n,
        covers_full,
        bundle_positions,
        region_positions,
    })
}
