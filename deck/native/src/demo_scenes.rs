//! The module demo scenes (#406), built in like `madness`: `--scene module-walk` and the F5 list (switcher.rs). They play
//! compiled rig modules, so they need the `modules` build and the modules' wasm in `DECK_MODULES`; a build without either
//! refuses them with the reason (`modules::check`, the graph's unknown kind).

/// the source of a demo scene by name (switcher.rs falls back to it after the other built-ins)
pub fn get(name: &str) -> Option<&'static str> {
    Some(match name {
        "module-walk" => include_str!("../scenes/module-walk.toml"),
        "module-analog" => include_str!("../scenes/module-analog.toml"),
        _ => return None,
    })
}
