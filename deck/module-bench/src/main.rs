//! module-bench (#306, agents/research/MODULE-BENCH.md) – a thin entry over the library:
//!   prepare [--exe]                      the workload modules for wasm32 and their instrumented copies (`gas`), the web
//!                                        suite with and without wasmi → www/dist,
//!                                        the sizes (raw + gz) → www/dist/sizes.json; --exe also a binary that carries them
//!   run [--label pc] [--blocks N] [--post URL]   wasmtime + wasmi natively → results/<label>-native.json + .md
//!   compare [dir]                        every results/*.json as one table + the issue's gates
use anyhow::{anyhow, bail, Context, Result};
use module_bench::{report, run_suite, Config, Device, Modules, Results, Size};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

mod gas;

fn here() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn dist() -> PathBuf {
    here().join("www").join("dist")
}

fn arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

/// `rustflags`: the build's only flags (a parent cargo's never leak into it)
fn cargo(args: &[&str], target_dir: &Path, rustflags: &str) -> Result<()> {
    let mut c = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    c.current_dir(here().parent().unwrap())
        .args(args)
        .arg("--target-dir")
        .arg(target_dir);
    c.env_remove("RUSTFLAGS")
        .env("CARGO_ENCODED_RUSTFLAGS", rustflags);
    let s = c.status().context("running cargo")?;
    if !s.success() {
        bail!("cargo {} failed", args.join(" "));
    }
    Ok(())
}

fn gz(bytes: &[u8]) -> u64 {
    let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    e.write_all(bytes).unwrap();
    e.finish().unwrap().len() as u64
}

/// wasm-instrument 0.4's mutable-global injection, tried on every module for the record (#306: "check its
/// maintenance"): after a wasmparser → wasm-encoder round trip (the padded `call_indirect` table index parity-wasm
/// refuses) it still fails on the `0xFC` ops of the precompiled std – so the gas copies come from `gas::inject`
fn wasm_instrument(wasm: &[u8]) -> Result<Vec<u8>> {
    use wasm_encoder::reencode::{Reencode, RoundtripReencoder};
    use wasm_instrument::gas_metering::{self, mutable_global::Injector, ConstantCostRules};
    use wasm_instrument::parity_wasm::{deserialize_buffer, elements::Module, serialize};
    let mut round = wasm_encoder::Module::new();
    RoundtripReencoder
        .parse_core_module(&mut round, wasmparser::Parser::new(0), wasm)
        .map_err(|e| anyhow!("re-encoding: {e:?}"))?;
    let m: Module = deserialize_buffer(&round.finish())
        .map_err(|e| anyhow!("parity-wasm cannot read the module: {e}"))?;
    let m = gas_metering::inject(m, Injector::new("gas_left"), &ConstantCostRules::default())
        .map_err(|_| anyhow!("the gas injection failed"))?;
    serialize(m).map_err(|e| anyhow!("{e}"))
}

const CRATES: [(&str, &str); 4] = [
    ("fm16", "fm16_module"),
    ("walk", "walk_module"),
    ("analog", "analog_module"),
    ("stress", "stress_module"),
];

