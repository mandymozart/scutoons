//! The parsers of deck-check: text in (sysfs, /proc, a tool's output) → facts. Pure functions, tested below.

use std::collections::BTreeSet;

/// `/proc/cpuinfo` → (model name, logical cpus).
pub fn cpuinfo(text: &str) -> (String, usize) {
    let mut model = String::new();
    let mut cpus = 0;
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("model name") {
            if model.is_empty() {
                model = v.trim_start_matches([' ', '\t', ':']).trim().to_owned();
            }
        }
        if line.starts_with("processor") {
            cpus += 1;
        }
    }
    (model, cpus)
}

/// `/proc/meminfo` → MemTotal in MiB.
pub fn meminfo_mib(text: &str) -> u64 {
    text.lines()
        .find_map(|l| l.strip_prefix("MemTotal:"))
        .and_then(|v| v.split_whitespace().next())
        .and_then(|kb| kb.parse::<u64>().ok())
        .map(|kb| kb / 1024)
        .unwrap_or(0)
}

/// A Vulkan device from `vulkaninfo --summary`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VulkanDevice {
    pub name: String,
    pub driver: String,
    pub api: String,
    pub device_type: String,
}

impl VulkanDevice {
    /// llvmpipe / lavapipe = Mesa's software Vulkan (the fallback the dogma asks for, not a GPU).
    pub fn software(&self) -> bool {
        let n = self.name.to_lowercase();
        n.contains("llvmpipe") || n.contains("lavapipe") || self.device_type.contains("CPU")
    }
}

/// `vulkaninfo --summary` → the devices of its "Devices:" section (`GPUn:` blocks with `deviceName`, `driverName`,
/// `apiVersion`, `deviceType`). Pure GPU devices first, software last.
pub fn vulkaninfo_summary(text: &str) -> Vec<VulkanDevice> {
    let mut out = Vec::new();
    let mut cur: Option<VulkanDevice> = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with("GPU") && line.ends_with(':') {
            if let Some(d) = cur.take() {
                out.push(d);
            }
            cur = Some(VulkanDevice::default());
            continue;
        }
        let Some(d) = cur.as_mut() else { continue };
        let Some((k, v)) = line.split_once('=') else { continue };
        let v = v.trim().to_owned();
        match k.trim() {
            "deviceName" => d.name = v,
            "driverName" => d.driver = v,
            "apiVersion" => d.api = v.split_whitespace().next().unwrap_or("").to_owned(),
            "deviceType" => d.device_type = v,
            _ => {}
        }
    }
    if let Some(d) = cur.take() {
        out.push(d);
    }
    out.sort_by_key(|d| d.software());
    out
}

/// `vainfo` → (decode codecs, encode codecs), lower-case names: h264 hevc av1 vp9 vp8 mpeg2 jpeg.
pub fn vainfo(text: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut dec = BTreeSet::new();
    let mut enc = BTreeSet::new();
    for line in text.lines() {
        let line = line.trim();
        let Some((profile, entry)) = line.split_once(':') else { continue };
        let Some(profile) = profile.trim().strip_prefix("VAProfile") else { continue };
        let codec = match profile {
            p if p.starts_with("H264") => "h264",
            p if p.starts_with("HEVC") => "hevc",
            p if p.starts_with("AV1") => "av1",
            p if p.starts_with("VP9") => "vp9",
            p if p.starts_with("VP8") => "vp8",
            p if p.starts_with("MPEG2") => "mpeg2",
            p if p.starts_with("JPEG") => "jpeg",
            _ => continue,
        };
        let entry = entry.trim();
        if entry.starts_with("VAEntrypointVLD") {
            dec.insert(codec.to_owned());
        } else if entry.starts_with("VAEntrypointEncSlice") || entry.starts_with("VAEntrypointEncPicture") {
            enc.insert(codec.to_owned());
        }
    }
    (dec, enc)
}

/// `/proc/asound/cards` → the card names (" 0 [PCH ]: HDA-Intel - HDA Intel PCH" → "HDA Intel PCH").
pub fn asound_cards(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| l.trim_start().chars().next().is_some_and(|c| c.is_ascii_digit()))
        .filter_map(|l| l.split(" - ").nth(1))
        .map(|s| s.trim().to_owned())
        .collect()
}

/// `/proc/asound/seq/clients` → the MIDI clients that are hardware or user ports (not the kernel's System /
/// Midi Through): `Client  20 : "Launchpad Mini" [Kernel]`.
pub fn seq_clients(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| l.starts_with("Client"))
        .filter_map(|l| {
            let name = l.split('"').nth(1)?;
            (name != "System" && !name.starts_with("Midi Through")).then(|| name.to_owned())
        })
        .collect()
}

