//! The results as one Markdown table and the issue's suggested gates (#306): the same function prints the native run,
//! `compare` over every results file, and the page's table (the web crate exports it), so no second formatter exists.
use crate::{Results, BUDGET_US};
use std::collections::BTreeMap;
use std::fmt::Write;

/// one gate of the issue: what was measured, the limit, pass / fail / not measured yet
pub struct Gate {
    pub name: String,
    pub value: String,
    pub pass: Option<bool>,
}

fn ms(us: f64) -> String {
    if us >= 1000.0 {
        format!("{:.1} ms", us / 1000.0)
    } else {
        format!("{us:.0} µs")
    }
}

fn opt(v: Option<f64>) -> String {
    v.map_or("–".into(), ms)
}

/// every runtime's workloads, the runaway, the growth, the bit-equality and the sizes
pub fn markdown(all: &[Results]) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "| device | runtime | workload | frames | compile | inst. | first | p50 µs | p99 µs | max µs | mean µs | p99 % budget | units / sample | mem KB | hash |");
    let _ = writeln!(
        s,
        "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|"
    );
    for r in all {
        for rt in &r.runtimes {
            for w in &rt.workloads {
                if let Some(e) = &w.error {
                    let _ = writeln!(
                        s,
                        "| {} | {} | {} | {} | error: {} |||||||||||",
                        r.device.label,
                        rt.runtime,
                        w.name,
                        w.frames,
                        e.replace('|', "/")
                    );
                    continue;
                }
                let _ = writeln!(
                    s,
                    "| {} | {} | {} | {} | {} | {} | {:.0} | {:.1} | {:.1} | {:.0} | {:.1} | {:.1} % | {} | {} | {} |",
                    r.device.label,
                    rt.runtime,
                    w.name,
                    w.frames,
                    ms(w.compile_us),
                    ms(w.instantiate_us),
                    w.first_us,
                    w.p50_us,
                    w.p99_us,
                    w.max_us,
                    w.mean_us,
                    w.p99_pct,
                    w.units.map_or("–".into(), |u| format!("{u:.0}")),
                    w.memory_bytes / 1024,
                    &w.hash[..w.hash.len().min(8)]
                );
            }
        }
    }
    let _ = writeln!(s, "\n| device | runtime | timer µs | runaway stopped by | budget | stopped after | past budget | show restart | grow results (pages / -1) | memory KB before → after | grow fault |");
    let _ = writeln!(s, "|---|---|---|---|---|---|---|---|---|---|---|");
    for r in all {
        for rt in &r.runtimes {
            let (how, budget, after, over, restart) = match &rt.runaway {
                Some(x) => (
                    x.how.clone(),
                    format!("{} ms", x.budget_ms),
                    opt(x.stopped_after_us),
                    opt(x.over_budget_us),
                    opt(x.restart_us),
                ),
                None => ("–".into(), "–".into(), "–".into(), "–".into(), "–".into()),
            };
            let (gr, mem, fault) = match &rt.grow {
                Some(g) => (
                    g.results
                        .iter()
                        .map(|v| format!("{v:.0}"))
                        .collect::<Vec<_>>()
                        .join(" "),
                    format!("{} → {}", g.memory_before / 1024, g.memory_after / 1024),
                    g.fault.clone().unwrap_or_else(|| "none".into()),
                ),
                None => ("–".into(), "–".into(), "–".into()),
            };
            let _ = writeln!(s, "| {} | {} | {:.1} | {how} | {budget} | {after} | {over} | {restart} | {gr} | {mem} | {} |", r.device.label, rt.runtime, rt.timer_us, fault.replace('|', "/"));
            if let Some(e) = &rt.error {
                let _ = writeln!(
                    s,
                    "| {} | {} | error: {} |||||||||",
                    r.device.label,
                    rt.runtime,
                    e.replace('|', "/")
                );
            }
        }
    }
    // bit equality: per workload, the runtimes grouped by their hash
    let mut by: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
    for r in all {
        for rt in &r.runtimes {
            for w in rt.workloads.iter().filter(|w| w.error.is_none()) {
                by.entry(format!("{} / {}", w.name, w.frames))
                    .or_default()
                    .entry(w.hash.clone())
                    .or_default()
                    .push(format!("{}:{}", r.device.label, rt.runtime));
            }
        }
    }
    let _ = writeln!(s, "\nBit equality (every output sample's bits hashed):");
    for (w, hashes) in &by {
        if hashes.len() == 1 {
            let n = hashes.values().next().map_or(0, |v| v.len());
            let _ = writeln!(s, "- {w}: equal on all {n} runs");
        } else {
            let groups: Vec<String> = hashes
                .iter()
                .map(|(h, who)| format!("{} = {}", &h[..8], who.join(", ")))
                .collect();
            let _ = writeln!(s, "- {w}: DIFFERENT – {}", groups.join(" · "));
        }
    }
    if let Some(sizes) = all.iter().find(|r| !r.sizes.is_empty()).map(|r| &r.sizes) {
        let _ = writeln!(s, "\n| file | bytes | gz |\n|---|---|---|");
        for z in sizes {
            let _ = writeln!(s, "| {} | {} | {} |", z.name, z.bytes, z.gz);
        }
    }
    let _ = writeln!(s, "\nGates (#306, suggested):");
    for g in gates(all) {
        let mark = match g.pass {
            Some(true) => "PASS",
            Some(false) => "FAIL",
            None => "open",
        };
        let _ = writeln!(s, "- [{mark}] {}: {}", g.name, g.value);
    }
    s
}

