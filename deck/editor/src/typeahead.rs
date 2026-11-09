//! A typeahead (#455; owner 2026-11-09: "The type head should be a UI component we can reuse"): a text field over a list,
//! narrowed as you type, chosen by arrows and enter or the mouse. It knows nothing of what it lists: a caller gives items
//! with an id and gets the id back, and can ask which ids the filter still shows (the stage highlights only those).
use crate::text::Field;
use crate::tokens::Tokens;
use crate::ui::Ui;

const WIDTH: f32 = 380.0;
const ROWS: usize = 10;

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub id: usize,
    pub label: String,
    pub detail: String,
}

pub struct Typeahead {
    items: Vec<Item>,
    field: Field,
    /// indices into `items`, what the filter leaves
    shown: Vec<usize>,
    /// the highlighted row, an index into `shown`
    at: usize,
    /// where it opened
    pub x: f32,
    pub y: f32,
    /// this frame's rows: (top, item index)
    rows: Vec<(f32, usize)>,
    frame: (f32, f32, f32),
}

/// every word of `query` somewhere in the item, whatever the case
fn matches(item: &Item, query: &str) -> bool {
    let hay = format!("{} {}", item.label, item.detail).to_lowercase();
    query.to_lowercase().split_whitespace().all(|w| hay.contains(w))
}

impl Typeahead {
    pub fn new(items: Vec<Item>, x: f32, y: f32) -> Typeahead {
        let mut t = Typeahead { items, field: Field::with(""), shown: Vec::new(), at: 0, x, y, rows: Vec::new(), frame: (0.0, 0.0, 0.0) };
        t.field.focused = true;
        t.narrow();
        t
    }

    fn narrow(&mut self) {
        let query = self.field.text.clone();
        self.shown = (0..self.items.len()).filter(|&i| matches(&self.items[i], &query)).collect();
        self.at = self.at.min(self.shown.len().saturating_sub(1));
    }

    pub fn insert(&mut self, text: &str) {
        self.field.insert(text);
        self.narrow();
    }

    pub fn backspace(&mut self) {
        self.field.backspace();
        self.narrow();
    }

    pub fn delete(&mut self) {
        self.field.delete();
        self.narrow();
    }

    pub fn left(&mut self) {
        self.field.left(false, false);
    }

    pub fn right(&mut self) {
        self.field.right(false, false);
    }

    /// the highlight one row up or down, round the ends
    pub fn step(&mut self, by: i32) {
        let n = self.shown.len() as i32;
        if n > 0 {
            self.at = (self.at as i32 + by).rem_euclid(n) as usize;
        }
    }

    /// the highlighted item's id
    pub fn chosen(&self) -> Option<usize> {
        self.shown.get(self.at).map(|&i| self.items[i].id)
    }

    /// does the filter still show this id
    pub fn shows(&self, id: usize) -> bool {
        self.shown.iter().any(|&i| self.items[i].id == id)
    }

    /// the pointer over a row highlights it
    pub fn hover(&mut self, x: f32, y: f32) {
        if let Some(i) = self.row_at(x, y) {
            if let Some(at) = self.shown.iter().position(|&s| s == i) {
                self.at = at;
            }
        }
    }

    fn row_at(&self, x: f32, y: f32) -> Option<usize> {
        let (fx, _, h) = self.frame;
        if x < fx || x > fx + WIDTH {
            return None;
        }
        self.rows.iter().find(|(top, _)| y >= *top && y < *top + h).map(|&(_, i)| i)
    }

    /// the id of the row at this point
    pub fn id_at(&self, x: f32, y: f32) -> Option<usize> {
        self.row_at(x, y).map(|i| self.items[i].id)
    }

    /// draw it, kept inside `limit` (the window, in logical pixels)
    pub fn draw(&mut self, ui: &mut Ui, t: &Tokens, limit: (f32, f32)) {
        let (font, row, pad) = (t.size.font * 0.85, t.size.row * 0.9, t.size.pad * 0.7);
        let visible = self.shown.len().min(ROWS);
        let height = row * (visible.max(1) as f32 + 1.0) + 4.0;
        let x = self.x.min(limit.0 - WIDTH - 8.0).max(8.0);
        let y = self.y.min(limit.1 - height - 8.0).max(t.size.bar + 4.0);
        self.frame = (x, y, row);
        ui.rect(x - 1.0, y - 1.0, WIDTH + 2.0, height + 2.0, t.color.divider);
        ui.rect(x, y, WIDTH, height, t.color.panel);
        ui.rect(x, y, WIDTH, row, t.color.field);
        let text_y = (row - font * 1.25) * 0.5;
        let adv = ui.atlas.advance(font);
        if self.field.text.is_empty() {
            ui.text(x + pad, y + text_y, font, "type to find", t.color.muted);
        } else {
            let (shown, _) = self.field.shown();
            ui.text(x + pad, y + text_y, font, &shown, t.color.text);
        }
        let (_, column) = self.field.shown();
        ui.rect(x + pad + column as f32 * adv, y + 4.0, 1.5, row - 8.0, t.color.text);
        self.rows.clear();
        let first = self.at.saturating_sub(ROWS - 1);
        if self.shown.is_empty() {
            ui.text(x + pad, y + row + text_y, font, "nothing fits", t.color.muted);
        }
        for (slot, &i) in self.shown.iter().skip(first).take(ROWS).enumerate() {
            let top = y + row * (slot as f32 + 1.0) + 2.0;
            if first + slot == self.at {
                ui.rect(x, top, WIDTH, row, t.color.tab_active);
            }
            let item = &self.items[i];
            ui.text(x + pad, top + text_y, font, &item.label, t.color.text);
            let w = adv * item.detail.chars().count() as f32;
            ui.text(x + WIDTH - pad - w, top + text_y, font, &item.detail, t.color.muted);
            self.rows.push((top, i));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> Typeahead {
        let item = |id: usize, label: &str, detail: &str| Item { id, label: label.into(), detail: detail.into() };
        Typeahead::new(vec![item(4, "voice:voice.cv", "in audio"), item(7, "drone:mix.in3", "in audio"), item(9, "drone:drone.gain", "in control")], 0.0, 0.0)
    }

    #[test]
    fn it_narrows_as_you_type_and_gives_the_id_back() {
        let mut t = list();
        assert_eq!(t.chosen(), Some(4));
        t.insert("mix");
        assert_eq!(t.chosen(), Some(7));
        assert!(t.shows(7) && !t.shows(4));
    }

    #[test]
    fn all_words_count_in_any_order_and_any_case() {
        let mut t = list();
        t.insert("CONTROL drone");
        assert_eq!(t.chosen(), Some(9));
    }

    #[test]
    fn taking_a_letter_back_widens_it_again() {
        let mut t = list();
        t.insert("mixx");
        assert_eq!(t.chosen(), None);
        t.backspace();
        assert_eq!(t.chosen(), Some(7));
    }

    #[test]
    fn the_arrows_go_round_the_ends() {
        let mut t = list();
        t.step(-1);
        assert_eq!(t.chosen(), Some(9));
        t.step(1);
        assert_eq!(t.chosen(), Some(4));
    }
}
