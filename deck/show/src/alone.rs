//! A show one deck plays on its own, as its own conductor: the deck's Play lists the published shows
//! (`GET /shows/published?toon=`), a show picked there runs on the same [`Subtree`] the editor's Show tab runs – it fires
//! the rows and sends full cells – and the cells for this deck's outlet go to the deck's piece side, as a relay's would.
//! No relay: the deck is its whole session. Its first stage outlet is the one it plays; GO fires the next row.
use crate::document::{OutletKind, Show};
use crate::subtree::{ClockChoice, Subtree};
use deck_ui::start::{Listed, Showing};
use protocol::{Cell, CellMode, Envelope, Message};
use serde_json::Value;

/// the clock domain a deck's own show plays on
pub const CLOCK: &str = "deck";

/// the API's path of the published shows of `toon`
pub fn path(toon: &str) -> String {
    format!("/shows/published?toon={toon}")
}

/// The published shows in the API's answer (`{ shows: [{ key: "show:<id>", content }] }`) as Play lists them; a body
/// that is not one: none.
pub fn listing(body: &str) -> Vec<Listed> {
    rows(body)
        .into_iter()
        .map(|(id, doc)| {
            let toon = doc.get("toon").and_then(Value::as_str).unwrap_or_default().to_string();
            Listed { name: id.clone(), id, toons: vec![toon], version: None, show: true }
        })
        .collect()
}

/// The published show `id` from the same answer, checked as a document on its own (validate.rs); why not.
pub fn find(body: &str, id: &str) -> Result<Show, String> {
    let doc = rows(body).into_iter().find(|(i, _)| i == id).map(|(_, d)| d).ok_or_else(|| format!("show:{id} is not published"))?;
    crate::document::parse(&doc.to_string()).map_err(|e| format!("show:{id}: {e}"))
}

fn rows(body: &str) -> Vec<(String, Value)> {
    let Ok(v) = serde_json::from_str::<Value>(body) else { return vec![] };
    let rows = v.get("shows").and_then(Value::as_array).cloned().unwrap_or_default();
    rows.into_iter()
        .filter_map(|r| {
            let id = r.get("key")?.as_str()?.strip_prefix("show:")?.to_string();
            let doc = r.get("content").filter(|c| c.is_object())?.clone();
            (!id.is_empty()).then_some((id, doc))
        })
        .collect()
}

/// One show this deck conducts for itself.
#[derive(Debug, Clone)]
pub struct Alone {
    subtree: Subtree,
    outlet: String,
}

impl Alone {
    /// The show on this deck: it plays the show's first stage outlet; a show without one cannot play here.
    pub fn new(show: Show) -> Result<Alone, String> {
        let outlet = show.outlets.iter().find(|o| o.kind == OutletKind::Stage).map(|o| o.id.clone()).ok_or_else(|| format!("show:{} has no stage outlet", show.id))?;
        Ok(Alone { subtree: Subtree::new(show, ClockChoice::Own(CLOCK.into())), outlet })
    }

    /// the outlet this deck plays
    pub fn outlet(&self) -> &str {
        &self.outlet
    }

    /// the pieces this deck's outlet plays, each once, in row order (to fetch before the first row)
    pub fn needs(&self) -> Vec<String> {
        crate::ready::needs(self.subtree.conductor().show(), &self.outlet).into_iter().map(|n| n.piece).collect()
    }

    /// GO: the next row (the first at the start); the cells for this deck's outlet
    pub fn go(&mut self, ms: u64) -> Vec<Cell> {
        let out = self.subtree.go(ms);
        self.mine(out)
    }

    /// Time passes (a row's hold, its moment); the cells for this deck's outlet
    pub fn tick(&mut self, ms: u64) -> Vec<Cell> {
        let out = self.subtree.tick(ms);
        self.mine(out)
    }