fn workload<'a>(
    r: &'a Results,
    runtime: &str,
    name: &str,
    frames: u32,
) -> Option<&'a crate::Workload> {
    r.runtimes
        .iter()
        .find(|x| x.runtime == runtime)?
        .workloads
        .iter()
        .find(|w| w.name == name && w.frames == frames && w.error.is_none())
}

/// the issue's suggested gates: FM16 p99 ≤ 25 % of the budget on the S22 (the browser's default runtime), the gas
/// instrumentation's overhead ≤ 1.5×, wasmi's growth of the web bundle ≤ 150 KB gz
pub fn gates(all: &[Results]) -> Vec<Gate> {
    let mut out = Vec::new();
    let phone = all
        .iter()
        .find(|r| r.device.label.to_lowercase().contains("s22"));
    let fm = |r: &Results, rt: &str| workload(r, rt, "fm16", 128).map(|w| w.p99_pct);
    match phone.and_then(|r| fm(r, "web")) {
        Some(p) => out.push(Gate {
            name: "FM16 p99 ≤ 25 % of the block budget on the S22 (browser WebAssembly)".into(),
            value: format!("{p:.1} %"),
            pass: Some(p <= 25.0),
        }),
        None => out.push(Gate {
            name: "FM16 p99 ≤ 25 % of the block budget on the S22 (browser WebAssembly)".into(),
            value: "to be filled from the S22".into(),
            pass: None,
        }),
    }
    for r in all {
        for rt in [
            "web",
            "web-gas",
            "web-loops",
            "web-wasmi",
            "wasmtime",
            "wasmi",
        ] {
            if let Some(p) = fm(r, rt) {
                out.push(Gate {
                    name: format!("  FM16 p99 % on {} / {rt}", r.device.label),
                    value: format!("{p:.1} % of {BUDGET_US:.0} µs"),
                    pass: None,
                });
            }
        }
    }
    let mut any = false;
    for r in all {
        // the gas-metered copy against the same module without it, the same worker
        for (gas, plain) in [("web-gas", "web"), ("web-loops", "web")] {
            if let (Some(g), Some(p)) = (
                workload(r, gas, "fm16", 128),
                workload(r, plain, "fm16", 128),
            ) {
                let x = g.mean_us / p.mean_us.max(1e-9);
                out.push(Gate {
                    name: format!(
                        "gas instrumentation ≤ 1.5× ({} {gas} / {plain}, fm16 mean)",
                        r.device.label
                    ),
                    value: format!("{x:.2}×"),
                    pass: Some(x <= 1.5),
                });
                any = true;
            }
        }
    }
    if !any {
        out.push(Gate {
            name: "gas instrumentation ≤ 1.5×".into(),
            value: "not measured".into(),
            pass: None,
        });
    }
    let size = |n: &str| {
        all.iter()
            .flat_map(|r| r.sizes.iter())
            .find(|z| z.name == n)
            .map(|z| z.gz)
    };
    match (size("bench-web.wasm"), size("bench-web-wasmi.wasm")) {
        (Some(a), Some(b)) => {
            let kb = (b as f64 - a as f64) / 1024.0;
            out.push(Gate {
                name: "wasmi's bundle growth ≤ 150 KB gz".into(),
                value: format!("{kb:.0} KB gz"),
                pass: Some(kb <= 150.0),
            });
        }
        _ => out.push(Gate {
            name: "wasmi's bundle growth ≤ 150 KB gz".into(),
            value: "sizes missing (run prepare)".into(),
            pass: None,
        }),
    }
    out
}
