//! `deck-scenarios [--json] [--speed X] [scenario.toml …]` – the headless deck host's runner (#363): every scenario in
//! deck/headless/scenarios (or the files named), each with the deck/ files it names bundled, run on the virtual clock;
//! one line per scenario (pass / xfail with its ticket / FAIL), exit 1 when one fails. `--speed 1` paces the virtual
//! clock in real time (a demo, a soak), the default runs as fast as it goes. `--json` prints the reports. The only
//! file I/O of the host lives here – the library takes bundles.
//! `deck-scenarios --bench <sandbox name | scene.toml> [--frames N] [--far] [--every-knob]` times the stage's CPU side
//! instead (bench.rs, #834); a scene with modules (crows-rave) needs `--features bench`.
use deck_headless::{run_paced, summary, Bundle};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(i) = args.iter().position(|a| a == "--bench") {
        std::process::exit(bench(&args, args.get(i + 1).map_or("crows-rave", String::as_str)));
    }
    let json = args.iter().any(|a| a == "--json");
    let speed = args.iter().position(|a| a == "--speed").and_then(|i| args.get(i + 1)).and_then(|s| s.parse::<f64>().ok()).filter(|s| *s > 0.0);
    let deck = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files: Vec<PathBuf> = args.iter().filter(|a| a.ends_with(".toml")).map(PathBuf::from).collect();
    if files.is_empty() {
        files = scenarios(&deck);
    }
    let t = Instant::now();
    let mut reports = Vec::new();
    for f in &files {
        let bundle = match bundle(&deck, f) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("{}: {e}", f.display());
                std::process::exit(2);
            }
        };
        let pace: Option<Box<dyn FnMut(f64)>> = speed.map(|s| {
            let start = Instant::now();
            let mut virt = 0.0;
            Box::new(move |ms: f64| {
                virt += ms / s;
                let due = Duration::from_secs_f64(virt / 1e3);
                if let Some(wait) = due.checked_sub(start.elapsed()) {
                    std::thread::sleep(wait);
                }
            }) as Box<dyn FnMut(f64)>
        });
        reports.push(run_paced(&bundle, pace));
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&reports).unwrap_or_default());
    }
    let (lines, green) = summary(&reports);
    for l in &lines {
        println!("{l}");
    }
    let n = |s: &str| lines.iter().filter(|l| l.starts_with(s)).count();
    println!("deck-headless: {} pass, {} xfail (missing capabilities), {} FAIL · {} ms", n("pass"), n("xfail"), n("FAIL"), t.elapsed().as_millis());
    if !green {
        std::process::exit(1);
    }
}

/// the bundled scenarios, in name order
fn scenarios(deck: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(deck.join("headless/scenarios"))
        .map(|d| d.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "toml")).collect())
        .unwrap_or_default();
    v.sort();
    v
}

/// a scenario file and the deck/ files its steps name
fn bundle(deck: &Path, file: &Path) -> Result<Bundle, String> {
    let scenario = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let name = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    deck_headless::bundle(&name, scenario, |p| std::fs::read_to_string(deck.join(p)).map_err(|e| e.to_string()))
}

/// `--bench`: one line of the stage's CPU side (bench.rs); exit 0, or 2 when the scene can't be built
fn bench(args: &[String], name: &str) -> i32 {
    #[cfg(feature = "bench")]
    register_modules();
    let frames = args.iter().position(|a| a == "--frames").and_then(|i| args.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(1200);
    let sandbox = Path::new(env!("CARGO_MANIFEST_DIR")).join("../native/scenes/sandbox").join(format!("{name}.toml"));
    let text = match std::fs::read_to_string(&sandbox).or_else(|_| std::fs::read_to_string(name)) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("--bench {name}: no sandbox scene and no file ({e})");
            return 2;
        }
    };
    let scene = match deck_space::scene::Scene::parse(&text) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("--bench {name}: {e:#}");
            return 2;
        }
    };
    let t = Instant::now();
    let mut clock = || t.elapsed().as_secs_f64() * 1000.0;
    match deck_headless::bench::bench(&scene, frames, !args.iter().any(|a| a == "--far"), args.iter().any(|a| a == "--every-knob"), &mut clock) {
        Ok(b) => {
            println!("{name}: {}", b.line());
            0
        }
        Err(e) => {
            eprintln!("--bench {name}: {e}{}", if cfg!(feature = "bench") { "" } else { " (a scene with modules: --features bench)" });
            2
        }
    }
}

/// the factory modules a bench scene may use, by their descriptors (as deck-space's tests/crows_rave.rs registers them)
#[cfg(feature = "bench")]
fn register_modules() {
    use deck_module::Describe;
    for json in [arp_module::Arp::DESCRIPTOR, analog_module::Analog::DESCRIPTOR, drumseq_module::DrumSeq::DESCRIPTOR, delay_module::Delay::DESCRIPTOR, reverb_module::Reverb::DESCRIPTOR] {
        if let Ok(d) = serde_json::from_str::<deck_catalog::Descriptor>(json) {
            deck_space::modreg::register(&d.name, &deck_space::modreg::ModuleDef::of(&d));
        }
    }
}
