//! `variants` (#396, research/WASM-RUNTIME-PAPER.md): the suite's workloads on wasmtime with one setting changed from
//! the deck's (`WasmtimeEngine::deck_config`) or another build of the modules (`.nan`: the NaN pass, `.simd`: simd128),
//! every pair `rounds` times interleaved (the median of the means), each output hash checked against the deck's and
//! against the browser's (results/pc-chrome.json `web`). EXPERIMENTS: a variant that changes float semantics is never a
//! default without the owner's decision.
use anyhow::{anyhow, bail, Result};
use deck_module::QUANTUM;
use deck_module_host::wasmtime_backend::{Wasmtime, WasmtimeEngine};
use deck_module_host::Loaded;
use module_bench::{native, run_suite, Config, Modules, Results, Workload};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;
use wasmtime::OptLevel;

pub struct Variant {
    pub name: &'static str,
    /// the module build: "" plain, ".nan", ".simd"
    pub modules: &'static str,
    pub what: &'static str,
    set: fn(&mut wasmtime::Config),
}

fn nan_off(c: &mut wasmtime::Config) {
    c.cranelift_nan_canonicalization(false);
}

fn epoch_off(c: &mut wasmtime::Config) {
    c.epoch_interruption(false);
}

fn spectre_off(c: &mut wasmtime::Config) {
    // SAFETY: both are Cranelift codegen flags wasmtime accepts; off only drops the mitigation (a bench measurement)
    unsafe {
        c.cranelift_flag_set("enable_heap_access_spectre_mitigation", "false");
        c.cranelift_flag_set("enable_table_access_spectre_mitigation", "false");
    }
}

pub const VARIANTS: &[Variant] = &[
    Variant {
        name: "deck",
        modules: "",
        what: "the deck's config (NaN canonicalization, epoch)",
        set: |_| {},
    },
    Variant {
        name: "nan-off",
        modules: "",
        what: "cranelift_nan_canonicalization off",
        set: nan_off,
    },
    Variant {
        name: "epoch-off",
        modules: "",
        what: "epoch_interruption off (no budget)",
        set: epoch_off,
    },
    Variant {
        name: "spectre-off",
        modules: "",
        what: "heap + table spectre mitigation off",
        set: spectre_off,
    },
    Variant {
        name: "opt-size",
        modules: "",
        what: "cranelift_opt_level SpeedAndSize",
        set: |c| {
            c.cranelift_opt_level(OptLevel::SpeedAndSize);
        },
    },
    Variant {
        name: "nan-off+epoch-off",
        modules: "",
        what: "both off",
        set: |c| {
            nan_off(c);
            epoch_off(c);
        },
    },
    Variant {
        name: "all-off",
        modules: "",
        what: "NaN, epoch, spectre off",
        set: |c| {
            nan_off(c);
            epoch_off(c);
            spectre_off(c);
        },
    },
    Variant {
        name: "nan-pass",
        modules: ".nan",
        what: "NaN pass in the wasm, cranelift canonicalization off",
        set: nan_off,
    },
    Variant {
        name: "nan-pass+deck",
        modules: ".nan",
        what: "NaN pass and the deck's config (the pass's own cost)",
        set: |_| {},
    },
    Variant {
        name: "simd",
        modules: ".simd",
        what: "modules with simd128, the deck's config",
        set: |_| {},
    },
    Variant {
        name: "simd+nan-off",
        modules: ".simd",
        what: "modules with simd128, canonicalization off",
        set: nan_off,
    },
    Variant {
        name: "nan-light",
        modules: ".nan-light",
        what: "the NaN pass without the store points (reinterpret, copysign); canonicalization off",
        set: nan_off,
    },
    Variant {
        name: "loops",
        modules: ".loops-max",
        what: "the loop counter in the wasm instead of the epoch (canonicalization on)",
        set: epoch_off,
    },
    Variant {
        name: "nan-pass+loops",
        modules: ".nan-loops",
        what: "NaN pass + loop counter in the wasm; Cranelift canonicalization and epoch off (option C + the counter)",
        set: |c| {
            nan_off(c);
            epoch_off(c);
        },
    },
];

/// workload name with its frames → (mean µs, p50 µs, hash)
type Row = BTreeMap<String, (f64, f64, String)>;

/// one workload's means, p50s and hashes, a round each
type Rounds = (Vec<f64>, Vec<f64>, Vec<String>);