fn prepare(args: &[String]) -> Result<()> {
    let deck = here().parent().unwrap().to_owned();
    let target = deck.join("target");
    let out = dist();
    std::fs::create_dir_all(&out)?;
    let modules = [
        "build",
        "--release",
        "--target",
        "wasm32-unknown-unknown",
        "-p",
        "fm16-module",
        "-p",
        "walk-module",
        "-p",
        "analog-module",
        "-p",
        "stress-module",
    ];
    cargo(&modules, &target.join("bench-wasm"), "")?;
    for (name, file) in CRATES {
        let plain = std::fs::read(
            target
                .join("bench-wasm/wasm32-unknown-unknown/release")
                .join(format!("{file}.wasm")),
        )?;
        std::fs::write(out.join(format!("{name}.wasm")), &plain)?;
        match wasm_instrument(&plain) {
            Ok(_) => println!("{name}: wasm-instrument 0.4 instruments it"),
            Err(e) => println!("{name}: wasm-instrument 0.4 fails – {e}"),
        }
        let gas = gas::inject(&plain, gas::Scheme::Metered)
            .with_context(|| format!("{name}: the gas pass"))?;
        std::fs::write(out.join(format!("{name}.gas.wasm")), gas)?;
        let loops = gas::inject(&plain, gas::Scheme::Loops)
            .with_context(|| format!("{name}: the loop-counter pass"))?;
        std::fs::write(out.join(format!("{name}.loops.wasm")), loops)?;
    }
    let web = [
        "build",
        "--release",
        "--target",
        "wasm32-unknown-unknown",
        "-p",
        "module-bench-web",
    ];
    cargo(&web, &target.join("bench-web"), "")?;
    std::fs::copy(
        target.join("bench-web/wasm32-unknown-unknown/release/module_bench_web.wasm"),
        out.join("bench-web.wasm"),
    )?;
    cargo(
        &[&web[..], &["--features", "wasmi"]].concat(),
        &target.join("bench-web-wasmi"),
        "",
    )?;
    std::fs::copy(
        target.join("bench-web-wasmi/wasm32-unknown-unknown/release/module_bench_web.wasm"),
        out.join("bench-web-wasmi.wasm"),
    )?;
    let mut sizes = Vec::new();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&out)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "wasm"))
        .collect();
    files.sort();
    // live-core as `npm run build:wasm -w shared/live-core` leaves it (the browser engine wasmi would join)
    let live_core = target.join("live-core-simd/wasm32-unknown-unknown/release/live_core.wasm");
    if !live_core.is_file() {
        println!("(live_core.wasm's size: `npm run build:wasm -w shared/live-core` first)");
    }
    files.push(live_core);
    for f in files.iter().filter(|f| f.is_file()) {
        let b = std::fs::read(f)?;
        sizes.push(Size {
            name: f.file_name().unwrap().to_string_lossy().into(),
            bytes: b.len() as u64,
            gz: gz(&b),
        });
    }
    std::fs::write(
        out.join("sizes.json"),
        serde_json::to_string_pretty(&sizes)?,
    )?;
    for z in &sizes {
        println!("{:28} {:>9} B {:>8} B gz", z.name, z.bytes, z.gz);
    }
    if args.iter().any(|a| a == "--exe") {
        // the C runtime linked in: the binary runs on a Windows without the VC++ redistributable
        cargo(
            &[
                "build",
                "--release",
                "-p",
                "module-bench",
                "--features",
                "embed",
            ],
            &target.join("bench-exe"),
            "-Ctarget-feature=+crt-static",
        )?;
        let exe = if cfg!(windows) {
            "module-bench.exe"
        } else {
            "module-bench"
        };
        std::fs::copy(target.join("bench-exe/release").join(exe), out.join(exe))?;
        println!("{exe} → {}", out.display());
    }
    Ok(())
}

#[cfg(feature = "embed")]
fn modules(_: &str) -> Result<Modules> {
    macro_rules! m {
        ($f:literal) => {
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/www/dist/", $f)).to_vec()
        };
    }
    Ok(Modules {
        fm16: m!("fm16.wasm"),
        walk: m!("walk.wasm"),
        analog: m!("analog.wasm"),
        stress: m!("stress.wasm"),
    })
}

#[cfg(not(feature = "embed"))]
fn modules(suffix: &str) -> Result<Modules> {
    let mut m = Modules::default();
    for name in Modules::NAMES {
        let f = dist().join(format!("{name}{suffix}.wasm"));
        m.set(
            name,
            std::fs::read(&f).with_context(|| {
                format!(
                    "{} – run `cargo run --release -p module-bench -- prepare` first",
                    f.display()
                )
            })?,
        );
    }
    Ok(m)
}

/// the date as UTC `YYYY-MM-DD hh:mm`
fn now_utc() -> String {
    let s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs()) as i64;
    let (days, rem) = (s.div_euclid(86_400), s.rem_euclid(86_400));
    // Howard Hinnant's civil_from_days
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02} UTC",
        rem / 3600,
        rem % 3600 / 60
    )
}

