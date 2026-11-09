//! A text field's model (#444, the gate: umlauts, dead keys, an IME, selection, copy and paste): one line of text with a
//! cursor and a selection anchor in byte offsets on char boundaries, the IME's preedit shown at the cursor until it
//! commits. The keys come from winit as the characters they produce (`KeyEvent::text` – dead keys and umlauts arrive
//! composed) or as named keys; the clipboard is the OS's (arboard).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Field {
    pub text: String,
    /// the cursor, a byte offset on a char boundary
    pub cursor: usize,
    /// the selection's other end (none = no selection)
    pub anchor: Option<usize>,
    /// the IME's composition at the cursor, and the cursor inside it (bytes)
    pub preedit: Option<(String, Option<(usize, usize)>)>,
    pub focused: bool,
}

impl Field {
    pub fn with(text: &str) -> Field {
        Field { text: text.to_string(), cursor: text.len(), ..Field::default() }
    }

    /// the selection as (from, to) in bytes, when there is one
    pub fn selection(&self) -> Option<(usize, usize)> {
        let a = self.anchor?;
        (a != self.cursor).then(|| (a.min(self.cursor), a.max(self.cursor)))
    }

    pub fn selected(&self) -> &str {
        self.selection().map_or("", |(a, b)| &self.text[a..b])
    }

    fn delete_selection(&mut self) -> bool {
        let Some((a, b)) = self.selection() else {
            self.anchor = None;
            return false;
        };
        self.text.replace_range(a..b, "");
        self.cursor = a;
        self.anchor = None;
        true
    }

    /// typed text (a character, an IME's commit, a paste) replaces the selection
    pub fn insert(&mut self, s: &str) {
        self.delete_selection();
        self.text.insert_str(self.cursor, s);
        self.cursor += s.len();
        self.preedit = None;
    }

    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        if let Some(prev) = self.prev_boundary(self.cursor) {
            self.text.replace_range(prev..self.cursor, "");
            self.cursor = prev;
        }
    }

    pub fn delete(&mut self) {
        if self.delete_selection() {
            return;
        }
        if let Some(next) = self.next_boundary(self.cursor) {
            self.text.replace_range(self.cursor..next, "");
        }
    }

    fn prev_boundary(&self, at: usize) -> Option<usize> {
        self.text[..at].char_indices().next_back().map(|(i, _)| i)
    }

    fn next_boundary(&self, at: usize) -> Option<usize> {
        self.text[at..].chars().next().map(|c| at + c.len_utf8())
    }

    /// the previous word's start (ctrl + left)
    fn prev_word(&self, at: usize) -> usize {
        let before = &self.text[..at];
        let trimmed = before.trim_end_matches(|c: char| !c.is_alphanumeric());
        trimmed.rfind(|c: char| !c.is_alphanumeric()).map_or(0, |i| i + before[i..].chars().next().map_or(1, char::len_utf8))
    }

    /// the next word's end (ctrl + right)
    fn next_word(&self, at: usize) -> usize {
        let after = &self.text[at..];
        let skipped = after.len() - after.trim_start_matches(|c: char| !c.is_alphanumeric()).len();
        let rest = &after[skipped..];
        at + skipped + rest.find(|c: char| !c.is_alphanumeric()).unwrap_or(rest.len())
    }

    /// move the cursor (`select` keeps the anchor: shift held)
    pub fn go(&mut self, to: usize, select: bool) {
        if select {
            self.anchor.get_or_insert(self.cursor);
        } else {
            self.anchor = None;
        }
        self.cursor = to.min(self.text.len());
    }

    pub fn left(&mut self, word: bool, select: bool) {
        if !select && !word {
            if let Some((a, _)) = self.selection() {
                self.go(a, false);
                return;
            }
        }
        let to = if word { self.prev_word(self.cursor) } else { self.prev_boundary(self.cursor).unwrap_or(0) };
        self.go(to, select);
    }

    pub fn right(&mut self, word: bool, select: bool) {
        if !select && !word {
            if let Some((_, b)) = self.selection() {
                self.go(b, false);
                return;
            }
        }
        let to = if word { self.next_word(self.cursor) } else { self.next_boundary(self.cursor).unwrap_or(self.text.len()) };
        self.go(to, select);
    }

    pub fn home(&mut self, select: bool) {
        self.go(0, select);
    }

    pub fn end(&mut self, select: bool) {
        self.go(self.text.len(), select);
    }

    pub fn select_all(&mut self) {
        self.anchor = Some(0);
        self.cursor = self.text.len();
    }

    /// the IME composes: shown at the cursor until it commits (`insert`) or is cancelled (an empty preedit)
    pub fn preedit(&mut self, text: String, cursor: Option<(usize, usize)>) {
        self.preedit = (!text.is_empty()).then_some((text, cursor));
    }

    /// the char index of a byte offset (for drawing)
    pub fn chars_before(&self, at: usize) -> usize {
        self.text[..at.min(self.text.len())].chars().count()
    }

    /// the byte offset of the char boundary nearest to char column `col` (a click)
    pub fn byte_at_char(&self, col: usize) -> usize {
        self.text.char_indices().nth(col).map_or(self.text.len(), |(i, _)| i)
    }

    /// the text as drawn: with the preedit at the cursor, and where the cursor is in it (chars)
    pub fn shown(&self) -> (String, usize) {
        match &self.preedit {
            Some((p, c)) => {
                let mut s = self.text.clone();
                s.insert_str(self.cursor, p);
                let inside = c.map_or(p.len(), |(_, end)| end.min(p.len()));
                let cursor = self.chars_before(self.cursor) + p[..inside].chars().count();
                (s, cursor)
            }
            None => (self.text.clone(), self.chars_before(self.cursor)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_selecting_and_deleting_keep_char_boundaries() {
        let mut f = Field::default();
        f.insert("Grüße ");
        f.insert("an");
        assert_eq!((f.text.as_str(), f.cursor), ("Grüße an", 10), "ü and ß are two bytes each");
        f.left(false, false);
        f.left(false, true);
        f.left(false, true);
        assert_eq!(f.selected(), " a");
        f.insert("ö");
        assert_eq!(f.text, "Grüßeön");
        f.home(false);
        f.right(false, false);
        f.right(false, false);
        f.delete();
        assert_eq!(f.text, "Grßeön");
        f.backspace();
        f.backspace();
        assert_eq!(f.text, "ßeön");
        f.select_all();
        assert_eq!(f.selected(), "ßeön");
        f.backspace();
        assert!(f.text.is_empty() && f.cursor == 0);
    }

    #[test]
    fn words_and_the_preedit() {
        let mut f = Field::with("one two  drei");
        f.end(false);
        f.left(true, false);
        assert_eq!(f.cursor, 9, "before 'drei'");
        f.left(true, true);
        assert_eq!(f.selected(), "two  ");
        f.home(false);
        f.right(true, false);
        assert_eq!(f.cursor, 3);
        f.end(false);
        f.preedit("ü".into(), Some((1, 2)));
        let (shown, cursor) = f.shown();
        assert_eq!((shown.as_str(), cursor), ("one two  dreiü", 14));
        f.insert("ü");
        assert_eq!((f.text.as_str(), f.preedit.is_none()), ("one two  dreiü", true));
        f.preedit(String::new(), None);
        assert!(f.preedit.is_none());
        assert_eq!(f.byte_at_char(4), 4);
        assert_eq!(f.byte_at_char(99), f.text.len());
    }
}
