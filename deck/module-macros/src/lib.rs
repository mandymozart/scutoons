//! `#[derive(Module)]` (#280, ABI v2 #340): reads the attributes on a module's struct and writes
//! - `Describe` for the struct: the descriptor as JSON, the parameters by index, `build()` with the defaults;
//! - on wasm32: the descriptor in the `deck.module` custom section and the ABI's exports (`dm_*`, deck-module/README.md).
//!
//! **ABI v1** (the default until #343 moves the three repository modules): `#[module(name = "…")]`,
//! `#[param(range = a..b, default = x)]` on `f32` fields, `#[port(input|output, audio|control|video|event, name = "…")]`.
//!
//! **ABI v2** (`#[module(abi = 2)]`, research/ABI-V2.md): additionally `domain = "audio"|"control"`, `doc`,
//! `needs` / `wants` (`"motion|position"`), `latency`; `#[reads("story.act", …)]`; ports `cv`, `channels = n|"any"`,
//! `sidechain`, `doc`, event `names` / `props` (`"a|b"`) / `capacity`, control-output `path` / `mode` / `choices`;
//! params `curve = log`, `unit`, `choices = "saw|pulse"` (the value is the index), `doc`; `#[asset(name, kind,
//! max_bytes, doc)]`; `#[preset(name, doc, values(p = x, …))]`. It writes `v2::Describe` with the constant `Shape`
//! (the layout is a `const` of it), associated constants per port / parameter (`Self::OUT`, index in its group),
//! variable (`Self::STORY_ACT: Var`) and asset (`Self::KICK: AssetSlot`), and the v2 exports with the counting
//! allocator.
//!
//! Everything it needs is a literal in the source, so the descriptor is a constant: a host reads it without running
//! the module. A mistake is a compile error at the attribute.
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Error, Expr, ExprLit, ExprRange, Fields, Lit, LitStr, Token, UnOp};

const MAX_PARAMS_V1: usize = 128;

struct Param {
    ident: syn::Ident,
    name: String,
    min: f64,
    max: f64,
    default: f64,
    curve: Option<&'static str>,
    unit: String,
    choices: Vec<String>,
    doc: String,
    span: Span,
}

struct Port {
    name: String,
    dir: &'static str,
    kind: &'static str,
    channels: Option<Channels>,
    sidechain: bool,
    names: Vec<String>,
    props: Vec<String>,
    capacity: Option<u16>,
    path: String,
    mode: Option<&'static str>,
    choices: Vec<String>,
    doc: String,
    span: Span,
}

#[derive(Clone, Copy)]
enum Channels {
    Count(u8),
    Any,
}

struct Asset {
    name: String,
    kind: &'static str,
    max_bytes: u64,
    doc: String,
}

struct Preset {
    name: String,
    doc: String,
    values: Vec<(String, f64, Span)>,
}

