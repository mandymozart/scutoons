//! The `deck-asset-timemap` binary against the derivative fixtures (tests/derivatives.rs lists how they were made).
use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_deck-asset-timemap")).args(args).output().expect("the binary runs")
}

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/derivatives/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn measured(derivative: &str) -> (f64, f64) {
    let out = run(&[&fixture("source.mov"), &fixture(derivative)]);
    assert!(out.status.success(), "{derivative}: {}", String::from_utf8_lossy(&out.stderr));
    let json = String::from_utf8(out.stdout).unwrap();
    let number = |key: &str| -> f64 {
        let rest = json.split(&format!("\"{key}\":")).nth(1).unwrap_or_else(|| panic!("{key} in {json}"));
        rest.split([',', '}']).next().unwrap().trim().parse().unwrap()
    };
    (number("offset"), number("fps"))
}

#[test]
fn prints_the_time_map_as_json() {
    let (offset, fps) = measured("hap_q_small.mov");
    assert_eq!((offset, fps.round()), (0.0, 12.0));
    if deck_media::AV1 {
        let (offset, fps) = measured("av1_from_half.mp4");
        assert!((offset - 0.5).abs() < 1e-6 && fps.round() == 12.0, "{offset} {fps}");
        let (offset, fps) = measured("av1_24fps.webm");
        assert!(offset.abs() < 1e-6 && fps.round() == 24.0, "{offset} {fps}");
    }
}

#[test]
fn a_failure_exits_non_zero_with_the_reason() {
    let out = run(&[&fixture("source.mov"), &fixture("missing.mp4")]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("missing.mp4"));

    let dir = std::env::temp_dir().join(format!("timemap-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let junk = dir.join("junk.mp4");
    std::fs::write(&junk, b"not a video at all").unwrap();
    let out = run(&[&fixture("source.mov"), junk.to_str().unwrap()]);
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!(out.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&out.stderr).trim().is_empty());

    assert_eq!(run(&[&fixture("source.mov")]).status.code(), Some(2), "usage");
}