fn cpu() -> String {
    if let Ok(c) = std::env::var("PROCESSOR_IDENTIFIER") {
        return c;
    }
    std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|t| {
            t.lines()
                .find(|l| l.starts_with("model name"))
                .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_owned())
        })
        .unwrap_or_default()
}

/// a JSON body to `http://host:port/path` (the serve script's /results) – plain HTTP/1.1 on the LAN, no client library
fn post(url: &str, body: &str) -> Result<()> {
    let rest = url
        .strip_prefix("http://")
        .ok_or_else(|| anyhow!("--post wants http://host:port/path"))?;
    let (host, path) = rest
        .split_once('/')
        .map_or((rest, "/".to_owned()), |(h, p)| (h, format!("/{p}")));
    let mut s = std::net::TcpStream::connect(host)?;
    write!(s, "POST {path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())?;
    let mut answer = String::new();
    s.read_to_string(&mut answer)?;
    println!("posted: {}", answer.lines().next().unwrap_or(""));
    Ok(())
}

fn run(args: &[String]) -> Result<()> {
    let label = arg(args, "--label").unwrap_or_else(|| "pc".into());
    let mut cfg = Config::default();
    if let Some(n) = arg(args, "--blocks") {
        cfg.blocks = n.parse()?;
    }
    let m = modules("")?;
    let device = Device {
        label: label.clone(),
        os: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        cpu: cpu(),
        cores: std::thread::available_parallelism().map_or(0, |n| n.get() as u32),
        when: now_utc(),
        user_agent: format!(
            "module-bench native (wasmtime 38, wasmi 2.0, {})",
            if cfg!(debug_assertions) {
                "DEBUG build – numbers invalid"
            } else {
                "release"
            }
        ),
        notes: vec![],
    };
    let mut results = Results {
        device,
        ..Results::default()
    };
    println!("wasmtime …");
    let engine = deck_module_host::wasmtime_backend::WasmtimeEngine::new(None);
    results
        .runtimes
        .push(run_suite::<deck_module_host::wasmtime_backend::Wasmtime>(
            &engine,
            "wasmtime",
            "native",
            &m,
            &cfg,
            Some("epoch"),
        ));
    println!("wasmi …");
    let engine = module_bench::wasmi_backend::WasmiEngine::default();
    results
        .runtimes
        .push(run_suite::<module_bench::wasmi_backend::Wasmi>(
            &engine,
            "wasmi",
            "native",
            &m,
            &cfg,
            Some("fuel"),
        ));
    if let Ok(t) = std::fs::read_to_string(dist().join("sizes.json")) {
        results.sizes = serde_json::from_str(&t).unwrap_or_default();
    }
    let json = serde_json::to_string_pretty(&results)?;
    let md = report::markdown(std::slice::from_ref(&results));
    println!("{md}");
    if let Some(url) = arg(args, "--post") {
        post(&url, &json)?;
    } else {
        let dir = here().join("results");
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join(format!("{label}-native.json")), &json)?;
        std::fs::write(dir.join(format!("{label}-native.md")), &md)?;
        println!("→ {}", dir.join(format!("{label}-native.json")).display());
    }
    Ok(())
}

fn compare(args: &[String]) -> Result<()> {
    let dir = args
        .get(1)
        .map_or_else(|| here().join("results"), PathBuf::from);
    let mut all = Vec::new();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    for f in files {
        let r: Results = serde_json::from_str(&std::fs::read_to_string(&f)?)
            .with_context(|| f.display().to_string())?;
        all.push(r);
    }
    if all.is_empty() {
        bail!("no results in {}", dir.display());
    }
    println!("{}", report::markdown(&all));
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("prepare") => prepare(&args),
        Some("compare") => compare(&args),
        Some("run") | None => run(&args),
        Some(x) => {
            bail!("unknown command {x}: prepare | run | compare (the header of src/main.rs)")
        }
    }
}
