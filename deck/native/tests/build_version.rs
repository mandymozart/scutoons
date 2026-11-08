//! #423: the build version formula exists twice – deck/os/version.sh (update.sh's bundle version and the env it hands the
//! Docker build) and deck-native's build.rs (git, for a plain PC build). They must say the same string. Skipped where
//! bash or git is missing, or where this build got its version from the env (then there is nothing to compare).
use std::path::Path;
use std::process::Command;

fn clean(s: &str) -> &str {
    s.strip_suffix(".dirty").unwrap_or(s)
}

#[test]
fn build_rs_and_version_sh_agree() {
    if option_env!("DECK_BUILD_VERSION").is_some_and(|v| !v.trim().is_empty()) {
        return;
    }
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../os/version.sh");
    let Some(out) = Command::new("bash").arg(&script).output().ok().filter(|o| o.status.success()) else { return };
    let sh = String::from_utf8_lossy(&out.stdout).trim().to_string();
    // the shape: <x.y.z>.r<count>.g<sha>[.dirty], or `dev` outside git
    if sh != "dev" {
        let parts: Vec<&str> = clean(&sh).split('.').collect();
        assert!(parts.len() == 5 && parts[3].starts_with('r') && parts[4].starts_with('g'), "version.sh: {sh}");
        assert!(parts[3][1..].chars().all(|c| c.is_ascii_digit()), "version.sh: {sh}");
    }
    // `.dirty` can differ: it is read when build.rs ran, the script reads it now
    assert_eq!(clean(deck_native::BUILD), clean(&sh), "build.rs and deck/os/version.sh disagree");
}

#[test]
fn the_const_is_what_the_label_shows() {
    assert_eq!(deck_ui::build_label(deck_native::BUILD), deck_native::BUILD);
}