struct Spec {
    name: String,
    abi: u32,
    domain: &'static str,
    doc: String,
    needs: Vec<String>,
    wants: Vec<String>,
    latency: u32,
    reads: Vec<String>,
    ports: Vec<Port>,
    params: Vec<Param>,
    others: Vec<syn::Ident>,
    assets: Vec<Asset>,
    preset: Option<Preset>,
    /// the first attribute that only ABI v2 knows, for the error under v1
    v2_only: Option<(Span, &'static str)>,
}

fn number(e: &Expr) -> Result<f64, Error> {
    match e {
        Expr::Lit(ExprLit { lit: Lit::Float(f), .. }) => f.base10_parse(),
        Expr::Lit(ExprLit { lit: Lit::Int(i), .. }) => i.base10_parse(),
        Expr::Unary(u) if matches!(u.op, UnOp::Neg(_)) => Ok(-number(&u.expr)?),
        Expr::Paren(p) => number(&p.expr),
        Expr::Group(g) => number(&g.expr),
        _ => Err(Error::new(e.span(), "a number literal is expected (the descriptor is a constant)")),
    }
}

fn is_name(s: &str) -> bool {
    !s.is_empty() && s.len() <= 24 && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') && !s.as_bytes()[0].is_ascii_digit()
}

fn snake(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// `story.act` → `STORY_ACT`
fn upper(s: &str) -> String {
    let mut out: String = s.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' }).collect();
    if out.as_bytes().first().is_some_and(|b| b.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

fn json_f(v: f64) -> String {
    format!("{v:?}")
}

/// a JSON string literal
fn js(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out += "\\\"",
            '\\' => out += "\\\\",
            '\n' => out += "\\n",
            '\r' => out += "\\r",
            '\t' => out += "\\t",
            c if (c as u32) < 0x20 => out += &format!("\\u{:04x}", c as u32),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn js_list(items: &[String]) -> String {
    format!("[{}]", items.iter().map(|s| js(s)).collect::<Vec<_>>().join(","))
}

/// `"a|b|c"` → the names (empty entries refused)
fn pipe_list(s: &LitStr) -> Result<Vec<String>, Error> {
    let v: Vec<String> = s.value().split('|').map(|x| x.trim().to_string()).collect();
    if v.iter().any(|x| x.is_empty()) {
        return Err(Error::new(s.span(), "a list is \"a|b|c\" without empty entries"));
    }
    Ok(v)
}

fn lit_str(m: &syn::meta::ParseNestedMeta) -> Result<LitStr, Error> {
    m.value()?.parse()
}

fn parse_module(a: &Attribute, spec: &mut Spec) -> Result<(), Error> {
    a.parse_nested_meta(|m| {
        let key = m.path.get_ident().map(|i| i.to_string()).unwrap_or_default();
        let span = m.path.span();
        if key != "name" && key != "abi" && spec.v2_only.is_none() {
            spec.v2_only = Some((span, "this key of #[module]"));
        }
        match key.as_str() {
            "name" => spec.name = lit_str(&m)?.value(),
            "abi" => {
                let e: Expr = m.value()?.parse()?;
                spec.abi = number(&e)? as u32;
                if spec.abi != 1 && spec.abi != 2 {
                    return Err(Error::new(e.span(), "abi = 1 or 2"));
                }
            }
            "domain" => {
                let s = lit_str(&m)?;
                spec.domain = match s.value().as_str() {
                    "audio" => "audio",
                    "control" => "control",
                    _ => return Err(Error::new(s.span(), "domain = \"audio\" or \"control\"")),
                };
            }
            "doc" => spec.doc = lit_str(&m)?.value(),
            "needs" => spec.needs = pipe_list(&lit_str(&m)?)?,
            "wants" => spec.wants = pipe_list(&lit_str(&m)?)?,
            "latency" => {
                let e: Expr = m.value()?.parse()?;
                let v = number(&e)?;
                if !(v >= 0.0 && v.fract() == 0.0 && v <= u32::MAX as f64) {
                    return Err(Error::new(e.span(), "latency = frames, a whole number ≥ 0"));
                }
                spec.latency = v as u32;
            }
            _ => return Err(m.error("module(name = \"…\") is the only key (ABI 2: abi, domain, doc, needs, wants, latency)")),
        }
        Ok(())
    })
}

fn parse_port(a: &Attribute, spec: &mut Spec) -> Result<(), Error> {
    let mut p = Port {
        name: String::new(),
        dir: "",
        kind: "",
        channels: None,
        sidechain: false,
        names: vec![],
        props: vec![],
        capacity: None,
        path: String::new(),
        mode: None,
        choices: vec![],
        doc: String::new(),
        span: a.span(),
    };
    let mut v2_key = None;
    a.parse_nested_meta(|m| {
        let id = m.path.get_ident().map(|i| i.to_string()).unwrap_or_default();
        let span = m.path.span();
        match id.as_str() {
            "input" => p.dir = "in",
            "output" => p.dir = "out",
            "audio" => p.kind = "audio",
            "control" => p.kind = "control",
            "video" => p.kind = "video",
            "event" => p.kind = "event",
            "name" => p.name = lit_str(&m)?.value(),
            "cv" => {
                p.kind = "cv";
                v2_key.get_or_insert(span);
            }
            "channels" => {
                v2_key.get_or_insert(span);
                let e: Expr = m.value()?.parse()?;
                p.channels = Some(match &e {
                    Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) if s.value() == "any" => Channels::Any,
                    _ => {
                        let n = number(&e)?;
                        if !(1.0..=32.0).contains(&n) || n.fract() != 0.0 {
                            return Err(Error::new(e.span(), "channels = 1 … 32 or \"any\""));
                        }
                        Channels::Count(n as u8)
                    }
                });
            }
            "sidechain" => {
                v2_key.get_or_insert(span);
                p.sidechain = true;
            }
            "names" => {
                v2_key.get_or_insert(span);
                p.names = pipe_list(&lit_str(&m)?)?;
            }
            "props" => {
                v2_key.get_or_insert(span);
                p.props = pipe_list(&lit_str(&m)?)?;
            }
            "capacity" => {
                v2_key.get_or_insert(span);
                let e: Expr = m.value()?.parse()?;
                let n = number(&e)?;
                if !(1.0..=512.0).contains(&n) || n.fract() != 0.0 {
                    return Err(Error::new(e.span(), "capacity = 1 … 512 records per block"));
                }
                p.capacity = Some(n as u16);
            }
            "path" => {
                v2_key.get_or_insert(span);
                p.path = lit_str(&m)?.value();
            }
            "mode" => {
                v2_key.get_or_insert(span);
                let s = lit_str(&m)?;
                p.mode = Some(match s.value().as_str() {
                    "set" => "set",
                    "add" => "add",
                    "mul" => "mul",
                    _ => return Err(Error::new(s.span(), "mode = \"set\", \"add\" or \"mul\"")),
                });
            }
            "choices" => {
                v2_key.get_or_insert(span);
                p.choices = pipe_list(&lit_str(&m)?)?;
            }
            "doc" => {
                v2_key.get_or_insert(span);
                p.doc = lit_str(&m)?.value();
            }
            _ => return Err(m.error("port(input|output, audio|control|video|event, name = \"…\")")),
        }
        Ok(())
    })?;
    if p.dir.is_empty() || p.kind.is_empty() || p.name.is_empty() {
        return Err(Error::new(a.span(), "a port needs a direction, a type and a name: #[port(output, audio, name = \"out\")]"));
    }
    if let (Some(span), None) = (v2_key, spec.v2_only) {
        spec.v2_only = Some((span, "this key of #[port]"));
    }
    spec.ports.push(p);
    Ok(())
}

fn parse_param(f: &syn::Field, a: &Attribute, spec: &mut Spec) -> Result<Param, Error> {
    let ident = f.ident.clone().expect("named");
    let (mut range, mut default, mut curve, mut unit, mut choices, mut doc) = (None, None, None, String::new(), Vec::new(), String::new());
    a.parse_nested_meta(|m| {
        let key = m.path.get_ident().map(|i| i.to_string()).unwrap_or_default();
        if key != "range" && key != "default" && spec.v2_only.is_none() {
            spec.v2_only = Some((m.path.span(), "this key of #[param]"));
        }
        match key.as_str() {
            "range" => {
                let e: Expr = m.value()?.parse()?;
                let Expr::Range(ExprRange { start: Some(s), end: Some(e), .. }) = e else {
                    return Err(m.error("range = a..b"));
                };
                // a..=b is the same: a range is inclusive in the descriptor
                range = Some((number(&s)?, number(&e)?));
            }
            "default" => {
                let e: Expr = m.value()?.parse()?;
                default = Some(number(&e)?);
            }
            "curve" => {
                let e: Expr = m.value()?.parse()?;
                let s = match &e {
                    Expr::Path(p) => p.path.get_ident().map(|i| i.to_string()).unwrap_or_default(),
                    Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) => s.value(),
                    _ => String::new(),
                };
                curve = Some(match s.as_str() {
                    "lin" => "lin",
                    "log" => "log",
                    _ => return Err(Error::new(e.span(), "curve = lin or log")),
                });
            }
            "unit" => unit = lit_str(&m)?.value(),
            "choices" => choices = pipe_list(&lit_str(&m)?)?,
            "doc" => doc = lit_str(&m)?.value(),
            _ => return Err(m.error("param(range = a..b, default = x)")),
        }
        Ok(())
    })?;
    let (min, max, default) = if !choices.is_empty() {
        if range.is_some() {
            return Err(Error::new(a.span(), "a parameter with choices has no range: its value is the index 0 … n − 1"));
        }
        if choices.len() < 2 {
            return Err(Error::new(a.span(), "choices names at least two values"));
        }
        let d = default.unwrap_or(0.0);
        if !(d >= 0.0 && d.fract() == 0.0 && (d as usize) < choices.len()) {
            return Err(Error::new(a.span(), format!("default = {d}: an index into the {} choices", choices.len())));
        }
        (0.0, (choices.len() - 1) as f64, d)
    } else {
        let (Some((min, max)), Some(default)) = (range, default) else {
            return Err(Error::new(a.span(), "a parameter needs both: #[param(range = 0.0..1.0, default = 0.5)]"));
        };
        (min, max, default)
    };
    if !(min < max && (min..=max).contains(&default)) {
        return Err(Error::new(a.span(), format!("the default {default} lies outside the range {min}..{max}")));
    }
    if curve == Some("log") && min <= 0.0 {
        return Err(Error::new(a.span(), "curve = log needs min > 0"));
    }
    let is_f32 = matches!(&f.ty, syn::Type::Path(p) if p.path.is_ident("f32"));
    if !is_f32 {
        return Err(Error::new(f.ty.span(), "a parameter is an f32"));
    }
    let name = ident.to_string();
    if !is_name(&name) {
        return Err(Error::new(ident.span(), "a parameter's name is lowercase letters, digits and _ (at most 24)"));
    }
    Ok(Param { span: ident.span(), ident, name, min, max, default, curve, unit, choices, doc })
}

fn parse_asset(a: &Attribute, spec: &mut Spec) -> Result<(), Error> {
    let (mut name, mut kind, mut max_bytes, mut doc) = (String::new(), "", 0u64, String::new());
    a.parse_nested_meta(|m| {
        match m.path.get_ident().map(|i| i.to_string()).unwrap_or_default().as_str() {
            "name" => name = lit_str(&m)?.value(),
            "kind" => {
                let s = lit_str(&m)?;
                kind = match s.value().as_str() {
                    "pcm" => "pcm",
                    "bytes" => "bytes",
                    "image" => "image",
                    _ => return Err(Error::new(s.span(), "kind = \"pcm\", \"bytes\" or \"image\"")),
                };
            }
            "max_bytes" => {
                let e: Expr = m.value()?.parse()?;
                let v = number(&e)?;
                if !(v >= 1.0 && v.fract() == 0.0 && v <= u32::MAX as f64) {
                    return Err(Error::new(e.span(), "max_bytes = a whole number of bytes above 0"));
                }
                max_bytes = v as u64;
            }
            "doc" => doc = lit_str(&m)?.value(),
            _ => return Err(m.error("asset(name = \"…\", kind = \"pcm\"|\"bytes\"|\"image\", max_bytes = n, doc = \"…\")")),
        }
        Ok(())
    })?;
    if !is_name(&name) || kind.is_empty() || max_bytes == 0 {
        return Err(Error::new(a.span(), "an asset needs a name (lowercase identifier), a kind and max_bytes"));
    }
    spec.v2_only.get_or_insert((a.span(), "#[asset]"));
    spec.assets.push(Asset { name, kind, max_bytes, doc });
    Ok(())
}

fn parse_preset(a: &Attribute, spec: &mut Spec) -> Result<(), Error> {
    let mut p = Preset { name: String::new(), doc: String::new(), values: vec![] };
    a.parse_nested_meta(|m| {
        match m.path.get_ident().map(|i| i.to_string()).unwrap_or_default().as_str() {
            "name" => p.name = lit_str(&m)?.value(),
            "doc" => p.doc = lit_str(&m)?.value(),
            "values" => m.parse_nested_meta(|v| {
                let key = v.path.get_ident().map(|i| i.to_string()).ok_or_else(|| v.error("a parameter's name"))?;
                let e: Expr = v.value()?.parse()?;
                p.values.push((key, number(&e)?, e.span()));
                Ok(())
            })?,
            _ => return Err(m.error("preset(name = \"…\", doc = \"…\", values(param = x, …))")),
        }
        Ok(())
    })?;
    if p.name.is_empty() {
        return Err(Error::new(a.span(), "the preset needs a name"));
    }
    if spec.preset.is_some() {
        return Err(Error::new(a.span(), "one factory preset per module; every other preset is a document (ABI-V2 §2)"));
    }
    spec.v2_only.get_or_insert((a.span(), "#[preset]"));
    spec.preset = Some(p);
    Ok(())
}

fn parse(input: &DeriveInput) -> Result<Spec, Error> {
    let ty = &input.ident;
    if !input.generics.params.is_empty() {
        return Err(Error::new(input.generics.span(), "a module cannot be generic"));
    }
    let Data::Struct(data) = &input.data else { return Err(Error::new(ty.span(), "Module derives on a struct")) };
    let Fields::Named(fields) = &data.fields else {
        return Err(Error::new(ty.span(), "a module's struct has named fields (or none: `struct Walk {}`)"));
    };
    let mut spec = Spec {
        name: snake(&ty.to_string()),
        abi: 1,
        domain: "audio",
        doc: String::new(),
        needs: vec![],
        wants: vec![],
        latency: 0,
        reads: vec![],
        ports: vec![],
        params: vec![],
        others: vec![],
        assets: vec![],
        preset: None,
        v2_only: None,
    };
    for a in &input.attrs {
        if a.path().is_ident("module") {
            parse_module(a, &mut spec)?;
        } else if a.path().is_ident("port") {
            parse_port(a, &mut spec)?;
        } else if a.path().is_ident("reads") {
            let list = a.parse_args_with(Punctuated::<LitStr, Token![,]>::parse_terminated)?;
            spec.reads.extend(list.iter().map(|s| s.value()));
            spec.v2_only.get_or_insert((a.span(), "#[reads]"));
        } else if a.path().is_ident("asset") {
            parse_asset(a, &mut spec)?;
        } else if a.path().is_ident("preset") {
            parse_preset(a, &mut spec)?;
        }
    }
    if !is_name(&spec.name) {
        return Err(Error::new(ty.span(), format!("module name '{}': lowercase letters, digits and _ (at most 24), not starting with a digit", spec.name)));
    }
    for f in &fields.named {
        match f.attrs.iter().find(|a| a.path().is_ident("param")) {
            Some(a) => {
                let p = parse_param(f, a, &mut spec)?;
                spec.params.push(p);
            }
            None => spec.others.push(f.ident.clone().expect("named")),
        }
    }
    Ok(spec)
}

fn expand(input: DeriveInput) -> Result<proc_macro2::TokenStream, Error> {
    let spec = parse(&input)?;
    if spec.abi == 2 {
        return v2(&input.ident, &spec);
    }
    if let Some((span, what)) = spec.v2_only {
        return Err(Error::new(span, format!("{what} is ABI 2: add `abi = 2` to #[module(…)]")));
    }
    v1(&input.ident, &spec)
}

/// ABI v1, unchanged (#280) – until #343
fn v1(ty: &syn::Ident, spec: &Spec) -> Result<proc_macro2::TokenStream, Error> {
    let (name, ports, params, others) = (&spec.name, &spec.ports, &spec.params, &spec.others);
    if let Some(p) = ports.iter().find(|p| p.dir == "in" && p.kind == "event") {
        return Err(Error::new(p.span, "events only go out of a module (v1)"));
    }
    if params.len() > MAX_PARAMS_V1 {
        return Err(Error::new(ty.span(), format!("at most {MAX_PARAMS_V1} parameters")));
    }
    unique_names(spec)?;

    // the descriptor, a constant
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".into());
    let mut json = format!("{{\"abi\":1,\"name\":\"{name}\",\"version\":\"{version}\",\"params\":[");
    json += &params
        .iter()
        .map(|p| format!("{{\"name\":\"{}\",\"min\":{},\"max\":{},\"default\":{}}}", p.name, json_f(p.min), json_f(p.max), json_f(p.default)))
        .collect::<Vec<_>>()
        .join(",");
    json += "],\"ports\":[";
    json += &ports.iter().map(|p| format!("{{\"name\":\"{}\",\"dir\":\"{}\",\"kind\":\"{}\"}}", p.name, p.dir, p.kind)).collect::<Vec<_>>().join(",");
    json += "]}";
    let json_len = json.len();
    let json_bytes = proc_macro2::Literal::byte_string(json.as_bytes());

    let count = params.len();
    let idents: Vec<_> = params.iter().map(|p| &p.ident).collect();
    let idx: Vec<usize> = (0..count).collect();
    let (mins, maxs, defaults) = (
        params.iter().map(|p| p.min as f32).collect::<Vec<_>>(),
        params.iter().map(|p| p.max as f32).collect::<Vec<_>>(),
        params.iter().map(|p| p.default as f32).collect::<Vec<_>>(),
    );

    Ok(quote! {
        impl ::deck_module::Describe for #ty {
            const NAME: &'static str = #name;
            const DESCRIPTOR: &'static str = #json;
            const PARAMS: usize = #count;

            fn build() -> Self {
                Self { #(#idents: #defaults,)* #(#others: ::core::default::Default::default(),)* }
            }

            fn set_param(&mut self, index: usize, value: f32) {
                if !value.is_finite() {
                    return;
                }
                match index {
                    #(#idx => self.#idents = value.clamp(#mins, #maxs),)*
                    _ => {}
                }
            }

            fn param(&self, index: usize) -> f32 {
                match index {
                    #(#idx => self.#idents,)*
                    _ => 0.0,
                }
            }
        }

        #[cfg(target_arch = "wasm32")]
        const _: () = {
            // the descriptor, readable without running the module (deck-module::host::read_descriptor)
            #[link_section = "deck.module"]
            #[used]
            static DESCRIPTOR: [u8; #json_len] = *#json_bytes;

            static SLOT: ::deck_module::rt::Slot<#ty> = ::deck_module::rt::Slot::new();

            #[no_mangle]
            pub extern "C" fn dm_abi() -> u32 {
                ::deck_module::ABI_VERSION
            }

            /// the address of the module's `Io` (deck-module::abi)
            #[no_mangle]
            pub extern "C" fn dm_io() -> u32 {
                SLOT.io() as usize as u32
            }

            /// the descriptor's address and length, for a host that instantiates anyway
            #[no_mangle]
            pub extern "C" fn dm_descriptor_ptr() -> u32 {
                DESCRIPTOR.as_ptr() as usize as u32
            }

            #[no_mangle]
            pub extern "C" fn dm_descriptor_len() -> u32 {
                #json_len as u32
            }

            /// builds the module (rate, seed, tempo, character already in `Io::inp`)
            #[no_mangle]
            pub extern "C" fn dm_new() {
                ::deck_module::rt::install_panic_hook();
                unsafe { SLOT.new_instance() }
            }

            #[no_mangle]
            pub extern "C" fn dm_set_param(index: u32, value: f32) {
                unsafe { SLOT.set_param(index, value) }
            }

            #[no_mangle]
            pub extern "C" fn dm_process(frames: u32) {
                unsafe { SLOT.process(frames) }
            }

            #[no_mangle]
            pub extern "C" fn dm_control(frames: u32) {
                unsafe { SLOT.control(frames) }
            }
        };
    })
}

/// parameters and ports share one namespace of identifiers
fn unique_names(spec: &Spec) -> Result<(), Error> {
    let mut seen: Vec<&str> = Vec::new();
    for n in spec.params.iter().map(|p| p.name.as_str()).chain(spec.ports.iter().map(|p| p.name.as_str())) {
        if !is_name(n) {
            return Err(Error::new(Span::call_site(), format!("port name '{n}': lowercase letters, digits and _ (at most 24)")));
        }
        if seen.contains(&n) {
            return Err(Error::new(Span::call_site(), format!("the name '{n}' is used twice (parameters and ports share one namespace)")));
        }
        seen.push(n);
    }
    Ok(())
}

/// the port's group for its index constant and the layout: audio (audio + cv), control, event, video
fn group(kind: &str) -> &'static str {
    match kind {
        "audio" | "cv" => "audio",
        "control" => "control",
        "event" => "event",
        _ => "video",
    }
}

/// ABI v2 (#340, research/ABI-V2.md)
fn v2(ty: &syn::Ident, spec: &Spec) -> Result<proc_macro2::TokenStream, Error> {
    unique_names(spec)?;
    let control = spec.domain == "control";
    for p in &spec.ports {
        let event = p.kind == "event";
        let control_out = p.kind == "control" && p.dir == "out";
        let audio = group(p.kind) == "audio";
        if control && audio {
            return Err(Error::new(p.span, "a control-domain module has no audio ports (it runs in the control half, ABI-V2 §5.2)"));
        }
        if p.channels.is_some() && p.kind != "audio" {
            return Err(Error::new(p.span, "channels is for audio ports (a cv port is 2 channels)"));
        }
        if p.sidechain && !(p.kind == "audio" && p.dir == "in") {
            return Err(Error::new(p.span, "sidechain is for audio inputs"));
        }
        if !event && (!p.names.is_empty() || !p.props.is_empty() || p.capacity.is_some()) {
            return Err(Error::new(p.span, "names, props and capacity are for event ports"));
        }
        if p.props.len() > 6 {
            return Err(Error::new(p.span, "at most 6 props (deck-events' Event)"));
        }
        if !control_out && (!p.path.is_empty() || p.mode.is_some() || !p.choices.is_empty()) {
            return Err(Error::new(p.span, "path, mode and choices are for control outputs"));
        }
        if p.mode.is_some_and(|m| m != "set") && p.path.is_empty() {
            return Err(Error::new(p.span, "mode acts on a target: it needs a path"));
        }
    }
    if let Some(w) = spec.wants.iter().find(|w| spec.needs.contains(w)) {
        return Err(Error::new(Span::call_site(), format!("'{w}' is in needs and in wants – a capability is one or the other")));
    }
    for (what, list) in [("reads", &spec.reads), ("needs", &spec.needs), ("wants", &spec.wants)] {
        if let Some((_, x)) = list.iter().enumerate().find(|(i, x)| x.is_empty() || list[..*i].contains(x)) {
            return Err(Error::new(Span::call_site(), format!("{what}: '{x}' is empty or listed twice")));
        }
    }
    let mut asset_names: Vec<&str> = vec![];
    for a in &spec.assets {
        if asset_names.contains(&a.name.as_str()) {
            return Err(Error::new(Span::call_site(), format!("the asset '{}' is declared twice", a.name)));
        }
        asset_names.push(&a.name);
    }
    let mut preset_values: Vec<(usize, f64)> = vec![];
    if let Some(preset) = &spec.preset {
        for (k, v, span) in &preset.values {
            let Some((i, p)) = spec.params.iter().enumerate().find(|(_, p)| &p.name == k) else {
                return Err(Error::new(*span, format!("the preset sets '{k}', which is no parameter")));
            };
            let ok = if p.choices.is_empty() { (p.min..=p.max).contains(v) } else { *v >= 0.0 && v.fract() == 0.0 && *v <= p.max };
            if !ok {
                return Err(Error::new(*span, format!("'{k}' = {v} lies outside {} … {}", p.min, p.max)));
            }
            preset_values.push((i, *v));
        }
    }

    // --- the descriptor, a constant
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".into());
    let mut json = format!("{{\"abi\":2,\"name\":{},\"version\":{},\"domain\":{}", js(&spec.name), js(&version), js(spec.domain));
    if !spec.doc.is_empty() {
        json += &format!(",\"doc\":{}", js(&spec.doc));
    }
    for (key, list) in [("needs", &spec.needs), ("wants", &spec.wants), ("reads", &spec.reads)] {
        if !list.is_empty() {
            json += &format!(",\"{key}\":{}", js_list(list));
        }
    }
    if spec.latency > 0 {
        json += &format!(",\"latency\":{}", spec.latency);
    }
    let params: Vec<String> = spec
        .params
        .iter()
        .map(|p| {
            let mut s = format!("{{\"name\":{},\"min\":{},\"max\":{},\"default\":{}", js(&p.name), json_f(p.min), json_f(p.max), json_f(p.default));
            if let Some(c) = p.curve {
                s += &format!(",\"curve\":{}", js(c));
            }
            if !p.unit.is_empty() {
                s += &format!(",\"unit\":{}", js(&p.unit));
            }
            if !p.choices.is_empty() {
                s += &format!(",\"choices\":{}", js_list(&p.choices));
            }
            s += ",\"port\":true";
            if !p.doc.is_empty() {
                s += &format!(",\"doc\":{}", js(&p.doc));
            }
            s + "}"
        })
        .collect();
    json += &format!(",\"params\":[{}]", params.join(","));
    let ports: Vec<String> = spec
        .ports
        .iter()
        .map(|p| {
            let mut s = format!("{{\"name\":{},\"dir\":{},\"type\":{}", js(&p.name), js(p.dir), js(p.kind));
            match p.channels {
                Some(Channels::Count(n)) => s += &format!(",\"channels\":{n}"),
                Some(Channels::Any) => s += ",\"channels\":\"any\"",
                None => {}
            }
            if !p.doc.is_empty() {
                s += &format!(",\"doc\":{}", js(&p.doc));
            }
            if p.sidechain {
                s += ",\"sidechain\":true";
            }
            for (key, list) in [("names", &p.names), ("props", &p.props), ("choices", &p.choices)] {
                if !list.is_empty() {
                    s += &format!(",\"{key}\":{}", js_list(list));
                }
            }
            if let Some(c) = p.capacity {
                s += &format!(",\"capacity\":{c}");
            }
            if !p.path.is_empty() {
                s += &format!(",\"path\":{}", js(&p.path));
            }
            if let Some(m) = p.mode {
                s += &format!(",\"mode\":{}", js(m));
            }
            s + "}"
        })
        .collect();
    json += &format!(",\"ports\":[{}]", ports.join(","));
    if !spec.assets.is_empty() {
        let assets: Vec<String> = spec
            .assets
            .iter()
            .map(|a| {
                let doc = if a.doc.is_empty() { String::new() } else { format!(",\"doc\":{}", js(&a.doc)) };
                format!("{{\"name\":{},\"kind\":{}{doc},\"max_bytes\":{}}}", js(&a.name), js(a.kind), a.max_bytes)
            })
            .collect();
        json += &format!(",\"assets\":[{}]", assets.join(","));
    }
    if let Some(p) = &spec.preset {
        let doc = if p.doc.is_empty() { String::new() } else { format!(",\"doc\":{}", js(&p.doc)) };
        let values: Vec<String> = p.values.iter().map(|(k, v, _)| format!("{}:{}", js(k), json_f(*v))).collect();
        json += &format!(",\"preset\":{{\"name\":{}{doc},\"values\":{{{}}}}}", js(&p.name), values.join(","));
    }
    json += "}";
    let json_len = json.len();
    let json_bytes = proc_macro2::Literal::byte_string(json.as_bytes());

    // --- the shape, a constant (`Layout::of` evaluates it)
    let mut shape = Vec::new();
    let n_params = spec.params.len() as u16;
    let count = |dir: &str, kind: &str| spec.ports.iter().filter(|p| p.dir == dir && p.kind == kind).count() as u16;
    let (ci, co) = (count("in", "control"), count("out", "control"));
    let n_vars = spec.reads.len() as u16;
    let n_assets = spec.assets.len() as u8;
    shape.push(quote! { s.params = #n_params; s.control_in = #ci; s.control_out = #co; s.vars = #n_vars; s.assets = #n_assets; });
    for (dir, field_audio, field_events) in [("in", quote!(audio_in), quote!(events_in)), ("out", quote!(audio_out), quote!(events_out))] {
        let mut ai = 0usize;
        let mut ei = 0usize;
        for p in spec.ports.iter().filter(|p| p.dir == dir) {
            match group(p.kind) {
                "audio" => {
                    let ch: u8 = match (p.kind, p.channels) {
                        ("cv", _) | (_, None) => 2,
                        (_, Some(Channels::Count(n))) => n,
                        (_, Some(Channels::Any)) => 32,
                    };
                    shape.push(quote! { s.#field_audio[#ai] = #ch; });
                    ai += 1;
                }
                "event" => {
                    let cap = p.capacity.unwrap_or(64);
                    shape.push(quote! { s.#field_events[#ei] = #cap; });
                    ei += 1;
                }
                _ => {}
            }
        }
    }
    for (i, n) in spec.needs.iter().chain(spec.wants.iter()).enumerate() {
        let msg = format!("ABI 2 knows no capability '{n}' (motion, position, position.precise, camera.face, camera.motion)");
        shape.push(quote! {
            s.sensors[#i] = match ::deck_module::v2::Sensor::from_name(#n) {
                Some(x) => Some(x),
                None => panic!(#msg),
            };
        });
    }

    // --- parameters, constants
    let idents: Vec<_> = spec.params.iter().map(|p| &p.ident).collect();
    let idx: Vec<usize> = (0..spec.params.len()).collect();
    let defaults: Vec<f32> = spec.params.iter().map(|p| p.default as f32).collect();
    let infos = spec.params.iter().map(|p| {
        let (min, max, d) = (p.min as f32, p.max as f32, p.default as f32);
        quote! { ::deck_module::v2::ParamInfo { min: #min, max: #max, default: #d } }
    });
    let preset = preset_values.iter().map(|(i, v)| {
        let v = *v as f32;
        quote! { (#i, #v) }
    });
    let others = &spec.others;
    let name = &spec.name;
    let domain = if control { quote!(Control) } else { quote!(Audio) };

    let mut consts = vec![];
    for (i, p) in spec.params.iter().enumerate() {
        let c = format_ident!("{}", upper(&p.name), span = p.span);
        consts.push(quote! { pub const #c: usize = #i; });
    }
    for dir in ["in", "out"] {
        for g in ["audio", "control", "event", "video"] {
            for (i, p) in spec.ports.iter().filter(|p| p.dir == dir && group(p.kind) == g).enumerate() {
                let c = format_ident!("{}", upper(&p.name), span = p.span);
                consts.push(quote! { pub const #c: usize = #i; });
            }
        }
    }
    for (i, r) in spec.reads.iter().enumerate() {
        let c = format_ident!("{}", upper(r));
        let i = i as u16;
        consts.push(quote! { pub const #c: ::deck_module::v2::Var = ::deck_module::v2::Var(#i); });
    }
    for (i, a) in spec.assets.iter().enumerate() {
        let c = format_ident!("{}", upper(&a.name));
        let i = i as u8;
        consts.push(quote! { pub const #c: ::deck_module::v2::AssetSlot = ::deck_module::v2::AssetSlot(#i); });
    }

    let call = if control {
        quote! {
            /// one control block (`InHdr` and the regions written by the host)
            #[no_mangle]
            pub extern "C" fn dm_control() {
                unsafe { RT.get().call(IO.ptr()) }
            }
        }
    } else {
        quote! {
            /// one audio block (`InHdr` and the regions written by the host)
            #[no_mangle]
            pub extern "C" fn dm_process() {
                unsafe { RT.get().call(IO.ptr()) }
            }
        }
    };
    let asset_buf = if spec.assets.is_empty() {
        quote! {}
    } else {
        quote! {
            /// a buffer for asset `slot` (`len` bytes) in the module's heap, at instantiation; 0 = refused
            #[no_mangle]
            pub extern "C" fn dm_asset_buf(slot: u32, len: u32) -> u32 {
                unsafe { RT.get().asset_buf(slot as usize, len as usize) as usize as u32 }
            }
        }
    };

    Ok(quote! {
        impl ::deck_module::v2::Describe for #ty {
            const NAME: &'static str = #name;
            const DESCRIPTOR: &'static str = #json;
            const DOMAIN: ::deck_module::v2::Domain = ::deck_module::v2::Domain::#domain;
            const SHAPE: ::deck_module::v2::Shape = {
                let mut s = ::deck_module::v2::Shape::EMPTY;
                #(#shape)*
                s
            };
            const PARAMS: &'static [::deck_module::v2::ParamInfo] = &[#(#infos),*];
            const PRESET: &'static [(usize, f32)] = &[#(#preset),*];

            fn build() -> Self {
                Self { #(#idents: #defaults,)* #(#others: ::core::default::Default::default(),)* }
            }

            fn load_params(&mut self, table: &[f32], dirty: ::deck_module::v2::Dirty) {
                for i in dirty.iter() {
                    let Some(&v) = table.get(i) else { continue };
                    if !v.is_finite() {
                        continue;
                    }
                    match i {
                        #(#idx => self.#idents = v,)*
                        _ => {}
                    }
                }
            }

            fn param(&self, index: usize) -> f32 {
                match index {
                    #(#idx => self.#idents,)*
                    _ => 0.0,
                }
            }
        }

        #[allow(dead_code)]
        impl #ty {
            #(#consts)*
        }

        // the layout is evaluated at compile time: a shape outside the ABI's limits is an error here
        const _: ::deck_module::v2::Layout = <#ty as ::deck_module::v2::Describe>::LAYOUT;

        #[cfg(target_arch = "wasm32")]
        const _: () = {
            // the descriptor, readable without running the module (deck-module::host::read_descriptor)
            #[link_section = "deck.module"]
            #[used]
            static DESCRIPTOR: [u8; #json_len] = *#json_bytes;

            // allocations inside a call are counted (strict: a trap) – ABI-V2 §6
            #[global_allocator]
            static ALLOC: ::deck_module::v2::alloc::Counting = ::deck_module::v2::alloc::Counting;

            static IO: ::deck_module::v2::rt::IoBuf<{ <#ty as ::deck_module::v2::Describe>::LAYOUT.size as usize }> = ::deck_module::v2::rt::IoBuf::new();
            static RT: ::deck_module::v2::rt::Slot<#ty> = ::deck_module::v2::rt::Slot::new();

            #[no_mangle]
            pub extern "C" fn dm_abi() -> u32 {
                ::deck_module::v2::ABI_VERSION
            }

            /// the address of the module's io: `inp` at 0, `out` at `Layout::out`
            #[no_mangle]
            pub extern "C" fn dm_io() -> u32 {
                IO.ptr() as usize as u32
            }

            /// the layout's hash: the host refuses a module whose layout it computes differently
            #[no_mangle]
            pub extern "C" fn dm_layout() -> u32 {
                <#ty as ::deck_module::v2::Describe>::LAYOUT.hash
            }

            /// the descriptor's address and length, for a host that instantiates anyway
            #[no_mangle]
            pub extern "C" fn dm_descriptor_ptr() -> u32 {
                DESCRIPTOR.as_ptr() as usize as u32
            }

            #[no_mangle]
            pub extern "C" fn dm_descriptor_len() -> u32 {
                #json_len as u32
            }

            /// builds the module: the host has written `inp` (rate, seed, tempo, character, the parameter table with
            /// every entry dirty)
            #[no_mangle]
            pub extern "C" fn dm_new() {
                ::deck_module::rt::install_panic_hook();
                unsafe { RT.get().create(IO.ptr()) }
            }

            #call
            #asset_buf
        };
    })
}

#[proc_macro_derive(Module, attributes(module, param, port, reads, asset, preset))]
pub fn derive_module(input: TokenStream) -> TokenStream {
    match expand(parse_macro_input!(input as DeriveInput)) {
        Ok(t) => t.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
