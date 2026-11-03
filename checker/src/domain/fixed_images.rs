use super::{biguint_of_json,domain_reject,FixedRegion,HostLimit,ResolveError,MAX_ELEMENT_BITS};
use crate::{json::Value,object::Interface};
use num_bigint::BigUint;
use num_traits::{One,ToPrimitive,Zero};
use std::collections::HashSet;

pub(super) fn resolve(domain:&Value,iface:&Interface)->Result<Vec<FixedRegion>,ResolveError> {
    let Some(value)=domain.get("fixed_regions")else{return Ok(vec![]);};
    let entries=value.as_arr().ok_or_else(||domain_reject("fixed_regions must be an array".into()))?;
    let mut seen=HashSet::new();let mut out=vec![];
    for entry in entries {
        let bad=|message:&str|domain_reject(format!("fixed_regions: {message}"));
        let fields=entry.as_obj().ok_or_else(||bad("entry must be an object"))?;
        let required=["name","A","W","image","sha256"];
        if fields.len()!=required.len() || required.iter().any(|k|!fields.contains_key(*k)) {return Err(bad("entry requires exactly name, A, W, image and sha256"));}
        let name=entry.get("name").and_then(Value::as_str).ok_or_else(||bad("name must be a string"))?;
        let position=iface.regions.iter().position(|r|r.name==name).ok_or_else(||bad("name is not an interface region"))?;
        let region=&iface.regions[position];
        if region.role!="in" || !seen.insert(position) {return Err(bad("fixed regions must be distinct in-role interface regions"));}
        let a=biguint_of_json(entry.get("A").unwrap(),"fixed region A")?;
        let w=biguint_of_json(entry.get("W").unwrap(),"fixed region W")?;
        if a!=BigUint::from(region.a) || w!=BigUint::from(region.w) || region.w==0 {return Err(bad("A/W differ from the nonempty interface region shape"));}
        let image=entry.get("image").and_then(Value::as_str).ok_or_else(||bad("image must be lowercase hexadecimal"))?;
        if image.len()%2!=0 || !image.bytes().all(|c|c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) {return Err(bad("image must be an even-length lowercase hexadecimal string"));}
        let supplied_bytes=image.len()/2;
        let available_bits=BigUint::from(supplied_bytes)*8u32;
        // If A exceeds the bit length of the actual finite input size, the
        // image is certainly too short. Do not construct 2^A to discover it.
        if region.a>available_bits.bits() {return Err(bad("image byte count differs from ceil(2^A*W/8)"));}
        let bits=&w<<region.a;
        if (&bits+7u32)/8u32!=BigUint::from(supplied_bytes) {return Err(bad("image byte count differs from ceil(2^A*W/8)"));}
        let bytes:Vec<u8>=(0..supplied_bytes).map(|i|u8::from_str_radix(&image[2*i..2*i+2],16).unwrap()).collect();
        let used=(&bits%8u32).to_u32().unwrap();
        if used!=0 && bytes.last().is_some_and(|b|*b>>used!=0) {return Err(bad("unused high bits in the last image byte must be zero"));}
        let hash=entry.get("sha256").and_then(Value::as_str).ok_or_else(||bad("sha256 must be a raw-byte hash"))?;
        if !crate::is_hash_hex(hash) || crate::sha256_hex(&bytes)!=hash {return Err(bad("sha256 does not equal the independently hashed raw bytes"));}
        if bits>BigUint::from(MAX_ELEMENT_BITS) {return Err(ResolveError::HostLimit(HostLimit{quantity:format!("fixed region {name:?} has {bits} bits (cap {MAX_ELEMENT_BITS})")}));}
        let cells=1usize<<region.a;
        let mut values=vec![BigUint::zero();cells];
        for (i,value) in values.iter_mut().enumerate() {
            for bit in 0..region.w as usize {
                let offset=i*region.w as usize+bit;
                if bytes[offset/8]&(1<<(offset%8))!=0 {*value|=BigUint::one()<<bit;}
            }
        }
        out.push(FixedRegion{position,a:region.a,w:region.w,sha256:hash.into(),cells:values});
    }
    out.sort_by_key(|r|r.position);
    Ok(out)
}
