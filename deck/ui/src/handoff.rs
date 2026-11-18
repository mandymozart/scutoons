//! F9 between the deck and the editor: why it stays put, in the words the deck's HUD and the editor's status bar say.

/// why F9 did not hand over
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// a lean deck: no `deck-editor` next to `deck`
    NoEditor,
    /// an editor without the stage next to it
    NoDeck,
    /// deck.toml's `editor = "locked"`
    Locked,
    /// the sibling would not start, or the hand-off file could not be written
    Failed(String),
}

impl Refusal {
    /// the sentence (the editor's status bar; the HUD upper-cases it)
    pub fn sentence(&self) -> String {
        match self {
            Refusal::NoEditor => "F9: this deck carries no editor (a lean deck) – the stage stays.".into(),
            Refusal::NoDeck => "F9: no stage next to this editor (deck is not installed here).".into(),
            Refusal::Locked => "F9: the editor is locked on this deck (deck.toml editor = \"locked\") – the stage stays.".into(),
            Refusal::Failed(why) => format!("F9: the switch failed – {why}"),
        }
    }

    /// the HUD's line (font8x8: ASCII, upper case)
    pub fn hud_line(&self) -> String {
        self.sentence().replace(['–', '—'], "-").to_uppercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hud_line_is_ascii_upper_case() {
        let l = Refusal::Locked.hud_line();
        assert!(l.is_ascii(), "{l}");
        assert_eq!(l, l.to_uppercase());
        assert!(Refusal::NoEditor.hud_line().contains("LEAN DECK"));
    }
}
