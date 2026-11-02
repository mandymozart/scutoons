//! `#[derive(Module)]` (#280): reads `#[module(name = "…")]`, `#[param(range = a..b, default = x)]` on `f32` fields and
//! `#[port(input|output, audio|control|video|event, name = "…")]` on the struct, and writes
//! - `Describe` for the struct: the descriptor as JSON, the parameters by index, `build()` with the defaults;
//! - on wasm32: the descriptor in the `deck.module` custom section and the ABI's exports (`dm_*`, deck-module/README.md).
//!
//! Everything it needs is a literal in the source, so the descriptor is a constant: a host reads it without running
//! the module. A mistake is a compile error at the attribute.
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, DeriveInput, Error, Expr, ExprLit, ExprRange, Fields, Lit, RangeLimits, UnOp};

const MAX_PARAMS: usize = 32;

struct Param {
    ident: syn::Ident,
    name: String,
    min: f64,
    max: f64,
    default: f64,
}

struct Port {
    name: String,
    dir: &'static str,
    kind: &'static str,
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

fn json_f(v: f64) -> String {
    format!("{v:?}")
}

fn expand(input: DeriveInput) -> Result<proc_macro2::TokenStream, Error> {
    let ty = &input.ident;
    if !input.generics.params.is_empty() {
        return Err(Error::new(input.generics.span(), "a module cannot be generic"));
    }
    let Data::Struct(data) = &input.data else { return Err(Error::new(ty.span(), "Module derives on a struct")) };
    let Fields::Named(fields) = &data.fields else {
        return Err(Error::new(ty.span(), "a module's struct has named fields (or none: `struct Walk {}`)"));
    };

    let mut name = snake(&ty.to_string());
    let mut ports: Vec<Port> = Vec::new();
    for a in &input.attrs {
        if a.path().is_ident("module") {
            a.parse_nested_meta(|m| {
                if m.path.is_ident("name") {
                    let s: syn::LitStr = m.value()?.parse()?;
                    name = s.value();
                    Ok(())
                } else {
                    Err(m.error("module(name = \"…\") is the only key"))
                }
            })?;
        } else if a.path().is_ident("port") {
            let (mut dir, mut kind, mut pname) = (None, None, None);
            a.parse_nested_meta(|m| {
                let id = m.path.get_ident().map(|i| i.to_string()).unwrap_or_default();
                match id.as_str() {
                    "input" => dir = Some("in"),
                    "output" => dir = Some("out"),
                    "audio" => kind = Some("audio"),
                    "control" => kind = Some("control"),
                    "video" => kind = Some("video"),
                    "event" => kind = Some("event"),
                    "name" => {
                        let s: syn::LitStr = m.value()?.parse()?;
                        pname = Some(s.value());
                    }
                    _ => return Err(m.error("port(input|output, audio|control|video|event, name = \"…\")")),
                }
                Ok(())
            })?;
            let (Some(dir), Some(kind), Some(pname)) = (dir, kind, pname) else {
                return Err(Error::new(a.span(), "a port needs a direction, a type and a name: #[port(output, audio, name = \"out\")]"));
            };
            if dir == "in" && kind == "event" {
                return Err(Error::new(a.span(), "events only go out of a module (v1)"));
            }
            ports.push(Port { name: pname, dir, kind });
        }
    }
    if !is_name(&name) {
        return Err(Error::new(ty.span(), format!("module name '{name}': lowercase letters, digits and _ (at most 24), not starting with a digit")));
    }

    let mut params: Vec<Param> = Vec::new();
    let mut others = Vec::new();
    for f in &fields.named {
        let ident = f.ident.clone().expect("named");
        let attr = f.attrs.iter().find(|a| a.path().is_ident("param"));
        let Some(a) = attr else {
            others.push(ident);
            continue;
        };
        let (mut range, mut default) = (None, None);
        a.parse_nested_meta(|m| {
            if m.path.is_ident("range") {
                let e: Expr = m.value()?.parse()?;
                let Expr::Range(ExprRange { start: Some(s), end: Some(e), limits, .. }) = e else {
                    return Err(m.error("range = a..b"));
                };
                if matches!(limits, RangeLimits::Closed(_)) {
                    // a..=b is the same: a range is inclusive in the descriptor
                }
                range = Some((number(&s)?, number(&e)?));
            } else if m.path.is_ident("default") {
                let e: Expr = m.value()?.parse()?;
                default = Some(number(&e)?);
            } else {
                return Err(m.error("param(range = a..b, default = x)"));
            }
            Ok(())
        })?;
        let (Some((min, max)), Some(default)) = (range, default) else {
            return Err(Error::new(a.span(), "a parameter needs both: #[param(range = 0.0..1.0, default = 0.5)]"));
        };
        if !(min < max && (min..=max).contains(&default)) {
            return Err(Error::new(a.span(), format!("the default {default} lies outside the range {min}..{max}")));
        }
        let is_f32 = matches!(&f.ty, syn::Type::Path(p) if p.path.is_ident("f32"));
        if !is_f32 {
            return Err(Error::new(f.ty.span(), "a parameter is an f32"));
        }
        let pname = ident.to_string();
        if !is_name(&pname) {
            return Err(Error::new(ident.span(), "a parameter's name is lowercase letters, digits and _ (at most 24)"));
        }
        params.push(Param { ident, name: pname, min, max, default });
    }
    if params.len() > MAX_PARAMS {
        return Err(Error::new(ty.span(), format!("at most {MAX_PARAMS} parameters")));
    }
    let mut seen: Vec<&str> = Vec::new();
    for n in params.iter().map(|p| p.name.as_str()).chain(ports.iter().map(|p| p.name.as_str())) {
        if !is_name(n) {
            return Err(Error::new(Span::call_site(), format!("port name '{n}': lowercase letters, digits and _ (at most 24)")));
        }
        if seen.contains(&n) {
            return Err(Error::new(Span::call_site(), format!("the name '{n}' is used twice (parameters and ports share one namespace)")));
        }
        seen.push(n);
    }

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

#[proc_macro_derive(Module, attributes(module, param, port))]
pub fn derive_module(input: TokenStream) -> TokenStream {
    match expand(parse_macro_input!(input as DeriveInput)) {
        Ok(t) => t.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
