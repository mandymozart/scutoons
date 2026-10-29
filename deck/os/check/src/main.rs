//! deck-check – the hardware check of the deck's boot flow (CYBERDECKS.md §1 classes, §3 the image; #173).
//! `deck-check [--out /var/lib/deck/machine.toml] [--no-write]`: probes GPU + Vulkan (vulkaninfo), VA-API decode /
//! encode (vainfo), audio cards + PipeWire rate / quantum, MIDI / USB / serial devices, RAM, CPU, network; prints the
//! report, writes machine.toml (class + feature set – what the suite reads), exits 1 when a must-have is missing
//! (no Vulkan at all, no audio device). Software Vulkan (llvmpipe) passes with a warning: the dogma's fallback.

mod parse;

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde::Serialize;

#[derive(Serialize, Default)]
struct Machine {
    class: String,
    arch: String,
    hostname: String,
    checked: String,
    warnings: Vec<String>,
    cpu: Cpu,
    memory: Memory,
    gpu: Gpu,
    video: Video,
    audio: Audio,
    devices: Devices,
    network: Network,
    features: Features,
}
#[derive(Serialize, Default)]
struct Cpu { model: String, cpus: usize }
#[derive(Serialize, Default)]
struct Memory { total_mib: u64 }
#[derive(Serialize, Default)]
struct Gpu { drivers: Vec<String>, vulkan: bool, vulkan_device: String, vulkan_driver: String, vulkan_api: String, vulkan_software: bool }
#[derive(Serialize, Default)]
struct Video { decode: Vec<String>, encode: Vec<String> }
#[derive(Serialize, Default)]
struct Audio { cards: Vec<String>, pipewire_rate: u32, pipewire_quantum: u32, pipewire_live: bool }
#[derive(Serialize, Default)]
struct Devices { midi: Vec<String>, usb: Vec<String>, serial: Vec<String> }
#[derive(Serialize, Default)]
struct Network { wired: Vec<String>, wifi: Vec<String> }
/// What the suite shows per deck (§1: every feature degrades per class).
#[derive(Serialize, Default)]
struct Features { video: bool, video_encode: bool, audio: bool, midi: bool, editing: bool, display: bool }

fn read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn run(cmd: &str, args: &[&str], env: &[(&str, &str)]) -> Option<String> {
    let mut c = Command::new(cmd);
    c.args(args);
    for (k, v) in env {
        c.env(k, v);
    }
    let out = c.output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    out.status.success().then_some(text)
}

fn list_dir(path: &str) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(path)
        .map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default();
    v.sort();
    v
}

/// The kernel drivers behind /sys/class/drm/card* (i915, xe, amdgpu, virtio_gpu, nouveau …).
fn drm_drivers() -> Vec<String> {
    let mut set = BTreeSet::new();
    for card in list_dir("/sys/class/drm").into_iter().filter(|n| n.starts_with("card") && !n.contains('-')) {
        if let Ok(link) = fs::read_link(format!("/sys/class/drm/{card}/device/driver")) {
            if let Some(name) = link.file_name() {
                set.insert(name.to_string_lossy().into_owned());
            }
        }
    }
    set.into_iter().collect()
}

fn usb_devices() -> Vec<String> {
    let mut v = Vec::new();
    for d in list_dir("/sys/bus/usb/devices") {
        if d.contains(':') || d.starts_with("usb") {
            continue; // interfaces and root hubs
        }
        let base = format!("/sys/bus/usb/devices/{d}");
        let product = read(&format!("{base}/product")).trim().to_owned();
        if product.is_empty() {
            continue;
        }
        let vendor = read(&format!("{base}/manufacturer")).trim().to_owned();
        let id = format!("{}:{}", read(&format!("{base}/idVendor")).trim(), read(&format!("{base}/idProduct")).trim());
        v.push(if vendor.is_empty() { format!("{product} [{id}]") } else { format!("{vendor} {product} [{id}]") });
    }
    v
}

