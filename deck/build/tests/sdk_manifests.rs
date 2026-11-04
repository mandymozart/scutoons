//! The baked SDK's standalone manifests (deck/build/sdk) say what the workspace's say: same name, version, features and
//! dependencies – so the image's SDK is the SDK the walk module is proven against, and a dependency added to deck-module
//! cannot slip past the compile service's vendored closure.

/// `key = value` lines of a `[section]`, trimmed (comments and blank lines left out)
fn section(manifest: &str, name: &str) -> Vec<String> {
    let mut inside = false;
    let mut out = Vec::new();
    for line in manifest.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        if let Some(header) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            inside = header == name;
        } else if inside {
            out.push(line.to_string());
        }
    }
    out
}

/// the names of a section's keys: `serde = { … }` and `serde.workspace = true` → `serde`
fn names(lines: &[String]) -> Vec<String> {
    lines.iter().filter_map(|l| l.split_once('=')).map(|(k, _)| k.trim().trim_end_matches(".workspace").to_string()).collect()
}

fn value(lines: &[String], key: &str) -> Option<String> {
    lines.iter().filter_map(|l| l.split_once('=')).find(|(k, _)| k.trim() == key).map(|(_, v)| v.trim().to_string())
}

const WORKSPACE: &str = include_str!("../../Cargo.toml");
const MODULE: &str = include_str!("../../module/Cargo.toml");
const MODULE_BAKED: &str = include_str!("../sdk/module.Cargo.toml");
const MACROS: &str = include_str!("../../module-macros/Cargo.toml");
const MACROS_BAKED: &str = include_str!("../sdk/macros.Cargo.toml");
const CATALOG: &str = include_str!("../../catalog/Cargo.toml");
const CATALOG_BAKED: &str = include_str!("../sdk/catalog.Cargo.toml");

#[test]
fn the_baked_deck_module_is_the_workspaces() {
    assert_eq!(value(&section(MODULE_BAKED, "package"), "name"), value(&section(MODULE, "package"), "name"));
    let workspace = section(WORKSPACE, "workspace.package");
    for key in ["version", "edition", "rust-version", "license", "publish"] {
        assert_eq!(value(&section(MODULE_BAKED, "package"), key), value(&workspace, key), "package.{key}");
    }
    assert_eq!(section(MODULE_BAKED, "features"), section(MODULE, "features"));
    let (real, baked) = (section(MODULE, "dependencies"), section(MODULE_BAKED, "dependencies"));
    assert_eq!(names(&baked), names(&real), "the same dependencies");
    // libm: the SDK's deterministic maths (ABI-V2 §6, #340) – the one dependency a member gets through it
    assert_eq!(names(&baked), ["deck-module-macros", "libm", "deck-catalog", "serde_json"]);
    assert_eq!(value(&baked, "libm"), value(&real, "libm"), "the same libm");
    // optional stays optional: a member must not get serde (nor the catalog) through the SDK
    for dep in ["deck-catalog", "serde_json"] {
        assert!(value(&baked, dep).unwrap().contains("optional = true"), "{dep}");
        assert!(value(&real, dep).unwrap().contains("optional = true"), "{dep}");
    }
}

/// deck-module's `host` reads descriptors into deck-catalog's type (#339): the baked catalog is the workspace's too
#[test]
fn the_baked_deck_catalog_is_the_workspaces() {
    assert_eq!(value(&section(CATALOG_BAKED, "package"), "name"), value(&section(CATALOG, "package"), "name"));
    let workspace = section(WORKSPACE, "workspace.package");
    for key in ["version", "edition", "rust-version", "license", "publish"] {
        assert_eq!(value(&section(CATALOG_BAKED, "package"), key), value(&workspace, key), "package.{key}");
    }
    assert_eq!(section(CATALOG_BAKED, "features"), section(CATALOG, "features"));
    let (real, baked) = (section(CATALOG, "dependencies"), section(CATALOG_BAKED, "dependencies"));
    assert_eq!(names(&baked), names(&real), "the same dependencies");
    assert!(value(&baked, "schemars").unwrap().contains("optional = true"));
}

#[test]
fn the_baked_macros_crate_is_the_workspaces() {
    assert_eq!(value(&section(MACROS_BAKED, "package"), "name"), value(&section(MACROS, "package"), "name"));
    let workspace = section(WORKSPACE, "workspace.package");
    for key in ["version", "edition", "rust-version", "license", "publish"] {
        assert_eq!(value(&section(MACROS_BAKED, "package"), key), value(&workspace, key), "package.{key}");
    }
    assert_eq!(section(MACROS_BAKED, "lib"), section(MACROS, "lib"));
    assert_eq!(section(MACROS_BAKED, "dependencies"), section(MACROS, "dependencies"), "syn, quote, proc-macro2 with the same features");
}
