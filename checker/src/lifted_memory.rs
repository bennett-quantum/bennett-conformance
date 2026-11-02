//! Sparse byte memory and monotonic allocation for F_B (ISA §§4.2, 7.2).
//! Quantum oracle construction later uncomputes this private forward state.
use crate::cert::{check_keys, u64_of};
use crate::eval::EvalError;
use crate::fault::Reject;
use crate::json::Value;
use crate::object::Object;
use num_bigint::BigUint;
use num_traits::{ToPrimitive, Zero};
use std::collections::{BTreeMap, HashMap};
fn bad(message: impl Into<String>) -> EvalError {
    EvalError::Reject(Reject::new(
        "E_CERT_OBJECT",
        "§4.2/§7.2",
        "object.layout",
        message,
    ))
}
fn keys(value: &Value, fields: &[&str], required: &[&str]) -> Result<(), EvalError> {
    let mut errors = Vec::new();
    if !check_keys(value, fields, required, "object.layout", &mut errors) {
        return Err(bad(errors
            .into_iter()
            .map(|e| e.message)
            .collect::<Vec<_>>()
            .join("; ")));
    }
    Ok(())
}
fn uint(value: Option<&Value>, field: &str) -> Result<u64, EvalError> {
    value.and_then(u64_of).ok_or_else(|| {
        bad(format!(
            "{field} is not a supported exact nonnegative integer"
        ))
    })
}
struct Entry {
    base: u64,
    size: u64,
    live: bool,
    pinned: bool,
}
struct Mapping {
    base: u64,
    size: u64,
    name: String,
}
pub(crate) struct Memory {
    entries: Vec<Entry>,
    mapped: Vec<Mapping>,
    bytes: BTreeMap<u64, u8>,
    bases: HashMap<String, u64>,
    heap_base: u64,
    heap_used: u64,
    heap_limit: u64,
    table_bits: u64,
}
impl Memory {
    pub(crate) fn new(
        object: &Object,
        oracle: &str,
        mem: &HashMap<String, Vec<BigUint>>,
        heap_limit: &BigUint,
    ) -> Result<Self, EvalError> {
        let layout = object
            .layout
            .as_ref()
            .ok_or_else(|| bad("memory rows are forbidden in a memoryless object"))?;
        keys(
            layout,
            &[
                "ptr_width",
                "H",
                "A_tab",
                "S_cells",
                "null_size",
                "heap_base",
                "heap_limit_max",
                "globals",
                "interface_regions",
                "block_image",
            ],
            &[
                "ptr_width",
                "H",
                "A_tab",
                "S_cells",
                "null_size",
                "heap_base",
                "heap_limit_max",
                "globals",
                "interface_regions",
            ],
        )?;
        let h = uint(layout.get("H"), "H")?;
        if uint(layout.get("ptr_width"), "ptr_width")? != object.header.ptr_width
            || h > object.header.ptr_width
        {
            return Err(bad(
                "layout and object pointer widths disagree, or H > ptr_width",
            ));
        }
        if h > 63 {
            return Err(EvalError::Unchecked(
                "host limit: sparse lifted memory currently supports H <= 63 (§12.2)".into(),
            ));
        }
        let end = 1u64 << h;
        let null = uint(layout.get("null_size"), "null_size")?;
        let heap_base = uint(layout.get("heap_base"), "heap_base")?;
        let heap_max = uint(layout.get("heap_limit_max"), "heap_limit_max")?;
        let table_bits = uint(layout.get("A_tab"), "A_tab")?;
        uint(layout.get("S_cells"), "S_cells")?;
        if null == 0
            || null > end
            || heap_base
                .checked_add(heap_max)
                .map(|e| e > end)
                .unwrap_or(true)
        {
            return Err(bad("null/heap ranges are outside lifted memory"));
        }
        let heap_limit = heap_limit
            .to_u64()
            .filter(|n| *n <= heap_max)
            .ok_or_else(|| bad("heap budget exceeds layout.heap_limit_max"))?;
        let mut result = Self {
            entries: Vec::new(),
            mapped: Vec::new(),
            bytes: BTreeMap::new(),
            bases: HashMap::new(),
            heap_base,
            heap_used: 0,
            heap_limit,
            table_bits,
        };
        let mut ranges = vec![(0, null), (heap_base, heap_max)];
        let globals = layout
            .get("globals")
            .and_then(Value::as_obj)
            .ok_or_else(|| bad("globals must be an object"))?;
        for (name, global) in globals {
            keys(
                global,
                &["base", "size", "image"],
                &["base", "size", "image"],
            )?;
            let base = uint(global.get("base"), "global.base")?;
            let size = uint(global.get("size"), "global.size")?;
            let image = global
                .get("image")
                .and_then(Value::as_str)
                .and_then(|name| object.data.get(name))
                .ok_or_else(|| bad("global image does not name data"))?;
            if image.len() as u128 != u128::from(size) * 2 {
                return Err(bad("global image byte size differs from layout size"));
            }
            for (i, pair) in image.as_bytes().chunks_exact(2).enumerate() {
                let value = u8::from_str_radix(
                    std::str::from_utf8(pair).map_err(|_| bad("invalid image bytes"))?,
                    16,
                )
                .map_err(|_| bad("invalid image hex"))?;
                if value != 0 {
                    if result.bytes.len() >= 1_000_000 {
                        return Err(EvalError::Unchecked(
                            "host limit: lifted memory image exceeds one million nonzero bytes"
                                .into(),
                        ));
                    }
                    result.bytes.insert(
                        base.checked_add(i as u64)
                            .ok_or_else(|| bad("global image address overflow"))?,
                        value,
                    );
                }
            }
            ranges.push((base, size));
            result.bases.insert(name.clone(), base);
            result.entries.push(Entry {
                base,
                size,
                live: true,
                pinned: true,
            });
        }
        let all_mappings = layout
            .get("interface_regions")
            .and_then(Value::as_obj)
            .ok_or_else(|| bad("interface_regions must be an object"))?;
        let iface = object.interface(oracle).map_err(bad)?;
        let mapping = all_mappings
            .get(oracle)
            .map(|m| {
                m.as_obj()
                    .ok_or_else(|| bad("oracle interface mapping must be an object"))
            })
            .transpose()?;
        for region in &iface.regions {
            if object.symbol(&region.name).is_some() || object.imports.contains_key(&region.name) {
                return Err(bad(
                    "interface region names must differ from every symbol/import (§9.1)",
                ));
            }
            if region.w != 8 {
                return Err(bad("lifted mapped interface regions require W = 8"));
            }
            let value = mapping
                .and_then(|m| m.get(&region.name))
                .ok_or_else(|| bad("interface region has no layout base"))?;
            keys(value, &["base"], &["base"])?;
            let base = uint(value.get("base"), "interface.base")?;
            if region.a > 63 {
                return Err(bad("interface region exceeds the H <= 63 memory range"));
            }
            let size = 1u64 << region.a;
            let cells = mem
                .get(&region.name)
                .ok_or_else(|| bad("mapped interface region tuple is missing"))?;
            if cells.len() as u128 != u128::from(size) || cells.iter().any(|v| v.bits() > 8) {
                return Err(bad("mapped interface tuple does not match its byte region"));
            }
            ranges.push((base, size));
            result.bases.insert(region.name.clone(), base);
            result.entries.push(Entry {
                base,
                size,
                live: true,
                pinned: true,
            });
            result.mapped.push(Mapping {
                base,
                size,
                name: region.name.clone(),
            });
        }
        if mapping.map(|m| m.len()).unwrap_or(0) != iface.regions.len() {
            return Err(bad("layout maps an unknown region of the oracle"));
        }
        for (i, (base, size)) in ranges.iter().enumerate() {
            let stop = base
                .checked_add(*size)
                .filter(|e| *e <= end)
                .ok_or_else(|| bad("layout range exceeds lifted memory"))?;
            for (other, extent) in ranges.iter().take(i) {
                if *size != 0 && *extent != 0 && *base < other + extent && *other < stop {
                    return Err(bad("layout ranges overlap"));
                }
            }
        }
        if result.table_full()
            && (table_bits < 64 && result.entries.len() as u64 > (1u64 << table_bits))
        {
            return Err(bad("too many pinned allocation-table pre-entries"));
        }
        if result.bytes.len() > 1_000_000 {
            return Err(EvalError::Unchecked(
                "host limit: lifted memory image exceeds one million nonzero bytes".into(),
            ));
        }
        Ok(result)
    }
    fn table_full(&self) -> bool {
        self.table_bits < usize::BITS as u64 && self.entries.len() >= (1usize << self.table_bits)
    }
    pub(crate) fn address(&self, name: &str) -> Option<u64> {
        self.bases.get(name).copied()
    }
    pub(crate) fn inrange(&self, p: &BigUint, n: &BigUint) -> bool {
        let (Some(p), Some(n)) = (p.to_u64(), n.to_u64()) else {
            return false;
        };
        let Some(end) = p.checked_add(n) else {
            return false;
        };
        self.entries
            .iter()
            .any(|e| e.live && e.base <= p && end <= e.base + e.size)
    }
    pub(crate) fn allocate(&mut self, size: &BigUint) -> Result<Option<u64>, EvalError> {
        let Some(size) = size.to_u64() else {
            return Ok(None);
        };
        let Some(used) = self
            .heap_used
            .checked_add(size.max(1))
            .filter(|n| *n <= self.heap_limit)
        else {
            return Ok(None);
        };
        if self.table_full() {
            return Ok(None);
        }
        if self.entries.len() >= 1_000_000 {
            return Err(EvalError::Unchecked(
                "host limit: lifted allocation table exceeds one million entries".into(),
            ));
        }
        let base = self.heap_base + self.heap_used;
        self.entries.push(Entry {
            base,
            size,
            live: true,
            pinned: false,
        });
        self.heap_used = used;
        Ok(Some(base))
    }
    pub(crate) fn free(&mut self, p: &BigUint) -> bool {
        let Some(p) = p.to_u64() else {
            return false;
        };
        let Some(entry) = self
            .entries
            .iter_mut()
            .find(|e| e.base == p && !e.pinned && e.live)
        else {
            return false;
        };
        entry.live = false;
        true
    }
    fn byte(&self, address: u64, mem: &HashMap<String, Vec<BigUint>>) -> u8 {
        for mapping in &self.mapped {
            if mapping.base <= address && address - mapping.base < mapping.size {
                return mem[&mapping.name][(address - mapping.base) as usize]
                    .to_u8()
                    .expect("validated byte tuple");
            }
        }
        self.bytes.get(&address).copied().unwrap_or(0)
    }
    pub(crate) fn load(
        &self,
        p: &BigUint,
        bytes: usize,
        mem: &HashMap<String, Vec<BigUint>>,
    ) -> BigUint {
        let p = p.to_u64().expect("inrange checked");
        let mut result = BigUint::zero();
        for i in 0..bytes {
            result |= BigUint::from(self.byte(p + i as u64, mem)) << (8 * i);
        }
        result
    }
    pub(crate) fn store(
        &mut self,
        p: &BigUint,
        value: &BigUint,
        bytes: usize,
        mem: &mut HashMap<String, Vec<BigUint>>,
    ) -> Result<bool, EvalError> {
        if !self.inrange(p, &bytes.into()) {
            return Ok(false);
        }
        let p = p.to_u64().expect("inrange checked");
        if self.bytes.len() + bytes > 1_000_000 {
            return Err(EvalError::Unchecked(
                "host limit: lifted sparse byte storage exceeds one million entries".into(),
            ));
        }
        for i in 0..bytes {
            let byte = ((value >> (8 * i)) & BigUint::from(255u32))
                .to_u8()
                .unwrap();
            let address = p + i as u64;
            if let Some(mapping) = self
                .mapped
                .iter()
                .find(|m| m.base <= address && address - m.base < m.size)
            {
                mem.get_mut(&mapping.name).unwrap()[(address - mapping.base) as usize] =
                    byte.into();
            } else if byte == 0 {
                self.bytes.remove(&address);
            } else {
                self.bytes.insert(address, byte);
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn object() -> Object {
        Object::parse(include_str!(
            "../tests/fixtures/replay-differential/oracle-store-load-heap/object.bisa"
        ))
        .unwrap()
    }
    fn memory(limit: u32) -> Memory {
        Memory::new(&object(), "o", &HashMap::new(), &limit.into()).unwrap()
    }
    #[test]
    fn bytes_are_little_endian_unaligned_and_accesses_need_one_live_entry() {
        let mut m = memory(32);
        let mut mem = HashMap::new();
        assert_eq!(m.address("g"), Some(4));
        assert_eq!(m.load(&4u32.into(), 2, &mem), BigUint::from(0x0b0au32));
        let p = m.allocate(&4u32.into()).unwrap().unwrap();
        assert_eq!(p, 16);
        assert_eq!(m.load(&p.into(), 4, &mem), BigUint::zero());
        m.store(&p.into(), &0x12345678u32.into(), 4, &mut mem)
            .unwrap();
        assert_eq!(m.load(&17u32.into(), 2, &mem), BigUint::from(0x3456u32));
        let q = m.allocate(&1u32.into()).unwrap().unwrap();
        assert_eq!(q, 20);
        assert!(
            !m.inrange(&19u32.into(), &2u32.into()),
            "adjacent allocations cannot be combined into one access"
        );
        assert!(
            !m.inrange(&u64::MAX.into(), &2u32.into()),
            "address addition cannot wrap"
        );
        assert!(!m.inrange(&0u32.into(), &0u32.into()));
        assert!(m.inrange(&20u32.into(), &0u32.into()));
    }
    #[test]
    fn zero_allocations_advance_once_and_free_never_reclaims_heap_or_table() {
        let mut m = memory(32);
        for expected in 16..19 {
            let p = m.allocate(&0u32.into()).unwrap().unwrap();
            assert_eq!(p, expected);
            assert!(m.inrange(&p.into(), &0u32.into()));
            assert!(!m.inrange(&p.into(), &1u32.into()));
            assert!(m.free(&p.into()));
            assert!(!m.free(&p.into()));
        }
        assert_eq!(
            m.allocate(&0u32.into()).unwrap(),
            None,
            "table entries remain history after free"
        );
        assert_eq!(m.heap_used, 3);
        assert_eq!(m.entries.len(), 4);
        let mut m = memory(4);
        assert_eq!(m.allocate(&4u32.into()).unwrap(), Some(16));
        assert!(m.free(&16u32.into()));
        assert_eq!(
            m.allocate(&0u32.into()).unwrap(),
            None,
            "free does not lower the bump counter"
        );
    }
    #[test]
    fn failing_allocations_and_frees_do_not_change_forward_state() {
        let mut m = memory(4);
        let old = m.entries.len();
        for n in [BigUint::from(5u32), BigUint::from(1u32) << 200] {
            assert_eq!(m.allocate(&n).unwrap(), None);
            assert_eq!(m.entries.len(), old);
            assert_eq!(m.heap_used, 0);
        }
        assert!(
            !m.free(&4u32.into()),
            "global pinned entries cannot be freed"
        );
        assert!(!m.free(&0u32.into()));
        assert!(m.entries[0].live);
    }
    #[test]
    fn all_thirteen_layout_fields_and_records_reject_wrong_types() {
        let paths = [
            "A_tab",
            "H",
            "S_cells",
            "globals",
            "globals.g",
            "globals.g.base",
            "globals.g.image",
            "globals.g.size",
            "heap_base",
            "heap_limit_max",
            "interface_regions",
            "null_size",
            "ptr_width",
        ];
        fn field_count(value: &Value) -> usize {
            match value {
                Value::Obj(map) => map.values().map(|child| 1 + field_count(child)).sum(),
                Value::Arr(array) => array.iter().map(|child| 1 + field_count(child)).sum(),
                _ => 0,
            }
        }
        assert_eq!(
            field_count(object().layout.as_ref().unwrap()),
            paths.len(),
            "layout fixture field inventory changed"
        );
        for path in paths {
            let mut object = object();
            let mut value = object.layout.as_mut().unwrap();
            for key in path.split('.') {
                let Value::Obj(map) = value else {
                    unreachable!()
                };
                value = map.get_mut(key).unwrap();
            }
            *value = Value::Null;
            let result = Memory::new(&object, "o", &HashMap::new(), &4u32.into());
            assert!(
                matches!(result,Err(EvalError::Reject(ref r))if r.code=="E_CERT_OBJECT"),
                "{path}"
            );
        }
    }
    #[test]
    fn layout_overlaps_pointer_mismatches_and_wrong_images_reject() {
        for (field, value) in [
            ("null_size", crate::json::n(5)),
            ("ptr_width", crate::json::n(32)),
            ("H", crate::json::n(4)),
        ] {
            let mut object = object();
            let Value::Obj(layout) = object.layout.as_mut().unwrap() else {
                unreachable!()
            };
            layout.insert(field.into(), value);
            assert!(matches!(
                Memory::new(&object, "o", &HashMap::new(), &4u32.into()),
                Err(EvalError::Reject(_))
            ));
        }
        for (field, value) in [
            ("size", crate::json::n(3)),
            ("image", crate::json::s("missing")),
        ] {
            let mut object = object();
            let Value::Obj(layout) = object.layout.as_mut().unwrap() else {
                unreachable!()
            };
            let Value::Obj(globals) = layout.get_mut("globals").unwrap() else {
                unreachable!()
            };
            let Value::Obj(global) = globals.get_mut("g").unwrap() else {
                unreachable!()
            };
            global.insert(field.into(), value);
            assert!(matches!(
                Memory::new(&object, "o", &HashMap::new(), &4u32.into()),
                Err(EvalError::Reject(_))
            ));
        }
    }
    #[test]
    fn invalid_stores_have_no_partial_byte_delta() {
        let mut m = memory(32);
        let mut mem = HashMap::new();
        let p = m.allocate(&1u32.into()).unwrap().unwrap();
        assert!(!m.store(&p.into(), &65535u32.into(), 2, &mut mem).unwrap());
        assert_eq!(m.load(&p.into(), 1, &mem), BigUint::zero());
        assert_eq!(m.bytes.len(), 2, "only the two global-image bytes exist");
    }
    #[test]
    fn mapped_interface_bytes_share_the_address_space_and_are_pinned() {
        let mut object = object();
        object
            .functions
            .iter_mut()
            .find(|f| f.name == "o")
            .unwrap()
            .groups[3]
            .push(crate::object::Param {
                name: "db".into(),
                ty: "region[1,8]".into(),
                suffix: Some("in".into()),
            });
        let symbol = object.symbols.iter_mut().find(|s| s.name == "o").unwrap();
        let Value::Obj(signature) = &mut symbol.signature else {
            unreachable!()
        };
        signature.insert(
            "mem".into(),
            crate::json::parse(r#"[{"A":1,"W":8,"role":"in"}]"#).unwrap(),
        );
        let Value::Obj(layout) = object.layout.as_mut().unwrap() else {
            unreachable!()
        };
        layout.insert(
            "interface_regions".into(),
            crate::json::parse(r#"{"o":{"db":{"base":8}}}"#).unwrap(),
        );
        let mut mem = HashMap::from([("db".into(), vec![0x12u32.into(), 0x34u32.into()])]);
        let mut m = Memory::new(&object, "o", &mem, &4u32.into()).unwrap();
        assert_eq!(m.address("db"), Some(8));
        assert_eq!(m.load(&8u32.into(), 2, &mem), BigUint::from(0x3412u32));
        assert!(m
            .store(&8u32.into(), &0xabcdu32.into(), 2, &mut mem)
            .unwrap());
        assert_eq!(
            mem["db"],
            vec![BigUint::from(0xcdu32), BigUint::from(0xabu32)]
        );
        assert!(!m.free(&8u32.into()));
        assert!(!m
            .store(&9u32.into(), &0xffffu32.into(), 2, &mut mem)
            .unwrap());
        assert_eq!(m.load(&8u32.into(), 2, &mem), BigUint::from(0xabcdu32));
        object
            .functions
            .iter_mut()
            .find(|f| f.name == "o")
            .unwrap()
            .groups[3][0]
            .name = "g".into();
        let Value::Obj(layout) = object.layout.as_mut().unwrap() else {
            unreachable!()
        };
        layout.insert(
            "interface_regions".into(),
            crate::json::parse(r#"{"o":{"g":{"base":8}}}"#).unwrap(),
        );
        let mem = HashMap::from([("g".into(), vec![0u32.into(), 0u32.into()])]);
        assert!(
            matches!(
                Memory::new(&object, "o", &mem, &4u32.into()),
                Err(EvalError::Reject(_))
            ),
            "an interface region cannot shadow an object symbol (§9.1)"
        );
    }
    #[test]
    fn lifted_memory_rows_have_the_specified_payloads_and_fault_causes() {
        let cases=[
            ("p = lift.addr @g\nv = lift.load.i8 p\nr = lift.trunc.i4 v\nlift.retval r",4,10,0),
            ("n = lift.const.i8 1\np = lift.alloc n\nlift.free p\nlift.free p\nlift.retval x0",4,0,7),
            ("p = lift.addr @g\nlift.free p\nlift.retval x0",4,0,7),
            ("n = lift.const.i8 0\np = lift.alloc n\nv = lift.alloc n\nr = lift.trunc.i4 v\nlift.retval r",2,1,0),
            ("n = lift.const.i8 4\np = lift.alloc n\nlift.free p\np = lift.alloc n\nlift.retval x0",4,0,2),
            ("n = lift.const.i8 1\np = lift.alloc n\nlift.retval x0",0,0,2),
            ("n = lift.const.i8 0\np = lift.alloc n\nv = lift.alloc n\nw = lift.alloc n\np = lift.alloc n\nlift.retval x0",4,0,2),
            ("p = lift.const.i8 255\nn = lift.const.i8 2\nlift.boundscheck p n\nlift.retval x0",4,0,7),
        ];
        for (body, heap, payload, code) in cases {
            let mut object = object();
            let function = object.functions.iter_mut().find(|f| f.name == "o").unwrap();
            // Direct evaluator tests replace only rows, retaining the published
            // header/register types. Certificate hash checks are tested separately.
            function.bytes = format!(
                "{}\n^entry():\n{body}",
                function
                    .bytes
                    .lines()
                    .take(2)
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            let budget = [
                100u32.into(),
                BigUint::from(heap as u32),
                0u32.into(),
                0u32.into(),
            ];
            let result = crate::eval::eval_oracle_bounded(
                &object,
                object.symbol("o").unwrap(),
                &[3u32.into()],
                &[],
                &[],
                &budget,
                0,
                100,
            )
            .unwrap();
            assert_eq!(
                (result.payload, result.code),
                (BigUint::from(payload as u32), BigUint::from(code as u32)),
                "{body}"
            );
        }
    }
    #[test]
    fn malformed_memory_rows_reject_before_indexing_operands() {
        for row in [
            "p = lift.alloc",
            "lift.free",
            "lift.store.i8 p",
            "p = lift.addr g",
            "(p, v) = lift.alloc n",
            "v = lift.load.i08 p",
        ] {
            let mut object = object();
            let f = object.functions.iter_mut().find(|f| f.name == "o").unwrap();
            f.bytes = format!(
                "{}\n^entry():\n{row}\nlift.retval x0",
                f.bytes.lines().take(2).collect::<Vec<_>>().join("\n")
            );
            let result = crate::eval::eval_oracle_bounded(
                &object,
                object.symbol("o").unwrap(),
                &[3u32.into()],
                &[],
                &[],
                &[100u32.into(), 4u32.into(), 0u32.into(), 0u32.into()],
                0,
                100,
            );
            assert!(
                matches!(result,Err(EvalError::Reject(ref r))if r.code=="E_CERT_OBJECT"),
                "{row}"
            );
        }
    }
}
