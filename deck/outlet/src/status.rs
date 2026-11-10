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
    /// #91: what the deck's piece side did with the last cell, and why a piece does not play
    pub piece: Option<String>,
    /// #91: the deck's answer to the last flash (deck-show `ready::line`) and what it lacks, one line each
    pub flash: Option<String>,
    pub flash_lacks: Vec<String>,
    /// #471: "please don't flash", the notice after a flash replaced the deck's own version and how the switch stands
    /// (deck-show `ready::Deck::keep_line`, `notice`)
    pub flash_notice: Vec<String>,
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
        if let Some(p) = &self.piece {
            out.push(up(&format!("PIECE {p}")));
        }
        if let Some(f) = &self.flash {
            out.push(up(f));
        }
        out.extend(self.flash_lacks.iter().map(|l| up(&format!("! {l}"))));
        out.extend(self.flash_notice.iter().map(|l| up(l)));
        out.extend(self.problems.iter().chain(&self.problems_relay).map(|p| up(&format!("! {p}"))));
        out
    }

    /// #91: the piece side's answer to a cell (deck-space piece.rs) in the HUD's words: what it does, then why not
    pub fn set_piece(&mut self, did: &str, problems: &[String]) {
        let why = problems.iter().map(String::as_str).collect::<Vec<_>>().join("; ");
        self.piece = Some(if why.is_empty() { did.to_string() } else { format!("{did} – {why}") });
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
        s.problems.clear();
        s.problems_relay = None;
        s.flash = Some("FLASH top@3: 1 missing".into());
        s.flash_lacks = vec!["row exit: scene:ghost is not on this deck".into()];
        assert_eq!(s.lines("left")[3..], ["FLASH TOP@3: 1 MISSING".to_string(), "! ROW EXIT: SCENE:GHOST IS NOT ON THIS DECK".to_string()]);
        s.flash_notice = vec!["please don't flash: rehearsing".into(), "flashed by force by ana – your top@2 is kept".into()];
        assert_eq!(s.lines("left")[5..], ["PLEASE DON'T FLASH: REHEARSING".to_string(), "FLASHED BY FORCE BY ANA - YOUR TOP@2 IS KEPT".to_string()]);
    }
}
