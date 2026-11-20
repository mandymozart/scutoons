//! wav, mp3 and ogg open through the same probe as the rest: a second of 440 Hz tone each.
use deck_media::audio::{decode, Pcm};

fn open(name: &str) -> Pcm {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/audio").join(name);
    decode(std::fs::read(path).unwrap()).unwrap().unwrap_or_else(|| panic!("{name}: no sound"))
}

fn check(name: &str, rate: u32, channels: u16, slack: f64) {
    let pcm = open(name);
    assert_eq!((pcm.rate, pcm.channels), (rate, channels), "{name}: rate and channels");
    assert!((pcm.seconds() - 1.0).abs() <= slack, "{name}: {} s", pcm.seconds());
    assert!(pcm.samples.iter().any(|s| s.abs() > 0.05), "{name}: not silent");
}

#[test]
fn wav() {
    check("tone.wav", 8000, 1, 0.001);
}

#[test]
fn mp3() {
    check("tone.mp3", 22050, 2, 0.1);
}

#[test]
fn ogg() {
    check("tone.ogg", 22050, 2, 0.1);
}