fn probe(pw_uid: &str) -> Machine {
    let mut m = Machine {
        arch: std::env::consts::ARCH.to_owned(),
        hostname: read("/etc/hostname").trim().to_owned(),
        checked: run("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"], &[]).unwrap_or_default().trim().to_owned(),
        ..Machine::default()
    };

    let (model, cpus) = parse::cpuinfo(&read("/proc/cpuinfo"));
    m.cpu = Cpu { model, cpus };
    m.memory.total_mib = parse::meminfo_mib(&read("/proc/meminfo"));

    m.gpu.drivers = drm_drivers();
    let vk = run("vulkaninfo", &["--summary"], &[]).map(|t| parse::vulkaninfo_summary(&t)).unwrap_or_default();
    if let Some(d) = vk.first() {
        m.gpu.vulkan = true;
        m.gpu.vulkan_device = d.name.clone();
        m.gpu.vulkan_driver = d.driver.clone();
        m.gpu.vulkan_api = d.api.clone();
        m.gpu.vulkan_software = d.software();
    }

    let va = run("vainfo", &[], &[]).or_else(|| run("vainfo", &["--display", "drm"], &[]));
    if let Some(t) = va {
        let (dec, enc) = parse::vainfo(&t);
        m.video.decode = dec.into_iter().collect();
        m.video.encode = enc.into_iter().collect();
    }

    m.audio.cards = parse::asound_cards(&read("/proc/asound/cards"));
    let conf = parse::pw_conf(&read("/etc/pipewire/pipewire.conf.d/10-deck-rt.conf"));
    let runtime = format!("/run/user/{pw_uid}");
    let live = run("pw-metadata", &["-n", "settings"], &[("XDG_RUNTIME_DIR", runtime.as_str())]).and_then(|t| parse::pw_settings(&t));
    if let Some((rate, quantum)) = live.or(conf) {
        m.audio.pipewire_rate = rate;
        m.audio.pipewire_quantum = quantum;
        m.audio.pipewire_live = live.is_some();
    }

    m.devices.midi = parse::seq_clients(&read("/proc/asound/seq/clients"));
    m.devices.usb = usb_devices();
    m.devices.serial = list_dir("/dev").into_iter().filter(|n| n.starts_with("ttyUSB") || n.starts_with("ttyACM")).map(|n| format!("/dev/{n}")).collect();

    for n in list_dir("/sys/class/net") {
        if n == "lo" {
            continue;
        }
        if Path::new(&format!("/sys/class/net/{n}/wireless")).exists() || n.starts_with("wl") {
            m.network.wifi.push(n);
        } else if Path::new(&format!("/sys/class/net/{n}/device")).exists() {
            m.network.wired.push(n);
        }
    }

    classify(&mut m);
    m
}

/// §1: x86 = A; ARM with Vulkan and ≥ 6 GB = B; the rest C. Features per class and per what was found.
fn classify(m: &mut Machine) {
    let gb = m.memory.total_mib / 1024;
    m.class = match m.arch.as_str() {
        "x86_64" => "A",
        _ if m.gpu.vulkan && !m.gpu.vulkan_software && gb >= 6 => "B",
        _ => "C",
    }
    .to_owned();
    let hw_vulkan = m.gpu.vulkan && !m.gpu.vulkan_software;
    m.features = Features {
        display: m.gpu.vulkan,
        video: m.class != "C" && hw_vulkan && !m.video.decode.is_empty(),
        video_encode: m.class != "C" && !m.video.encode.is_empty(),
        audio: !m.audio.cards.is_empty(),
        midi: !m.devices.midi.is_empty(),
        editing: m.class == "A" && hw_vulkan && gb >= 8,
    };
    if m.gpu.vulkan_software {
        m.warnings.push("Vulkan is software only (llvmpipe) – no GPU driver found; video and 3D will be slow".to_owned());
    }
    if m.video.decode.is_empty() && m.class != "C" {
        m.warnings.push("no VA-API decode profiles – video plays in software".to_owned());
    }
    if m.audio.pipewire_quantum == 0 {
        m.warnings.push("PipeWire did not answer and no deck config was read".to_owned());
    } else if !m.audio.pipewire_live {
        m.warnings.push("PipeWire rate / quantum are the configured values (the daemon was not running)".to_owned());
    }
    if gb < 4 {
        m.warnings.push(format!("{gb} GB RAM – the browser kiosk wants 4 GB or more"));
    }
}

