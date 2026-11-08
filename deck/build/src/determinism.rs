//! What the service pins so that a module's native run equals its wasm run, and the same source gives the same file
//! (research/ABI-V2.md §6, #344). Three things, all written by the service, never by the member:
//! - **`clippy.toml`** next to the member's `Cargo.toml`: `disallowed-methods` for the float functions of `std` whose
//!   results are the platform's libm (`f32::sin` …: wasm32 links a deterministic one, a native test run uses the
//!   machine's). The service runs `cargo clippy` after the build and hands the `clippy::disallowed_methods` warnings on
//!   (the creator's Problems panel), each saying what to call instead – `deck_module::math`, the SDK's libm. A warning,
//!   not an error: the wasm itself is correct, only its native `testing::Rig` run would differ.
//! - **pinned RUSTFLAGS** (`CARGO_ENCODED_RUSTFLAGS`, replacing whatever the environment had): the generic cpu and no
//!   SIMD and no relaxed SIMD, whatever the toolchain defaults become (threads need `atomics`, off on wasm32-unknown-unknown:
//!   stable rustc cannot even name it in `-Ctarget-feature`, it warns). The deck's wasmtime refuses a module using relaxed
//!   SIMD or threads at load anyway (module-host, and so the strict run); this keeps the compiler from emitting them.
//! - the profile in `compile::manifest` (fat LTO, `panic = "abort"` …).
use std::fmt::Write;

