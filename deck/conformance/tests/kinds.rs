//! (a) Per node kind: the native deck's engine against live-core after `lower()`, block patterns 128 / 37 / 1024.
//! The test prints the table and writes it to `deck/target/conformance/kinds.md` (the report of ENGINE-ALIGNMENT.md §2).
//! It fixes only what must hold for the numbers to mean something: the stage plays the scene, the lowering is
//! reported, nothing panics. How far the engines differ is the finding, not an assertion.
mod common;
use common::*;
use deck_native::audio::Build;
use std::fmt::Write;

#[test]
fn every_kind_through_both_engines() {
    let mut md = String::new();
    writeln!(md, "| kind | block | max abs | RMS of diff | max abs after 0.25 s | level RMS stage / live-core | first differing sample | bit-exact | verdict |").unwrap();
    writeln!(md, "|---|---|---|---|---|---|---|---|---|").unwrap();
    let mut cut = String::new();
    writeln!(cut, "| kind | stage 128 = 37 = 1024 | live-core 128 = 37 = 1024 |").unwrap();
    writeln!(cut, "|---|---|---|").unwrap();
    for case in CASES {
        let stage: Vec<Vec<f32>> = PATTERNS.iter().map(|&b| stage_render(case.toml, b, FRAMES, Build::detect())).collect();
        assert!(peak(&stage[0]) > 1e-4, "{}: the stage plays something", case.name);
        let low = lower(case.toml);
        let mut verdict = String::new();
        if !low.page.is_empty() {
            write!(verdict, "missing: page node {}", low.page.join(", ")).unwrap();
        }
        if !low.dropped_params.is_empty() {
            write!(verdict, "{}lowered away: {}", if verdict.is_empty() { "" } else { "; " }, low.dropped_params.join(", ")).unwrap();
        }
        let core: Vec<Vec<f32>> = PATTERNS.iter().map(|&b| core_render(&low.engine_toml, b, FRAMES)).collect();
        let core_silent = peak(&core[0]) < 1e-6;
        for (i, &b) in PATTERNS.iter().enumerate() {
            if core_silent {
                writeln!(md, "| {} | {b} | – | – | – | {:.3} / 0 | – | – | missing (live-core is silent; stage peak {:.3}) |", case.name, rms(&stage[i]), peak(&stage[i])).unwrap();
                continue;
            }
            let d = diff(&stage[i], &core[i]);
            let first = d.first.map_or("–".to_string(), |f| f.to_string());
            writeln!(md, "| {} | {b} | {:.3e} | {:.3e} | {:.3e} | {:.3} / {:.3} | {first} | {} | {} |", case.name, d.max_abs, d.rms, diff(&stage[i][12_000..], &core[i][12_000..]).max_abs, rms(&stage[i]), rms(&core[i]), if d.exact { "yes" } else { "no" }, if i == 0 { verdict.as_str() } else { "" }).unwrap();
        }
        let same = |v: &[Vec<f32>]| if hash(&v[0]) == hash(&v[1]) && hash(&v[1]) == hash(&v[2]) { "same" } else { "differs" };
        writeln!(cut, "| {} | {} | {} |", case.name, same(&stage), if core_silent { "silent".into() } else { same(&core).to_string() }).unwrap();
        // the avx2 and baseline builds of the stage agree (the stage's own claim, §3.4)
        let base = stage_render(case.toml, 128, FRAMES, Build { batch: true, avx2: false });
        assert_eq!(hash(&base), hash(&stage[0]), "{}: stage avx2 ≡ baseline", case.name);
    }
    let text = format!("{md}\nBlock-cut sensitivity (the same engine, three block patterns):\n\n{cut}");
    std::fs::write(report_path("kinds.md"), &text).unwrap();
    println!("{text}");
}