fn report(m: &Machine) -> String {
    let yes = |b: bool| if b { "yes" } else { "no" };
    let list = |v: &[String]| if v.is_empty() { "none".to_owned() } else { v.join(", ") };
    let mut s = String::new();
    s += &format!("  machine   {} · {} · class {}\n", m.hostname, m.arch, m.class);
    s += &format!("  cpu       {} × {}\n", m.cpu.cpus, m.cpu.model);
    s += &format!("  memory    {} MiB\n", m.memory.total_mib);
    s += &format!("  gpu       {} · Vulkan {}{}\n", list(&m.gpu.drivers), if m.gpu.vulkan { format!("{} ({}, {})", m.gpu.vulkan_device, m.gpu.vulkan_driver, m.gpu.vulkan_api) } else { "NONE".to_owned() }, if m.gpu.vulkan_software { " – software" } else { "" });
    s += &format!("  video     decode {} · encode {}\n", list(&m.video.decode), list(&m.video.encode));
    s += &format!("  audio     {} · PipeWire {} Hz / {} frames{}\n", list(&m.audio.cards), m.audio.pipewire_rate, m.audio.pipewire_quantum, if m.audio.pipewire_live { "" } else { " (configured)" });
    s += &format!("  midi      {}\n", list(&m.devices.midi));
    s += &format!("  serial    {}\n", list(&m.devices.serial));
    s += &format!("  usb       {}\n", list(&m.devices.usb));
    s += &format!("  network   wired {} · wifi {}\n", list(&m.network.wired), list(&m.network.wifi));
    s += &format!("  features  display {} · video {} · encode {} · audio {} · midi {} · editing {}\n", yes(m.features.display), yes(m.features.video), yes(m.features.video_encode), yes(m.features.audio), yes(m.features.midi), yes(m.features.editing));
    for w in &m.warnings {
        s += &format!("  warning   {w}\n");
    }
    s
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut out = "/var/lib/deck/machine.toml".to_owned();
    let mut write = true;
    let mut pw_uid = "1000".to_owned();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => { i += 1; out = args.get(i).cloned().unwrap_or(out); }
            "--no-write" => write = false,
            "--pw-uid" => { i += 1; pw_uid = args.get(i).cloned().unwrap_or(pw_uid); }
            "-h" | "--help" => { println!("deck-check [--out PATH] [--no-write] [--pw-uid UID]"); return; }
            a => { eprintln!("deck-check: unknown argument {a}"); std::process::exit(2); }
        }
        i += 1;
    }

    let m = probe(&pw_uid);
    print!("{}", report(&m));

    let mut missing = Vec::new();
    if !m.gpu.vulkan {
        missing.push("no Vulkan device at all (not even llvmpipe – is mesa / vulkan-swrast installed?)");
    }
    if m.audio.cards.is_empty() {
        missing.push("no audio device (ALSA lists no card)");
    }

    if write {
        let text = format!("# written by deck-check {} – the machine's class and feature set (CYBERDECKS.md §1); re-run: deck-check\n{}", m.checked, toml::to_string_pretty(&m).unwrap_or_default());
        if let Some(dir) = Path::new(&out).parent() {
            let _ = fs::create_dir_all(dir);
        }
        match fs::write(&out, text) {
            Ok(()) => println!("  written   {out}"),
            Err(e) => eprintln!("  not written: {out}: {e}"),
        }
    }
    if !missing.is_empty() {
        for x in &missing {
            eprintln!("  MISSING   {x}");
        }
        std::process::exit(1);
    }
}
