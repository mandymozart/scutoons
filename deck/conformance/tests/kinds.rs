//! (a) Per node kind: the native deck's engine against the browser deck – live-core after `lower()` – block patterns 128 /
//! 37 / 1024. Since P6 (#320) both are deck-engine, so the bar is bit for bit; the table goes to
//! `deck/target/conformance/kinds.md` (the report of ENGINE-ALIGNMENT.md §2).
mod common;
use common::*;
use deck_native::audio::Build;
use std::fmt::Write;

#[test]
fn every_kind_through_both_decks() {
    let mut md = String::new();
    writeln!(md, "| kind | block | max abs | first differing sample | bit-exact | page nodes |").unwrap();
    writeln!(md, "|---|---|---|---|---|---|").unwrap();
    let mut wrong = Vec::new();
    for case in CASES.iter().chain(CONTROL_CASES) {
        let low = lower(case.toml);
        for &b in &PATTERNS {
            let stage = stage_render(case.toml, b, FRAMES, Build::detect());
            assert!(peak(&stage) > 1e-4, "{}: the native deck plays something", case.name);
            let core = core_render(&low.engine_toml, b, FRAMES);
            let d = diff(&stage, &core);
            let first = d.first.map_or("–".to_string(), |f| f.to_string());
            writeln!(md, "| {} | {b} | {:.3e} | {first} | {} | {} |", case.name, d.max_abs, if d.exact { "yes" } else { "NO" }, low.page.join(", ")).unwrap();
            if !d.exact || !low.page.is_empty() {
                wrong.push(format!("{}/{b}: first difference {first}, page {:?}", case.name, low.page));
            }
        }
        // the avx2 and baseline builds of the native deck agree (its own claim, §3.4)
        let base = stage_render(case.toml, 128, FRAMES, Build { batch: true, avx2: false });
        assert_eq!(hash(&base), hash(&stage_render(case.toml, 128, FRAMES, Build::detect())), "{}: native avx2 ≡ baseline", case.name);
    }
    std::fs::write(report_path("kinds.md"), &md).unwrap();
    println!("{md}");
    assert!(wrong.is_empty(), "the browser deck differs from the native deck:\n{}", wrong.join("\n"));
}