/// `pw-metadata -n settings` → (clock.rate, clock.quantum) when both are present.
pub fn pw_settings(text: &str) -> Option<(u32, u32)> {
    let find = |key: &str| {
        text.lines()
            .find(|l| l.contains(&format!("key:'{key}'")))
            .and_then(|l| l.split("value:'").nth(1))
            .and_then(|v| v.split('\'').next())
            .and_then(|v| v.parse::<u32>().ok())
    };
    // force-* wins when set (a client asked), else the default the config set
    let rate = find("clock.force-rate").filter(|r| *r > 0).or_else(|| find("clock.rate"))?;
    let quantum = find("clock.force-quantum").filter(|q| *q > 0).or_else(|| find("clock.quantum"))?;
    Some((rate, quantum))
}

/// The deck's PipeWire config (`default.clock.rate = 48000`, `default.clock.quantum = 128`) → (rate, quantum).
pub fn pw_conf(text: &str) -> Option<(u32, u32)> {
    let find = |key: &str| {
        text.lines()
            .map(str::trim)
            .find(|l| l.starts_with(key))
            .and_then(|l| l.split('=').nth(1))
            .and_then(|v| v.trim().parse::<u32>().ok())
    };
    Some((find("default.clock.rate")?, find("default.clock.quantum")?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_and_memory() {
        let (m, n) = cpuinfo("processor\t: 0\nmodel name\t: Intel(R) Core(TM) i7-8550U CPU @ 1.80GHz\nprocessor\t: 1\nmodel name\t: x\n");
        assert_eq!(m, "Intel(R) Core(TM) i7-8550U CPU @ 1.80GHz");
        assert_eq!(n, 2);
        assert_eq!(meminfo_mib("MemTotal:       16274372 kB\nMemFree: 1 kB\n"), 15892);
    }

    #[test]
    fn vulkan_summary_orders_hardware_first() {
        let text = "Devices:\n========\nGPU0:\n\tapiVersion         = 1.3.289\n\tdriverVersion      = 0.0.1\n\tdeviceName         = llvmpipe (LLVM 18.1.8, 256 bits)\n\tdriverName         = llvmpipe\n\tdeviceType         = PHYSICAL_DEVICE_TYPE_CPU\nGPU1:\n\tapiVersion         = 1.3.289\n\tdeviceName         = Intel(R) UHD Graphics 620 (KBL GT2)\n\tdriverName         = Intel open-source Mesa driver\n\tdeviceType         = PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU\n";
        let d = vulkaninfo_summary(text);
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].name, "Intel(R) UHD Graphics 620 (KBL GT2)");
        assert!(!d[0].software());
        assert!(d[1].software());
        assert_eq!(d[0].api, "1.3.289");
        assert!(vulkaninfo_summary("vulkaninfo: error\n").is_empty());
    }

    #[test]
    fn vainfo_profiles() {
        let text = "libva info: VA-API version 1.22.0\nvainfo: Supported profile and entrypoints\n      VAProfileH264Main               : VAEntrypointVLD\n      VAProfileH264Main               : VAEntrypointEncSliceLP\n      VAProfileHEVCMain               : VAEntrypointVLD\n      VAProfileAV1Profile0            : VAEntrypointVLD\n      VAProfileJPEGBaseline           : VAEntrypointEncPicture\n      VAProfileNone                   : VAEntrypointVideoProc\n";
        let (dec, enc) = vainfo(text);
        assert_eq!(dec.iter().cloned().collect::<Vec<_>>(), ["av1", "h264", "hevc"]);
        assert_eq!(enc.iter().cloned().collect::<Vec<_>>(), ["h264", "jpeg"]);
    }

    #[test]
    fn audio_and_midi() {
        assert_eq!(asound_cards(" 0 [PCH            ]: HDA-Intel - HDA Intel PCH\n                      HDA Intel PCH at 0xe1310000 irq 136\n 1 [Audio          ]: USB-Audio - Scarlett 2i2 USB\n"), ["HDA Intel PCH", "Scarlett 2i2 USB"]);
        assert_eq!(seq_clients("Client info\n  cur  clients : 3\nClient   0 : \"System\" [Kernel]\n  Port   0 : \"Timer\"\nClient  14 : \"Midi Through\" [Kernel]\nClient  20 : \"Launchpad Mini\" [Kernel]\n"), ["Launchpad Mini"]);
    }

    #[test]
    fn pipewire_live_and_configured() {
        let live = "Found \"settings\" metadata 30\nupdate: id:0 key:'log.level' value:'2' type:''\nupdate: id:0 key:'clock.rate' value:'48000' type:''\nupdate: id:0 key:'clock.quantum' value:'128' type:''\nupdate: id:0 key:'clock.force-quantum' value:'0' type:''\n";
        assert_eq!(pw_settings(live), Some((48000, 128)));
        assert_eq!(pw_settings("nothing"), None);
        assert_eq!(pw_conf("context.properties = {\n    default.clock.rate          = 48000\n    default.clock.quantum       = 128\n}\n"), Some((48000, 128)));
    }
}
