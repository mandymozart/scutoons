//! The headless deck host's scenarios natively (#363): every one passes or fails only on the missing capability its
//! `expected_fail` names; every missing capability has a scenario that says what it must answer.
mod common;
use deck_headless::{caps::CAPS, run, summary, Status};

#[test]
fn every_scenario_passes_or_fails_only_on_its_missing_capability() {
    let reports: Vec<_> = common::bundles().iter().map(run).collect();
    let (lines, green) = summary(&reports);
    for l in &lines {
        println!("{l}");
    }
    assert!(green, "a scenario failed:\n{}", lines.iter().filter(|l| l.starts_with("FAIL")).cloned().collect::<Vec<_>>().join("\n"));
    assert!(reports.iter().filter(|r| r.status == Status::Pass).count() >= 10, "the core's own capabilities are driven");
}

#[test]
fn every_missing_capability_has_an_expected_fail_scenario() {
    let bundles = common::bundles();
    for (name, ticket, _, _) in CAPS {
        let found = bundles.iter().any(|b| {
            let t: toml::Table = b.scenario.parse().unwrap();
            let names_it = t["step"].as_array().unwrap().iter().any(|s| s.get("do").and_then(|d| d.as_str()) == Some(name));
            names_it && t.get("expected_fail").and_then(|x| x.as_str()).is_some_and(|x| x.starts_with(ticket))
        });
        assert!(found, "the missing capability '{name}' ({ticket}) has no scenario with expected_fail = \"{ticket} …\"");
    }
}