/// What a disallowed method is replaced by (the text of the warning's note)
enum Use {
    /// `deck_module::math::<name>` (f32; the f64 functions are `deck_module::math::libm::<name>`)
    Math(&'static str),
    /// a libm function the SDK re-exports as `deck_module::math::libm::<name>`
    Libm(&'static str),
    /// words
    Say(&'static str),
}

/// `std`'s float methods whose result depends on the platform (the transcendentals and their relatives), by method name
const METHODS: &[(&str, Use)] = &[
    ("sin", Use::Math("sin")),
    ("cos", Use::Math("cos")),
    ("tan", Use::Math("tan")),
    ("asin", Use::Math("asin")),
    ("acos", Use::Math("acos")),
    ("atan", Use::Math("atan")),
    ("atan2", Use::Math("atan2")),
    ("sinh", Use::Math("sinh")),
    ("cosh", Use::Math("cosh")),
    ("tanh", Use::Math("tanh")),
    ("exp", Use::Math("exp")),
    ("exp2", Use::Math("exp2")),
    ("ln", Use::Math("ln")),
    ("log2", Use::Math("log2")),
    ("log10", Use::Math("log10")),
    ("powf", Use::Math("pow")),
    ("cbrt", Use::Math("cbrt")),
    ("sin_cos", Use::Say("sin and cos from deck_module::math")),
    ("powi", Use::Say("pow from deck_module::math (or multiply)")),
    ("log", Use::Say("ln(x) / ln(base) from deck_module::math")),
    ("hypot", Use::Libm("hypot")),
    ("exp_m1", Use::Libm("expm1")),
    ("ln_1p", Use::Libm("log1p")),
    ("asinh", Use::Libm("asinh")),
    ("acosh", Use::Libm("acosh")),
    ("atanh", Use::Libm("atanh")),
];

/// the text after "use" in a warning: `float` is `f32` or `f64`
fn advice(float: &str, using: &Use) -> String {
    let single = float == "f32";
    match using {
        Use::Math(name) if single => format!("deck_module::math::{name} (libm: the same bits native and wasm)"),
        Use::Math(name) => format!("deck_module::math::libm::{} (libm: the same bits native and wasm)", if *name == "ln" { "log" } else { name }),
        Use::Libm(name) if single => format!("deck_module::math::libm::{name}f"),
        Use::Libm(name) => format!("deck_module::math::libm::{name}"),
        Use::Say(words) => format!("{words} (deck_module::math is libm: the same bits native and wasm)"),
    }
}

/// The `clippy.toml` the service writes beside every member's `Cargo.toml`
pub fn clippy_toml() -> String {
    let mut out = String::from(
        "# written by the module build service (#344, ABI-V2 §6): std's float functions are the platform's libm – wasm32 links\n\
         # a deterministic one, a native test run uses the machine's. deck_module::math (libm) gives the same bits everywhere.\n\
         disallowed-methods = [\n",
    );
    for float in ["f32", "f64"] {
        for (method, using) in METHODS {
            let _ = writeln!(out, "    {{ path = \"{float}::{method}\", reason = \"use {}\" }},", advice(float, using));
        }
    }
    out.push_str("]\n");
    out
}

/// The compiler flags every module is built with
pub const RUSTFLAGS: &[&str] = &["-Ctarget-cpu=generic", "-Ctarget-feature=-simd128,-relaxed-simd"];

/// `RUSTFLAGS` as cargo's `CARGO_ENCODED_RUSTFLAGS` wants them (separated by 0x1f)
pub fn encoded_rustflags() -> String {
    RUSTFLAGS.join("\u{1f}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_float_transcendental_is_listed_for_both_floats() {
        let toml = clippy_toml();
        for float in ["f32", "f64"] {
            for method in ["sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sinh", "cosh", "tanh", "exp", "exp2", "ln", "log2", "log10", "powf", "powi", "log", "cbrt", "hypot"] {
                assert!(toml.contains(&format!("path = \"{float}::{method}\"")), "{float}::{method}");
            }
        }
        // exact operations stay allowed: sqrt, floor, abs, mul_add are correctly rounded everywhere
        for method in ["sqrt", "floor", "ceil", "round", "trunc", "abs", "mul_add", "min", "max", "clamp"] {
            assert!(!toml.contains(&format!("::{method}\"")), "{method} is exact and allowed");
        }
    }

    #[test]
    fn each_reason_says_what_to_call_instead() {
        let toml = clippy_toml();
        assert!(toml.contains("path = \"f32::sin\", reason = \"use deck_module::math::sin (libm"), "{toml}");
        assert!(toml.contains("path = \"f32::powf\", reason = \"use deck_module::math::pow (libm"), "{toml}");
        assert!(toml.contains("path = \"f64::sin\", reason = \"use deck_module::math::libm::sin (libm"), "{toml}");
        assert!(toml.contains("path = \"f64::ln\", reason = \"use deck_module::math::libm::log (libm"), "{toml}");
        assert!(toml.contains("path = \"f32::hypot\", reason = \"use deck_module::math::libm::hypotf\""), "{toml}");
        assert!(toml.contains("path = \"f64::exp_m1\", reason = \"use deck_module::math::libm::expm1\""), "{toml}");
    }

    /// the text is TOML: one array of tables with a path and a reason each, ending properly
    #[test]
    fn the_file_is_one_array_of_path_and_reason() {
        let toml = clippy_toml();
        let entries: Vec<&str> = toml.lines().filter(|l| l.starts_with("    {")).collect();
        assert_eq!(entries.len(), METHODS.len() * 2);
        for e in &entries {
            assert!(e.starts_with("    { path = \"f") && e.contains("\", reason = \"use ") && e.ends_with("\" },"), "{e}");
        }
        assert!(toml.lines().any(|l| l == "disallowed-methods = [") && toml.trim_end().ends_with("\n]"));
    }

    /// what the warnings point to exists in the SDK (deck_module::math, and the libm it re-exports)
    #[test]
    fn the_suggested_functions_exist() {
        let math = include_str!("../../module/src/math.rs");
        for (_, using) in METHODS {
            if let Use::Math(name) = using {
                assert!(math.contains(&format!("pub fn {name}(")) || math.contains(&format!("{name} =>")), "deck_module::math::{name}");
            }
        }
        // the libm functions the f32 / f64 advice names (a rename in libm breaks this test, not a creator's hint)
        use deck_module::math::libm;
        let _: [fn(f32, f32) -> f32; 1] = [libm::hypotf];
        let _: [fn(f32) -> f32; 5] = [libm::expm1f, libm::log1pf, libm::asinhf, libm::acoshf, libm::atanhf];
        let _: [fn(f64) -> f64; 7] = [libm::sin, libm::log, libm::expm1, libm::log1p, libm::asinh, libm::acosh, libm::atanh];
    }

    #[test]
    fn the_flags_are_pinned_and_encoded_for_cargo() {
        assert_eq!(encoded_rustflags(), "-Ctarget-cpu=generic\u{1f}-Ctarget-feature=-simd128,-relaxed-simd");
        assert!(RUSTFLAGS.iter().any(|f| f.contains("-relaxed-simd") && f.contains("-simd128")));
    }

    /// deck/os/modules.sh builds the X1's modules (#406) with the flags the service pins: the script's copy equals this file's
    #[test]
    fn os_modules_script_pins_the_service_s_flags() {
        let script = include_str!("../../os/modules.sh");
        let wanted = format!("CARGO_ENCODED_RUSTFLAGS=$'{}'", RUSTFLAGS.join("\\x1f"));
        assert!(script.contains(&wanted), "deck/os/modules.sh must export {wanted}");
    }
}
