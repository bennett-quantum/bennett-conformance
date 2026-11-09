//! Static image-launch/1 template checks cannot attest a future runtime copy.
use super::*;
pub(super) fn check(
    entry: &Value,
    cert: &Value,
    docs: &LoadedDocs,
    libraries: &[Value],
    images: &[FixedRegion],
) -> Result<String> {
    let witness = get(entry, "witness")?;
    let wanted = hash(get(witness, "manifest_hash")?)?;
    let manifest = libraries
        .iter()
        .find(|v| crate::sha256_hex(&jcs(v).expect("loaded JCS document")) == wanted)
        .ok_or_else(|| {
            Error::Unavailable(
                "launch manifest was not supplied; applicability unestablished".into(),
            )
        })?;
    let version = get(manifest, "version")?
        .as_str()
        .ok_or_else(|| invalid("launch manifest version must be a string"))?;
    if version != "bennett.image-launch/1" {
        return Err(Error::Unavailable(
            "unsupported launch manifest version; applicability unestablished".into(),
        ));
    }
    keys(
        manifest,
        &["version", "bindings", "copies"],
        &["version", "bindings", "copies"],
    )?;
    let fields = [
        "object_hash",
        "interface_hash",
        "closure_hash",
        "target_hash",
        "schedule_hash",
        "invocation",
    ];
    let bindings = get(manifest, "bindings")?;
    keys(bindings, &fields, &fields)?;
    let root = get(cert, "bindings")?;
    for key in &fields[..5] {
        hash(get(bindings, key)?)?;
        if let Some(bound) = root.get(key) {
            if get(bindings, key)? != bound {
                return Err(invalid(format!(
                    "launch manifest {key} differs from the selected device/root binding"
                )));
            }
        }
    }
    if get(bindings, "object_hash")?.as_str() != Some(docs.object.object_hash.as_str()) {
        return Err(invalid("launch manifest names another device object"));
    }
    if let Some(root_inv) = root.get("invocation") {
        if get(bindings, "invocation")? != root_inv {
            return Err(invalid("launch manifest root invocation differs"));
        }
    }
    let copies = array(get(manifest, "copies")?)?;
    let mut previous = None;
    for copy in copies {
        let fields = ["context_key", "mem_slot", "A", "W", "image", "sha256"];
        keys(copy, &fields, &fields)?;
        schema::context(get(copy, "context_key")?, &docs.object)?;
        let key = (
            jcs(get(copy, "context_key")?).expect("context JSON"),
            schema::uint(get(copy, "mem_slot")?)?,
        );
        if previous.as_ref().is_some_and(|p| p >= &key) {
            return Err(invalid(
                "launch copies must be sorted with unique context/mem-slot pairs",
            ));
        }
        previous = Some(key);
        schema::uint(get(copy, "A")?)?;
        schema::uint(get(copy, "W")?)?;
        hash(get(copy, "sha256")?)?;
        let context = get(copy, "context_key")?;
        let symbol = if array(context)?.is_empty() {
            docs.object
                .symbol_by_closure(&docs.root_closure)
                .ok_or_else(|| invalid("launch root is absent"))?
        } else {
            return Err(Error::Unavailable("nested launch-copy context resolution remains unavailable; applicability unestablished".into()));
        };
        let iface = docs
            .object
            .interface(&symbol.name)
            .map_err(Error::Unavailable)?;
        let slot = index(get(copy, "mem_slot")?)?;
        let region = iface
            .regions
            .get(slot)
            .ok_or_else(|| invalid("launch copy mem slot is absent from its reached subject"))?;
        let mut raw = copy.as_obj().expect("validated copy object").clone();
        raw.remove("context_key");
        raw.remove("mem_slot");
        raw.insert("name".into(), s(&region.name));
        super::images(&Value::Arr(vec![Value::Obj(raw)]), &iface, true)?;
    }
    for ((slot, image), raw) in array(get(witness, "copy_indices")?)?
        .iter()
        .zip(images)
        .zip(array(get(entry, "images")?)?)
    {
        let copy = copies
            .get(index(slot)?)
            .ok_or_else(|| invalid("copy index is out of range"))?;
        if get(copy, "context_key")? != get(entry, "context_key")?
            || index(get(copy, "mem_slot")?)? != image.position
        {
            return Err(invalid("selected copy names another context or mem slot"));
        }
        for key in ["A", "W", "image", "sha256"] {
            if get(copy, key)? != get(raw, key)? {
                return Err(invalid(format!("selected launch copy has different {key}")));
            }
        }
    }
    Err(Error::Unavailable("image-launch template checked; the launch provider's fresh clean factor, performed copy, opaque-handle association and mutation exclusion cannot be attested by a static checker; applicability unestablished".into()))
}