fn key(w: &Workload) -> String {
    if w.frames == 128 {
        w.name.clone()
    } else {
        format!("{} ×{}", w.name, w.frames)
    }
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

/// the browser's hashes and means from an earlier run (`web` of results/<file>)
fn reference(dir: &Path, file: &str) -> Option<Row> {
    let r: Results = serde_json::from_str(&std::fs::read_to_string(dir.join(file)).ok()?).ok()?;
    let web = r.runtimes.into_iter().find(|r| r.runtime == "web")?;
    Some(
        web.workloads
            .iter()
            .map(|w| (key(w), (w.mean_us, w.p50_us, w.hash.clone())))
            .collect(),
    )
}

pub fn run(
    results: &Path,
    label: &str,
    blocks: Option<usize>,
    rounds: usize,
    only: Option<&str>,
    load: impl Fn(&str) -> Result<Modules>,
) -> Result<String> {
    if rounds == 0 {
        bail!("--rounds 0");
    }
    let mut cfg = Config {
        runaway: false,
        grow: false,
        ..Config::default()
    };
    if let Some(n) = blocks {
        cfg.blocks = n;
        cfg.blocks_1024 = n / 8;
    }
    // `--only a,b`: those and the deck's (the reference, always the first row)
    let chosen: Vec<&Variant> = VARIANTS
        .iter()
        .filter(|v| v.name == "deck" || only.map_or(true, |o| o.split(',').any(|n| n == v.name)))
        .collect();
    let mut sets = BTreeMap::new();
    for v in &chosen {
        if !sets.contains_key(v.modules) {
            sets.insert(v.modules, load(v.modules)?);
        }
    }
    let engines: Vec<WasmtimeEngine> = chosen
        .iter()
        .map(|v| {
            let mut c = WasmtimeEngine::deck_config();
            (v.set)(&mut c);
            WasmtimeEngine::from_config(&c, None)
        })
        .collect();
    // per variant, per workload: the means and p50s of every round, the hashes seen
    let mut runs: Vec<BTreeMap<String, Rounds>> = vec![BTreeMap::new(); chosen.len() + 1];
    let index = Loaded::<Wasmtime>::load(&engines[0], &sets[""].fm16)?
        .param_index("index")
        .ok_or_else(|| anyhow!("fm16 has no 'index'"))?;
    for round in 0..rounds {
        println!("round {} · native …", round + 1);
        let q = QUANTUM as u32;
        let native = [
            native::fm16(q, cfg.blocks, index),
            native::fm16(8 * q, cfg.blocks_1024, index),
            native::analog(cfg.blocks),
        ];
        for w in &native {
            let s = runs[chosen.len()].entry(key(w)).or_default();
            s.0.push(w.mean_us);
            s.1.push(w.p50_us);
            s.2.push(w.hash.clone());
        }
        for (i, v) in chosen.iter().enumerate() {
            println!("round {} · {} …", round + 1, v.name);
            let r =
                run_suite::<Wasmtime>(&engines[i], v.name, "native", &sets[v.modules], &cfg, None);
            for w in &r.workloads {
                if let Some(e) = &w.error {
                    bail!("{} {}: {e}", v.name, w.name);
                }
                let s = runs[i].entry(key(w)).or_default();
                s.0.push(w.mean_us);
                s.1.push(w.p50_us);
                s.2.push(w.hash.clone());
            }
        }
    }
    let rows: Vec<Row> = runs
        .into_iter()
        .map(|m| {
            m.into_iter()
                .map(|(k, (means, p50s, hashes))| {
                    let h = if hashes.iter().all(|h| *h == hashes[0]) {
                        hashes[0].clone()
                    } else {
                        "unstable".into()
                    };
                    (k, (median(means), median(p50s), h))
                })
                .collect()
        })
        .collect();
    let chrome = reference(results, "pc-chrome.json");
    let node = reference(results, "pc-node.json");
    let names: Vec<String> = rows[0].keys().cloned().collect();
    let mut md = String::new();
    let _ = writeln!(md, "# wasmtime variants ({label}, #396) – median of {rounds} rounds; mean µs (p50) per sample\n");
    let _ = write!(md, "| variant | modules |");
    for n in &names {
        let _ = write!(md, " {n} |");
    }
    let _ = writeln!(md, " fm16 / analog vs deck | bits = deck | bits = Chrome |");
    let _ = writeln!(md, "|---|---|{}---|---|---|", "---|".repeat(names.len()));
    let deck = &rows[0];
    let gain = |row: &Row, w: &str| {
        deck.get(w)
            .zip(row.get(w))
            .map_or("–".into(), |(d, r)| format!("{:.2}×", r.0 / d.0))
    };
    let labels = chosen
        .iter()
        .map(|v| {
            (
                v.name,
                if v.modules.is_empty() {
                    "plain"
                } else {
                    &v.modules[1..]
                },
            )
        })
        .chain([("native Rust (no wasm)", "rlib")]);
    for ((name, modules), row) in labels.zip(&rows) {
        let _ = write!(md, "| {name} | {modules} |");
        for n in &names {
            let _ = match row.get(n) {
                Some((mean, p50, _)) => write!(md, " {mean:.1} ({p50:.1}) |"),
                None => write!(md, " – |"),
            };
        }
        let same = |other: Option<&Row>| match other {
            None => "–".to_owned(),
            Some(o) => {
                let diff: Vec<&String> = names
                    .iter()
                    .filter(|n| {
                        row.contains_key(*n) && o.get(*n).map(|x| &x.2) != row.get(*n).map(|x| &x.2)
                    })
                    .collect();
                if diff.is_empty() {
                    "yes".into()
                } else {
                    format!(
                        "NO: {}",
                        diff.iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
        };
        let _ = writeln!(
            md,
            " {} / {} | {} | {} |",
            gain(row, "fm16"),
            gain(row, "analog x2"),
            same(Some(deck)),
            same(chrome.as_ref())
        );
    }
    for (name, r) in [
        ("Chrome (pc-chrome.json, web)", &chrome),
        ("Node (pc-node.json, web)", &node),
    ] {
        if let Some(r) = r {
            let _ = write!(md, "| {name} | plain |");
            for n in &names {
                let _ = match r.get(n) {
                    Some((mean, p50, _)) => write!(md, " {mean:.1} ({p50:.1}) |"),
                    None => write!(md, " – |"),
                };
            }
            let _ = writeln!(md, " {} / {} | | |", gain(r, "fm16"), gain(r, "analog x2"));
        }
    }
    let _ = writeln!(md);
    for v in &chosen {
        let _ = writeln!(md, "- **{}**: {}", v.name, v.what);
    }
    let _ = writeln!(md, "- **native Rust**: the module crates compiled for this machine, run by `deck_module::testing::Rig` (src/native.rs)");
    Ok(md)
}
