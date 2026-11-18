//! The icons (#456; owner 2026-11-09: "having some icon library we have in osct can be super useful"): Feather (MIT,
//! feathericons.com), the set of the OSCT editor, as the inner SVG of a 24 × 24 viewBox. Drawn with stroke 2.5 and square
//! caps; an adapter rasterises them. Only where a word makes no sense.
pub const ICONS: &[(&str, &str)] = &[
    ("list", r#"<line x1="8" y1="6" x2="21" y2="6"></line><line x1="8" y1="12" x2="21" y2="12"></line><line x1="8" y1="18" x2="21" y2="18"></line><line x1="3" y1="6" x2="3.01" y2="6"></line><line x1="3" y1="12" x2="3.01" y2="12"></line><line x1="3" y1="18" x2="3.01" y2="18"></line>"#),
    ("tree", r#"<line x1="9" y1="5" x2="21" y2="5"></line><line x1="13" y1="12" x2="21" y2="12"></line><line x1="13" y1="19" x2="21" y2="19"></line><path d="M4 3v16h5"></path><line x1="4" y1="12" x2="9" y2="12"></line>"#),
    ("inspector", r#"<rect x="6" y="6" width="12" height="12"></rect><rect x="3" y="3" width="4" height="4"></rect><rect x="17" y="3" width="4" height="4"></rect><rect x="3" y="17" width="4" height="4"></rect><rect x="17" y="17" width="4" height="4"></rect>"#),
    ("nodes", r#"<rect x="9" y="2" width="6" height="6" rx="1"></rect><rect x="2" y="16" width="6" height="6" rx="1"></rect><rect x="16" y="16" width="6" height="6" rx="1"></rect><path d="M12 8v4M5 16v-4h14v4"></path>"#),
    ("sliders", r#"<line x1="4" y1="21" x2="4" y2="14"></line><line x1="4" y1="10" x2="4" y2="3"></line><line x1="12" y1="21" x2="12" y2="12"></line><line x1="12" y1="8" x2="12" y2="3"></line><line x1="20" y1="21" x2="20" y2="16"></line><line x1="20" y1="12" x2="20" y2="3"></line><line x1="1" y1="14" x2="7" y2="14"></line><line x1="9" y1="8" x2="15" y2="8"></line><line x1="17" y1="16" x2="23" y2="16"></line>"#),
    ("globe", r#"<circle cx="12" cy="12" r="10"></circle><line x1="2" y1="12" x2="22" y2="12"></line><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path>"#),
    ("folder", r#"<path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>"#),
    ("code", r#"<polyline points="16 18 22 12 16 6"></polyline><polyline points="8 6 2 12 8 18"></polyline>"#),
    ("previous", r#"<polyline points="15 18 9 12 15 6"></polyline>"#),
    ("next", r#"<polyline points="9 18 15 12 9 6"></polyline>"#),
    ("replay", r#"<polyline points="1 4 1 10 7 10"></polyline><path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"></path>"#),
    ("camera", r#"<path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"></path><circle cx="12" cy="13" r="4"></circle>"#),
    ("close", r#"<line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line>"#),
    ("window-min", r#"<line x1="6" y1="12" x2="18" y2="12"></line>"#),
    ("window-max", r#"<rect x="6" y="6" width="12" height="12"></rect>"#),
    ("window-restore", r#"<rect x="6" y="9" width="9" height="9"></rect><polyline points="9 9 9 6 18 6 18 15 15 15"></polyline>"#),
    ("reset", r#"<polyline points="1 4 1 10 7 10"></polyline><path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"></path>"#),
    ("up", r#"<polyline points="18 15 12 9 6 15"></polyline>"#),
    ("down", r#"<polyline points="6 9 12 15 18 9"></polyline>"#),
    ("book", r#"<path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"></path><path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"></path>"#),
    ("entry", r#"<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path><polyline points="14 2 14 8 20 8"></polyline><line x1="16" y1="13" x2="8" y2="13"></line><line x1="16" y1="17" x2="8" y2="17"></line>"#),
    ("target", r#"<circle cx="12" cy="12" r="10"></circle><line x1="22" y1="12" x2="18" y2="12"></line><line x1="6" y1="12" x2="2" y2="12"></line><line x1="12" y1="6" x2="12" y2="2"></line><line x1="12" y1="22" x2="12" y2="18"></line>"#),
    ("model", r#"<path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"></path><polyline points="3.27 6.96 12 12.01 20.73 6.96"></polyline><line x1="12" y1="22.08" x2="12" y2="12"></line>"#),
    ("video", r#"<rect x="2" y="2" width="20" height="20" rx="2.18" ry="2.18"></rect><line x1="7" y1="2" x2="7" y2="22"></line><line x1="17" y1="2" x2="17" y2="22"></line><line x1="2" y1="12" x2="22" y2="12"></line><line x1="2" y1="7" x2="7" y2="7"></line><line x1="2" y1="17" x2="7" y2="17"></line><line x1="17" y1="17" x2="22" y2="17"></line><line x1="17" y1="7" x2="22" y2="7"></line>"#),
    ("image", r#"<rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect><circle cx="8.5" cy="8.5" r="1.5"></circle><polyline points="21 15 16 10 5 21"></polyline>"#),
    ("layers", r#"<polygon points="12 2 2 7 12 12 22 7 12 2"></polygon><polyline points="2 17 12 22 22 17"></polyline><polyline points="2 12 12 17 22 12"></polyline>"#),
    ("type", r#"<polyline points="4 7 4 4 20 4 20 7"></polyline><line x1="9" y1="20" x2="15" y2="20"></line><line x1="12" y1="4" x2="12" y2="20"></line>"#),
    ("light", r#"<circle cx="12" cy="12" r="5"></circle><line x1="12" y1="1" x2="12" y2="3"></line><line x1="12" y1="21" x2="12" y2="23"></line><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"></line><line x1="1" y1="12" x2="3" y2="12"></line><line x1="21" y1="12" x2="23" y2="12"></line><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line>"#),
    ("dark", r#"<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path>"#),
    ("plus",r#"<line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line>"#),
    ("search", r#"<circle cx="11" cy="11" r="8"></circle><line x1="21" y1="21" x2="16.65" y2="16.65"></line>"#),
    ("eye", r#"<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path><circle cx="12" cy="12" r="3"></circle>"#),
    ("droplet", r#"<path d="M12 2.69l5.66 5.66a8 8 0 1 1-11.31 0z"></path>"#),
    ("play", r#"<polygon points="5 3 19 12 5 21 5 3"></polygon>"#),
    ("pause", r#"<rect x="6" y="4" width="4" height="16"></rect><rect x="14" y="4" width="4" height="16"></rect>"#),
    ("move", r#"<polyline points="5 9 2 12 5 15"></polyline><polyline points="9 5 12 2 15 5"></polyline><polyline points="15 19 12 22 9 19"></polyline><polyline points="19 9 22 12 19 15"></polyline><line x1="2" y1="12" x2="22" y2="12"></line><line x1="12" y1="2" x2="12" y2="22"></line>"#),
    ("rotate", r#"<polyline points="23 4 23 10 17 10"></polyline><path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>"#),
    ("scale", r#"<polyline points="15 3 21 3 21 9"></polyline><polyline points="9 21 3 21 3 15"></polyline><line x1="21" y1="3" x2="14" y2="10"></line><line x1="3" y1="21" x2="10" y2="14"></line>"#),
    ("package", r#"<line x1="16.5" y1="9.4" x2="7.5" y2="4.21"></line><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"></path><polyline points="3.27 6.96 12 12.01 20.73 6.96"></polyline><line x1="12" y1="22.08" x2="12" y2="12"></line>"#),
    ("node", r#"<circle cx="12" cy="12" r="4"></circle><line x1="1.05" y1="12" x2="7" y2="12"></line><line x1="17.01" y1="12" x2="22.96" y2="12"></line>"#),
    ("grid", r#"<rect x="3" y="3" width="7" height="7"></rect><rect x="14" y="3" width="7" height="7"></rect><rect x="14" y="14" width="7" height="7"></rect><rect x="3" y="14" width="7" height="7"></rect>"#),
    ("settings", r#"<circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>"#),
    ("open", r#"<line x1="5" y1="12" x2="19" y2="12"></line><polyline points="12 5 19 12 12 19"></polyline>"#),
    ("trash", r#"<polyline points="3 6 5 6 21 6"></polyline><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path><line x1="10" y1="11" x2="10" y2="17"></line><line x1="14" y1="11" x2="14" y2="17"></line>"#),
    ("link", r#"<path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path>"#),
    ("menu", r#"<line x1="3" y1="12" x2="21" y2="12"></line><line x1="3" y1="6" x2="21" y2="6"></line><line x1="3" y1="18" x2="21" y2="18"></line>"#),
    ("info", r#"<circle cx="12" cy="12" r="10"></circle><line x1="12" y1="16" x2="12" y2="12"></line><line x1="12" y1="8" x2="12.01" y2="8"></line>"#),
    ("activity", r#"<polyline points="22 12 18 12 15 21 9 3 6 12 2 12"></polyline>"#),
    ("users", r#"<path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path><circle cx="9" cy="7" r="4"></circle><path d="M23 21v-2a4 4 0 0 0-3-3.87"></path><path d="M16 3.13a4 4 0 0 1 0 7.75"></path>"#),
    ("alert", r#"<path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path><line x1="12" y1="9" x2="12" y2="13"></line><line x1="12" y1="17" x2="12.01" y2="17"></line>"#),
    ("external", r#"<path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path><polyline points="15 3 21 3 21 9"></polyline><line x1="10" y1="14" x2="21" y2="3"></line>"#),
    ("pointer", r#"<path d="M3 3l7.07 16.97 2.51-7.39 7.39-2.51L3 3z"></path><path d="M13 13l6 6"></path>"#),
    ("maximize", r#"<path d="M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3"></path>"#),
    ("square", r#"<rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>"#),
    ("tool", r#"<path d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.77-3.77a6 6 0 0 1-7.94 7.94l-6.91 6.91a2.12 2.12 0 0 1-3-3l6.91-6.91a6 6 0 0 1 7.94-7.94l-3.76 3.76z"></path>"#),
    // the reader's account icon (viewer/src/lib/icons/account.svg), its 100 × 100 drawing scaled into the 24 × 24 box
    ("account", r##"<g fill="#fff" stroke="none" transform="translate(-3.3 -3.8) scale(0.31)"><path d="M49.7,68.3c-3.9,0-6.1,3.2-6.2,3.3L43,72.3l0.5,0.7c0.1,0.1,2.3,3.3,6.2,3.3c4,0,6.2-3.1,6.2-3.3l0.5-0.7l-0.5-0.7C55.8,71.5,53.6,68.3,49.7,68.3z M49.7,73.8c-1.6,0-2.8-0.9-3.5-1.5c0.7-0.6,1.9-1.5,3.5-1.5c1.6,0,2.8,0.9,3.5,1.5C52.5,72.9,51.3,73.8,49.7,73.8z"/><path d="M45,58.5c0-1.2-0.9-2.2-2.2-2.2c-1.2,0-2.2,1-2.2,2.2s1,2.2,2.2,2.2S45,59.7,45,58.5z"/><path d="M56.6,56.3c-1.2,0-2.2,1-2.2,2.2s1,2.2,2.2,2.2c1.2,0,2.2-1,2.2-2.2S57.8,56.3,56.6,56.3z"/><path d="M85.1,33.7c0-4.3-3.5-7.8-7.8-7.8c-2,0-3.8,0.8-5.2,2c-5.6-5.9-13.6-9.5-22.3-9.5C41,18.4,33,22,27.4,27.9c-1.4-1.2-3.2-2-5.2-2c-4.3,0-7.8,3.5-7.8,7.8c0,3.5,2.3,6.5,5.5,7.5c-0.7,2.6-1.1,5.3-1.1,8.2c0,3.7,0.7,7.3,1.9,10.6c-0.1-0.5-0.2-0.9-0.2-1.4C22.8,72.7,35,83.5,49.7,83.5c16.3,0,29.6-13.3,29.6-29.6c0-0.2,0-0.4,0-0.6h0.1c0,0.5,0,1,0,1.5c0,1.8-0.2,3.5-0.5,5.2c1.2-3.3,1.9-6.9,1.9-10.6c0-2.8-0.4-5.6-1.1-8.2C82.8,40.2,85.1,37.3,85.1,33.7z M49.7,81c-15,0-27.1-12.2-27.1-27.1c0-0.2,0-0.4,0-0.6h0.1h54.1c0,0.2,0,0.4,0,0.6C76.8,68.8,64.7,81,49.7,81z"/></g>"##),
    // a panel's drag handle: two columns of dots
    ("grip", r##"<circle fill="#fff" cx="9" cy="5" r="1.6"></circle><circle fill="#fff" cx="15" cy="5" r="1.6"></circle><circle fill="#fff" cx="9" cy="12" r="1.6"></circle><circle fill="#fff" cx="15" cy="12" r="1.6"></circle><circle fill="#fff" cx="9" cy="19" r="1.6"></circle><circle fill="#fff" cx="15" cy="19" r="1.6"></circle>"##),
    // filled, for the one button that starts and stops (owner 2026-11-09: "a play icon and a stop icon like in unreal")
    ("play-fill", r##"<polygon fill="#fff" points="6 3 20 12 6 21 6 3"></polygon>"##),
    ("left-fill", r##"<polygon fill="#fff" points="19 4 7 12 19 20"></polygon>"##),
    ("stop-fill", r##"<rect fill="#fff" x="5" y="5" width="14" height="14" rx="1"></rect>"##),
    ("bolt-fill", r##"<polygon fill="#fff" points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon>"##),
    ("bolt", r#"<polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon>"#),
];

/// is there an icon of this name
pub fn has(name: &str) -> bool {
    ICONS.iter().any(|(n, _)| *n == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ones_the_editor_uses_are_there() {
        for name in ["up", "down", "next", "close", "play-fill", "stop-fill", "grip", "menu", "info", "account", "tool", "pointer", "maximize", "square", "window-min", "window-max", "window-restore"] {
            assert!(has(name), "{name}");
        }
    }

    #[test]
    fn names_are_unique() {
        let mut names: Vec<&str> = ICONS.iter().map(|(n, _)| *n).collect();
        names.sort_unstable();
        let n = names.len();
        names.dedup();
        assert_eq!(n, names.len());
    }
}
