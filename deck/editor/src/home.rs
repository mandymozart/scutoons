//! What opens items from nothing (decisions/deck.md [workspaces]): New (a scene, a module, a show – the ☰ menu's New …),
//! the recent items (the switcher's Last opened) and the help's link (the ☰ menu's Docs). While no tab is open the deck's
//! idle scene plays behind Home (fireflies on black).

/// where the help is (creator.scutoons.com/docs, #624): a press opens it in the browser, or copies it
pub const DOCS: &str = "https://creator.scutoons.com/docs";

/// `url` in the system's browser (a desktop's; a kiosk deck has none, so the caller copies the link instead)
pub fn open_in_browser(url: &str) -> std::io::Result<()> {
    let mut c = if cfg!(windows) {
        let mut c = std::process::Command::new("rundll32");
        c.args(["url.dll,FileProtocolHandler", url]);
        c
    } else if cfg!(target_os = "macos") {
        let mut c = std::process::Command::new("open");
        c.arg(url);
        c
    } else {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(url);
        c
    };
    c.spawn().map(drop)
}

/// what New makes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum New {
    Scene,
    Module,
    Show,
}

/// what opens an item or the help
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    New(New),
    /// a line of Last opened: its index in what was drawn
    Recent(usize),
    Docs,
}

/// one line: an address, its name
pub struct Line {
    pub key: String,
    pub name: String,
}
