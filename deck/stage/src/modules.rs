//! The stage's module registry (#185, CYBERDECKS.md §8): a module is a cargo feature of deck-stage that registers node
//! kinds or tempo sources – the table is deck-core's `stage::STAGE_MODULES` (data only, so `deck-scene-features` reads
//! it without building the stage); this file says which of them this build carries. A scene that needs a module the
//! build lacks is refused before anything starts, with the feature's name – main.rs shows it on screen. Runtime
//! modules (wasm generators, later) will join here; no dylib plugins (DECISIONS 2026-10-30).
use crate::scene::Scene;
use anyhow::{anyhow, bail, Result};
use deck_core::stage::{node_needs, StageModule, STAGE_MODULES};
use std::collections::BTreeMap;

/// whether this build carries the module `feature` (None: the stage has no such module)
pub fn built(feature: &str) -> Option<bool> {
    Some(match feature {
        "core" => true,
        "space" => cfg!(feature = "space"),
        "chapter" => cfg!(feature = "chapter"),
        "live" => cfg!(feature = "live"),
        "sync" => cfg!(feature = "sync"),
        "midi" => cfg!(feature = "midi"),
        "osc" => cfg!(feature = "osc"),
        "link" => cfg!(feature = "link"),
        "pipewire" => cfg!(all(target_os = "linux", feature = "pipewire")),
        _ => return None,
    })
}

fn has(feature: &str) -> bool {
    built(feature) == Some(true)
}

/// the modules this build carries (their kinds and sources)
pub fn registry() -> impl Iterator<Item = &'static StageModule> {
    STAGE_MODULES.iter().filter(|m| has(m.feature))
}

/// the modules this build carries besides the core, comma-separated ("-" for the core alone)
pub fn summary() -> String {
    let names: Vec<&str> = registry().map(|m| m.feature).filter(|&f| f != deck_core::stage::CORE).collect();
    if names.is_empty() {
        "-".into()
    } else {
        names.join(",")
    }
}

/// A scene against this build: Err names each missing module's feature and the nodes that need it.
pub fn check(scene: &Scene) -> Result<()> {
    check_with(scene, has)
}

/// `check` against the modules `carried` says are there
pub fn check_with(scene: &Scene, carried: impl Fn(&str) -> bool) -> Result<()> {
    let mut missing: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for n in &scene.nodes {
        for f in node_needs(&n.id, &n.kind, &n.params).map_err(|e| anyhow!(e))? {
            if !carried(f) {
                missing.entry(f).or_default().push(&n.id);
            }
        }
    }
    if missing.is_empty() {
        return Ok(());
    }
    let what: Vec<String> = missing.iter().map(|(f, ids)| format!("'{f}' (node {})", ids.join(", "))).collect();
    let features: Vec<&str> = missing.keys().copied().collect();
    bail!(
        "this build of the stage does not carry the module{} {} – build deck-stage with --features {}",
        if missing.len() > 1 { "s" } else { "" },
        what.join(", "),
        features.join(",")
    )
}

/// The scene without the nodes whose module this build lacks, and without their links – for the tests and benches that
/// play the bundled scenes on any build (the stage itself refuses such a scene, `check`).
pub fn carried_part(scene: &Scene) -> Scene {
    let carried = |n: &crate::scene::NodeDef| node_needs(&n.id, &n.kind, &n.params).is_ok_and(|f| f.iter().all(|f| has(f)));
    let gone: Vec<&str> = scene.nodes.iter().filter(|n| !carried(n)).map(|n| n.id.as_str()).collect();
    let refers = |r: &str| gone.contains(&crate::scene::split_ref(r).0);
    let mut out = scene.clone();
    out.nodes.retain(carried);
    out.links.retain(|l| !refers(&l.from) && !refers(&l.to));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Kind;

    #[test]
    fn the_table_and_the_stage_agree() {
        for m in STAGE_MODULES {
            assert!(built(m.feature).is_some(), "module '{}' is no feature of deck-stage", m.feature);
            for k in m.kinds {
                assert!(Kind::parse(k).is_some(), "kind '{k}' of module '{}' is no stage kind", m.feature);
            }
            for s in m.sources {
                assert!(crate::tempo::Source::parse(s).is_some(), "source '{s}' of module '{}' is no tempo source", m.feature);
            }
        }
        for k in Kind::ALL {
            assert!(deck_core::stage::module_of_kind(k.name()).is_some(), "{k:?} is in no module (deck-core stage.rs)");
        }
        for s in crate::tempo::Source::ALL {
            assert!(STAGE_MODULES.iter().any(|m| m.sources.contains(&s.name())), "{s:?} is in no module");
        }
        assert_eq!(registry().next().unwrap().feature, "core");
    }

    #[test]
    fn a_missing_module_is_named_with_its_feature() {
        let cathedral = Scene::parse(crate::scene::CATHEDRAL_SCENE).unwrap();
        let e = check_with(&cathedral, |f| f == "core").unwrap_err().to_string();
        assert_eq!(
            e,
            "this build of the stage does not carry the modules 'chapter' (node music), 'space' (node space) – \
             build deck-stage with --features chapter,space"
        );
        assert!(check_with(&cathedral, |_| true).is_ok());
        let midi = Scene::parse("[[node]]\nid = \"clock\"\nkind = \"tempo\"\nsource = \"midi\"\n").unwrap();
        let e = check_with(&midi, |f| f != "midi").unwrap_err().to_string();
        assert!(e.contains("module 'midi' (node clock)") && e.ends_with("--features midi"), "{e}");
    }

    /// what this very build does: the core alone refuses the bundled scenes' modules, the full build plays them
    #[test]
    fn this_build_checks_the_bundled_scenes() {
        let cathedral = Scene::parse(crate::scene::CATHEDRAL_SCENE).unwrap();
        let r = check(&cathedral);
        if cfg!(all(feature = "space", feature = "chapter")) {
            assert!(r.is_ok(), "{r:?}");
        } else {
            assert!(r.unwrap_err().to_string().contains("--features"));
        }
        assert!(check(&Scene::parse(crate::scene::TEST_PATTERN_SCENE).unwrap()).is_ok(), "the core plays the test pattern");
        let part = carried_part(&cathedral);
        assert!(check(&part).is_ok());
        assert_eq!(part.nodes.len() == cathedral.nodes.len(), cfg!(all(feature = "space", feature = "chapter")));
    }
}