    /// what the show page says
    pub fn showing(&self) -> Showing {
        let show = self.subtree.conductor().show();
        let row = self.subtree.conductor().current().and_then(|c| show.rows.iter().position(|r| r.id == c)).map_or(0, |i| i + 1);
        Showing { name: show.id.clone(), row, rows: show.rows.len() }
    }

    /// the cell that ends the show on this deck: black, what played fades
    pub fn leave() -> Cell {
        Cell { mode: CellMode::Black, layers: None, of: None, send: None }
    }

    fn mine(&self, out: Vec<Envelope>) -> Vec<Cell> {
        out.into_iter()
            .filter_map(|e| match e.message {
                Message::Cell(c) if c.outlet == self.outlet => Some(c.cell),
                Message::Panic(p) if p.black == Some(true) => Some(Alone::leave()),
                _ => None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::{LayerSrc, PieceSrc};

    const NEW_SCENE: &str = include_str!("../../demos/new-scene.show.json");
    const FAR: &str = include_str!("../../demos/far-nodes.show.json");

    fn body() -> String {
        let doc = |text: &str, id: &str| {
            let mut v: Value = serde_json::from_str(text).unwrap();
            v["id"] = id.into();
            v["toon"] = "sandbox".into();
            v
        };
        serde_json::json!({ "status": "success", "shows": [
            { "key": "show:demo-new-scene", "content": doc(NEW_SCENE, "demo-new-scene") },
            { "key": "show:demo-far-nodes", "content": doc(FAR, "demo-far-nodes") },
            { "key": "show:broken", "content": { "id": "broken" } },
            { "key": "scene:nave", "content": {} } ] })
        .to_string()
    }

    /// the piece a cell plays, if it plays one
    fn piece(cell: &Cell) -> Option<String> {
        cell.layers.as_ref()?.iter().find_map(|l| match &l.src {
            LayerSrc::Piece(PieceSrc { piece, .. }) => Some(piece.clone()),
            _ => None,
        })
    }

    #[test]
    fn the_listing_names_the_published_shows_by_toon() {
        let l = listing(&body());
        assert_eq!(l.iter().map(|l| (l.id.as_str(), l.show, l.toons[0].as_str())).collect::<Vec<_>>(), [("demo-new-scene", true, "sandbox"), ("demo-far-nodes", true, "sandbox"), ("broken", true, "")]);
        assert!(listing("<html>").is_empty() && listing("{}").is_empty());
        assert_eq!(path("sandbox"), "/shows/published?toon=sandbox");
        assert!(find(&body(), "broken").is_err(), "a document that does not open says why");
        assert_eq!(find(&body(), "gone").unwrap_err(), "show:gone is not published");
    }

    #[test]
    fn a_picked_show_drives_this_deck_s_outlet_row_by_row() {
        let mut a = Alone::new(find(&body(), "demo-far-nodes").unwrap()).unwrap();
        assert_eq!(a.outlet(), "left", "the first stage outlet");
        assert_eq!(a.needs(), ["character:keys"]);
        assert_eq!(a.showing(), Showing { name: "demo-far-nodes".into(), row: 0, rows: 3 });
        let cells = a.go(0);
        assert_eq!(cells.iter().filter_map(piece).collect::<Vec<_>>(), ["character:keys"], "row 1 plays its piece on this deck");
        assert_eq!(a.showing().row, 1);
        let cells = a.go(1000);
        assert_eq!(cells.len(), 1, "row 2: a values cell on this outlet, a full cell");
        assert_eq!(piece(&cells[0]).as_deref(), Some("character:keys"));
        let cells = a.go(2000);
        assert_eq!(cells.iter().map(|c| c.mode).collect::<Vec<_>>(), [CellMode::Black], "row 3: black");
        assert!(a.go(3000).is_empty(), "after the last row nothing fires");
        assert_eq!(a.showing().row, 3);
        assert!(a.tick(4000).is_empty());
        assert_eq!(Alone::leave().mode, CellMode::Black);
    }
}
