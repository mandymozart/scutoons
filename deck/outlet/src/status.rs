//! What the HUD says about a Live outlet (#391; was `Status` in deck-native's live/mod.rs): each part is written by one of
//! the outlet's threads (the relay's link line, the preloader's `Preload::done` / `loading_line`, the composer's
//! `Recompose::composed`), and `lines` words them for the HUD (font8x8: upper case, ASCII dashes) – the same on every deck.

/// What the HUD says about the outlet (each part written by its thread).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Status {
    /// the relay: joining, the client id, offline and why
    pub link: String,
    /// the preload: n / m ready, the first missing one's reason
    pub media: String,
    /// the cell: its layers, fading
    pub cell: String,
    /// what the current cell can't show (layer: reason)
    pub problems: Vec<String>,
    /// the relay's last refusal
    pub problems_relay: Option<String>,
}

impl Status {
    /// the HUD's lines (font8x8: upper case, ASCII dashes)
    pub fn lines(&self, outlet: &str) -> Vec<String> {
        let up = |s: &str| s.replace(['–', '—'], "-").to_uppercase();
        let mut out = vec![up(&format!("LIVE {outlet}  {}", self.link))];
        if !self.media.is_empty() {
            out.push(up(&format!("MEDIA {}", self.media)));
        }
        if !self.cell.is_empty() {
            out.push(up(&format!("CELL {}", self.cell)));
        }
        out.extend(self.problems.iter().chain(&self.problems_relay).map(|p| up(&format!("! {p}"))));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lines_are_upper_case_ascii_and_only_what_is_known() {
        let mut s = Status { link: "joining wss://x – y".into(), ..Default::default() };
        assert_eq!(s.lines("left"), ["LIVE LEFT  JOINING WSS://X - Y"]);
        s.media = "1 / 2 READY".into();
        s.cell = "3 layers + space 12 ms".into();
        s.problems = vec!["panel: not preloaded".into()];
        s.problems_relay = Some("RELAY forbidden: no".into());
        assert_eq!(
            s.lines("left"),
            ["LIVE LEFT  JOINING WSS://X - Y", "MEDIA 1 / 2 READY", "CELL 3 LAYERS + SPACE 12 MS", "! PANEL: NOT PRELOADED", "! RELAY FORBIDDEN: NO"]
        );
    }
}
