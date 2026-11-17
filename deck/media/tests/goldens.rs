//! The fixture clips decoded whole: their tracks as the demuxers read them, and their goldens (`deck_media::golden`),
//! which tests/wasm.rs compares bit for bit in wasm32.
mod common;
use deck_media::{Clip, Codec, Frames, Hap, Player, Software};

#[test]
fn the_demuxers_read_the_tracks() {
    let open = |name: &str| Clip::open(common::fixture(name).into()).unwrap();
    for (name, codec, audio) in [
        ("av1.mp4", Codec::Av1, Some(Codec::Aac)),
        ("av1.webm", Codec::Av1, Some(Codec::Vorbis)),
        ("hap.mov", Codec::Hap(Hap::Rgb), None),
        ("hap_alpha.mov", Codec::Hap(Hap::Rgba), None),
        ("hap_q.mov", Codec::Hap(Hap::Q), None),
    ] {
        let clip = open(name);
        let v = clip.video.as_ref().unwrap();
        assert_eq!((&v.codec, v.width, v.height, v.samples.len()), (&codec, 64, 48, 12), "{name}");
        assert!((v.length() - 1.0).abs() < 1e-3, "{name}: {} s", v.length());
        assert!(v.samples[0].key, "{name}: it starts on a key frame");
        assert_eq!(clip.audio.as_ref().map(|a| a.codec.clone()), audio, "{name}");
    }
}

#[test]
fn every_fixture_decodes_to_its_golden() {
    let got: Vec<String> = common::FIXTURES.iter().map(|f| deck_media::golden(&common::fixture(f.0)).unwrap().line()).collect();
    let want: Vec<&str> = common::FIXTURES.iter().map(|f| f.2).collect();
    assert_eq!(got, want);
}

#[test]
fn the_same_pictures_from_both_containers_and_hap_alpha_opaque_as_hap() {
    let video = |name: &str| common::FIXTURES.iter().find(|f| f.0 == name).unwrap().2.split(' ').next().unwrap();
    assert_eq!(video("av1.mp4"), video("av1.webm"), "one AV1 stream in MP4 and in WebM");
    assert_eq!(video("hap.mov"), video("hap_alpha.mov"), "the pattern is opaque: DXT5's colours are DXT1's");
}

#[test]
fn hap_stays_blocks_for_the_gpu() {
    let clip = Clip::open(common::fixture("hap_alpha.mov").into()).unwrap();
    let mut p = Player::new(&clip, Box::new(Software::default())).unwrap();
    let mut q = Frames::new(4);
    let f = p.frame_for(0.0, &mut q).unwrap().unwrap();
    assert_eq!((f.picture.format, f.picture.data.len()), (deck_media::Format::Bc3, 16 * 12 * 16), "16 × 12 DXT5 blocks");
    let rgba = f.picture.to_rgba();
    assert_eq!(rgba.data.len(), 64 * 48 * 4);
    assert!(rgba.data.chunks(4).all(|p| p[3] == 255), "opaque");
}

/// `DUMP=<dir> cargo test -p deck-media --test goldens dump`: each fixture's frame at 0.5 s as a PPM, to look at
#[test]
fn dump() {
    let Ok(out) = std::env::var("DUMP") else { return };
    for (name, _, _) in common::FIXTURES {
        let clip = Clip::open(common::fixture(name).into()).unwrap();
        let mut p = Player::new(&clip, Box::new(Software::default())).unwrap();
        let mut q = Frames::new(4);
        let f = p.frame_for(0.5, &mut q).unwrap().unwrap().picture.to_rgba();
        let mut ppm = format!("P6 {} {} 255\n", f.width, f.height).into_bytes();
        for px in f.data.chunks(4) {
            ppm.extend_from_slice(&px[..3]);
        }
        std::fs::write(format!("{out}/{name}.ppm"), ppm).unwrap();
    }
}
