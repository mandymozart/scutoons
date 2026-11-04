//! The loader's side of the ABI (feature `host`): the descriptor a module declares, found in the wasm without running
//! it, parsed into deck-catalog's one `Descriptor` (#339: the type the built-ins use too) and validated into the
//! shape its memory layout is derived from (`Shape::of`, ABI-V2 §1.1). Used by deck-module-host, the compile service
//! (#281) and their tests.
use crate::layout::ABI_VERSION;
use deck_catalog::{Descriptor, PortType};

/// the custom section the derive writes the descriptor into
pub const SECTION: &str = "deck.module";

fn ident(s: &str) -> bool {
    !s.is_empty() && s.len() <= 24 && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') && !s.as_bytes()[0].is_ascii_digit()
}

/// Parses a module's descriptor (the `deck.module` JSON) into the catalog's one `Descriptor` and [`validate`]s it.
pub fn parse(json: &str) -> Result<Descriptor, String> {
    let d: Descriptor = serde_json::from_str(json).map_err(|e| format!("descriptor: {e}"))?;
    validate(&d)?;
    Ok(d)
}

/// A module's descriptor against this host: the ABI version, the names and ranges, a domain a module can have, no audio port on a
/// control-domain module, the limits of the derived layout (`Shape::of`: ports, channels, capacities, the closed sensor
/// vocabulary), and the catalog's rules (`Descriptor::check`). Returns the shape the layout is computed from.
pub fn validate(d: &Descriptor) -> Result<crate::Shape, String> {
    if d.abi != Some(ABI_VERSION) {
        let abi = d.abi.map_or("no ABI".to_string(), |a| format!("ABI {a}"));
        return Err(format!("module '{}' speaks {abi}, this host speaks {ABI_VERSION} – recompile it against this SDK", d.name));
    }
    names_and_ranges(d)?;
    match d.domain {
        deck_catalog::Domain::Audio => {}
        deck_catalog::Domain::Control => {
            if let Some(p) = d.ports.iter().find(|p| matches!(p.ty, PortType::Audio | PortType::Cv)) {
                return Err(format!("module '{}': a control-domain module has no audio port ('{}')", d.name, p.name));
            }
        }
        deck_catalog::Domain::Video => return Err(format!("module '{}': the domain is audio or control", d.name)),
    }
    let shape = crate::Shape::of(d)?;
    d.check()?;
    Ok(shape)
}

/// the module's, parameters' and ports' names are identifiers in one namespace; every parameter has
/// min < max and a default inside
fn names_and_ranges(d: &Descriptor) -> Result<(), String> {
    if !ident(&d.name) {
        return Err(format!("module name '{}' is not a lowercase identifier (a-z, 0-9, _; at most 24)", d.name));
    }
    let mut names: Vec<&str> = Vec::new();
    for p in d.params.iter() {
        if !ident(&p.name) {
            return Err(format!("parameter name '{}' is not a lowercase identifier", p.name));
        }
        if !(p.min.is_finite() && p.max.is_finite() && p.min < p.max && p.default.is_some_and(|v| (p.min..=p.max).contains(&v))) {
            return Err(format!("parameter '{}': needs min < max and a default inside", p.name));
        }
        names.push(&p.name);
    }
    for p in d.ports.iter() {
        if !ident(&p.name) {
            return Err(format!("port name '{}' is not a lowercase identifier", p.name));
        }
        names.push(&p.name);
    }
    names.sort_unstable();
    if let Some(w) = names.windows(2).find(|w| w[0] == w[1]) {
        return Err(format!("module '{}': the name '{}' is used twice (parameters and ports share one namespace)", d.name, w[0]));
    }
    Ok(())
}

/// The `deck.module` custom section of a wasm binary (the descriptor's JSON), without instantiating it.
pub fn read_descriptor(wasm: &[u8]) -> Option<&str> {
    fn leb(b: &[u8], at: &mut usize) -> Option<usize> {
        let (mut v, mut shift) = (0usize, 0);
        loop {
            let byte = *b.get(*at)?;
            *at += 1;
            v |= usize::from(byte & 0x7f).checked_shl(shift)?;
            if byte & 0x80 == 0 {
                return Some(v);
            }
            shift += 7;
            if shift > 28 {
                return None;
            }
        }
    }
    if wasm.get(..4)? != b"\0asm" {
        return None;
    }
    let mut at = 8;
    while at < wasm.len() {
        let id = wasm[at];
        at += 1;
        let size = leb(wasm, &mut at)?;
        let end = at.checked_add(size)?;
        if id == 0 {
            let mut p = at;
            let n = leb(wasm, &mut p)?;
            let name = wasm.get(p..p.checked_add(n)?)?;
            if name == SECTION.as_bytes() {
                return core::str::from_utf8(wasm.get(p + n..end)?).ok();
            }
        }
        at = end;
    }
    None
}
