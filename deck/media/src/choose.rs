//! Which of an asset's derivatives this deck plays: GPU-texture frames (HAP) where the GPU samples BC, else a codec a
//! hardware decoder takes (`backend::hardware_codecs`, the adapter's capability query), else one the software backend
//! decodes (rav1d's AV1, the floor); of those the tallest that fits the deck's height, else the smallest; at one height
//! the codec with the smaller file. TODO(#325): the derivatives come with the scene's asset manifest; until then a host passes the one file
//! it has (`Derivative::only`).
use crate::backend::Software;
use crate::demux::{Codec, Hap};

#[derive(Debug, Clone, PartialEq)]
pub struct Derivative {
    pub codec: Codec,
    pub width: u32,
    pub height: u32,
    /// where the host fetches it (a CDN path, a cached file)
    pub path: String,
}

impl Derivative {
    /// a lone file, its codec told by its name (`.mov` = HAP, else AV1) – what plays until the manifest says more
    pub fn only(path: &str) -> Derivative {
        let codec = if path.to_ascii_lowercase().ends_with(".mov") { Codec::Hap(Hap::Rgb) } else { Codec::Av1 };
        Derivative { codec, width: 0, height: 0, path: path.to_string() }
    }
}

/// what a deck can play
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    /// the GPU samples BC1 / BC3 (wgpu's `TEXTURE_COMPRESSION_BC`)
    pub bc: bool,
    /// codecs a hardware decoder takes
    pub hardware: Vec<Codec>,
    /// the tallest picture worth playing (the screen's height, or the quality rung's)
    pub max_height: u32,
}

impl Device {
    /// this deck: its GPU's BC sampling, its height, and the codecs its hardware decoders take
    pub fn here(bc: bool, max_height: u32) -> Device {
        Device { bc, hardware: crate::backend::hardware_codecs(), max_height }
    }

    fn rank(&self, d: &Derivative) -> Option<u8> {
        match &d.codec {
            Codec::Hap(Hap::Other) => None,
            Codec::Hap(_) if self.bc => Some(0),
            c if self.hardware.contains(c) => Some(1),
            Codec::Hap(_) => Some(3),
            c if Software::codecs().contains(c) => Some(2),
            _ => None,
        }
    }
}

/// at one height, the codec that makes the smaller file for the same picture first
fn thrift(c: &Codec) -> u8 {
    match c {
        Codec::Av1 => 0,
        Codec::Hevc => 1,
        Codec::Vp9 => 2,
        Codec::H264 => 3,
        _ => 4,
    }
}

/// The derivative to play on `device`; None: nothing it can play.
pub fn choose<'a>(list: &'a [Derivative], device: &Device) -> Option<&'a Derivative> {
    let best = list.iter().filter_map(|d| device.rank(d)).min()?;
    let class: Vec<&Derivative> = list.iter().filter(|d| device.rank(d) == Some(best)).collect();
    let fits = class.iter().filter(|d| d.height <= device.max_height).max_by_key(|d| (d.height, std::cmp::Reverse(thrift(&d.codec))));
    fits.or_else(|| class.iter().min_by_key(|d| (d.height, thrift(&d.codec)))).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(codec: Codec, height: u32) -> Derivative {
        Derivative { codec, width: height * 16 / 9, height, path: format!("{height}") }
    }

    #[test]
    fn a_native_deck_takes_hap_a_phone_its_decoder_and_both_the_height_that_fits() {
        let list = [d(Codec::Hap(Hap::Rgb), 1080), d(Codec::Hap(Hap::Q), 2160), d(Codec::Av1, 540), d(Codec::Av1, 1080), d(Codec::H264, 720)];
        let x1 = Device { bc: true, hardware: vec![], max_height: 1080 };
        assert_eq!(choose(&list, &x1).unwrap().path, "1080", "HAP, the tallest that fits");
        let phone = Device { bc: false, hardware: vec![Codec::H264], max_height: 2400 };
        assert_eq!(choose(&list, &phone).unwrap().codec, Codec::H264, "its hardware decoder first");
        if crate::AV1 {
            let small = Device { bc: false, hardware: vec![], max_height: 720 };
            assert_eq!(choose(&list, &small).unwrap().path, "540", "software AV1, the tallest under the screen");
            let tiny = Device { bc: false, hardware: vec![], max_height: 240 };
            assert_eq!(choose(&list, &tiny).unwrap().path, "540", "nothing fits: the smallest");
        }
        assert!(choose(&[d(Codec::Other("xyz".into()), 720)], &x1).is_none());
    }

    #[test]
    fn without_hardware_av1_the_codec_the_gpu_decodes() {
        // the PC (RTX 2070 SUPER: H.264, HEVC, VP9) and the X1 (UHD 620: the same) decode no AV1 in hardware
        let list = [d(Codec::Av1, 1080), d(Codec::Hevc, 1080), d(Codec::H264, 1080), d(Codec::H264, 720)];
        let pc = Device { bc: false, hardware: vec![Codec::H264, Codec::Hevc, Codec::Vp9], max_height: 1440 };
        assert_eq!(choose(&list, &pc).unwrap().codec, Codec::Hevc, "HEVC: hardware, and smaller than H.264");
        let h264_only = Device { bc: false, hardware: vec![Codec::H264], max_height: 1080 };
        let got = choose(&list, &h264_only).unwrap();
        assert_eq!((&got.codec, got.height), (&Codec::H264, 1080));
        let newer = Device { bc: false, hardware: vec![Codec::Av1, Codec::Hevc], max_height: 1080 };
        assert_eq!(choose(&list, &newer).unwrap().codec, Codec::Av1, "AV1 where the GPU has it");
        if crate::AV1 {
            let none = Device { bc: false, hardware: vec![], max_height: 1080 };
            assert_eq!(choose(&list, &none).unwrap().codec, Codec::Av1, "software: rav1d's AV1, the floor");
        }
    }

    #[test]
    fn one_file_passes_through() {
        let only = [Derivative::only("live/x/loop.mov")];
        assert_eq!(choose(&only, &Device { bc: false, hardware: vec![], max_height: 0 }).unwrap().path, "live/x/loop.mov", "HAP decodes in software too");
    }
}
