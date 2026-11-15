//! A show cell's piece in Build (#568; owner 2026-11-15 [workspaces]): the cell inspector's "open in Build" hands Build the
//! rig the cell's piece plays – a rig, character or item as the core loads it (deck-space holders.rs `opened`), a scene's
//! first cast member with a rig (published.rs `Cast::opened`, the scene's cached copy the knobs' loader made) – and keeps
//! the link to the cell: the top bar says it, "values → cell" writes Build's knob positions into the cell through the
//! rows' `Edit::Take` (the piece's slots and ranges are deck-show's, values.rs `from_build`). Which far node's holder plays
//! on which outlet of a row is deck-show's (far.rs); here it is asked with the loader's reach and handed to the drawing.
use super::Host;
use deck_show::far::{self, Link};
use deck_show::rows::{self, Edit};
use deck_show::values;
use deck_space::holders::{self, Opened, Reach};
use std::collections::HashMap;

/// the cell inspector's button, Build's two buttons while it is linked
pub const OPEN: &str = "open in Build";
pub const TO_CELL: &str = "values → cell";
pub const BACK: &str = "back to Show";

/// Build linked to a show cell: the cell, its piece, the rig open (`character:keys`) and its slots' prefix in the piece.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildLink {
    pub row: String,
    pub column: String,
    pub piece: String,
    pub place: String,
    pub prefix: String,
}

/// What Build is asked to play: a rig at hand with the values the cell sets (its rig's slots), or one the server's
/// catalog has (rigs.rs opens it).
pub enum ToBuild {
    Play(Box<Opened>, Vec<(String, f32)>),
    Server(String),
}

impl Host {
    /// the link's words for the top bar: `row 2 together · left` (and the member, when it is not the piece itself)
    pub fn build_line(&self) -> Option<String> {
        let l = self.build.as_ref()?;
        let n = self.shown().rows.iter().position(|r| r.id == l.row).map_or(0, |i| i + 1);
        let member = if l.place == l.piece { String::new() } else { format!(" · {}", l.place) };
        Some(format!("row {n} {} · {}{member}", l.row, l.column))
    }

    /// the press on "open in Build": what Build plays waits in `to_build` for the editor (main.rs), or why not
    pub(super) fn ask_build(&mut self) {
        match self.open_in_build() {
            Ok(t) => self.to_build = Some(t),
            Err(why) => self.status = why,
        }
    }

    /// The selected cell's piece for Build, and the link kept.
    pub fn open_in_build(&mut self) -> Result<ToBuild, String> {
        let (Some(row), Some(column)) = self.selected() else { return Err("select a cell first".into()) };
        let piece = rows::cell_inspector(self.shown(), &row, &column).and_then(|i| i.knobs_of).ok_or("the cell plays no piece – nothing to open in Build")?;
        let link = |place: String, prefix: String| BuildLink { row: row.clone(), column: column.clone(), piece: piece.clone(), place, prefix };
        let opened = match deck_space::published::SceneRef::parse(&piece) {
            Some(r) => {
                let file = self.knobs.cache.join("scenes").join(r?.cache_name());
                let text = std::fs::read_to_string(file).map_err(|_| format!("{piece}: not loaded yet – {}", values::LOADING))?;
                let cast = deck_space::published::Cast::from_cache(&text)?;
                cast.opened().into_iter().next().ok_or_else(|| format!("{piece}: no member of its cast has a rig to open"))?
            }
            None => match holders::opened(&piece, |p| super::values::rig_file(self.knobs.rigs.as_deref(), p)) {
                Ok(o) => o,
                Err(why) if why.ends_with("is not on this deck") => {
                    let place = deck_space::rig::address::Place::parse(&piece).map_err(|e| format!("{piece}: {e}"))?;
                    self.build = Some(link(piece.clone(), String::new()));
                    self.status = format!("{piece}: asking the server for its rig");
                    return Ok(ToBuild::Server(format!("{}.{}", place.kind, place.id)));
                }
                Err(why) => return Err(why),
            },
        };
        // the knobs as the cell sets them, by the rig's own slots (the member's prefix off)
        let set = match self.knobs.get(&piece) {
            Some(Ok(r)) => rows::cell_knobs(self.shown(), &row, &column, &r.knobs, "en").into_iter().filter(|k| k.set).filter_map(|k| Some((k.path.strip_prefix(&opened.prefix)?.to_string(), k.value))).collect(),
            _ => Vec::new(),
        };
        self.build = Some(link(opened.place.clone(), opened.prefix.clone()));
        self.status = format!("{}: open in Build", self.build_line().unwrap_or_default());
        Ok(ToBuild::Play(Box::new(opened), set))
    }

    /// "values → cell": Build's knob positions (its rig's slots) into the linked cell, as the piece names them. Whether it
    /// was written; the status says what.
    pub fn values_to_cell(&mut self, positions: &[(String, f32)]) -> bool {
        let Some(l) = self.build.clone() else {
            self.status = "Build is not linked to a cell – open one from Show's cell inspector".into();
            return false;
        };
        let read = match self.knobs.get(&l.piece) {
            Some(Ok(r)) => r,
            Some(Err(why)) => {
                self.status = why;
                return false;
            }
            None => {
                self.status = values::LOADING.into();
                return false;
            }
        };
        let (taken, left) = values::from_build(positions, &l.prefix, &read.knobs);
        if taken.is_empty() {
            self.status = format!("none of Build's knobs turns a knob of {}", l.piece);
            return false;
        }
        let done = self.edit_rows_with(&Edit::Take { row: l.row, column: l.column, positions: taken }, Some(&read.knobs));
        if done && left > 0 {
            self.status = format!("{} ({left} of Build's knobs are not the piece's)", self.status);
        }
        done
    }

    /// The far nodes of row `row`'s pieces and where their holders play (deck-show far.rs); a piece's reach is asked of the
    /// loader, one not known yet waits.
    pub fn row_links(&mut self, row: &str) -> Vec<Link> {
        let show = self.shown().clone();
        let Some(i) = show.rows.iter().position(|r| r.id == row) else { return Vec::new() };
        let mut got: HashMap<String, Reach> = HashMap::new();
        for (_, p) in far::on_row(&show, i) {
            if let Some(Ok(r)) = self.knobs.get(&p) {
                got.insert(p, r.reach.clone());
            }
        }
        far::links(&show, row, |p| got.get(p).cloned())
    }

    /// Build's far wires while it is linked: the linked cell's far nodes as Build's node ids (the member's prefix off)
    pub fn build_wires(&mut self) -> Vec<Link> {
        let Some(l) = self.build.clone() else { return Vec::new() };
        self.row_links(&l.row)
            .into_iter()
            .filter(|x| x.from == l.column)
            .filter_map(|mut x| {
                x.node = x.node.strip_prefix(&l.prefix)?.to_string();
                Some(x)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::super::rows::tests::{host_with, press};
    use super::super::values::KnobHit;
    use super::super::Hit;
    use super::ToBuild;
    use deck_show::far::Holder;

    const SHOW: &str = include_str!("../../../show/fixtures/far-outlets.show.json");

    fn host() -> super::Host {
        let mut host = host_with(SHOW);
        host.knobs.rigs = Some(concat!(env!("CARGO_MANIFEST_DIR"), "/../show/fixtures/rigs").into());
        for p in ["character:keys", "character:bells", "item:incense_burner"] {
            assert!(matches!(host.knobs.now(p), Some(Ok(_))), "{p}: {:?}", host.knobs.now(p).map(|a| a.err()));
        }
        host
    }

    fn select(host: &mut super::Host, row: usize, column: &str) {
        host.row = row;
        let c = deck_show::view::plan(host.shown()).columns.iter().position(|c| c.id == column).unwrap();
        press(host, false, |h| *h == Hit::Cell(row, c));
    }

    /// the cell inspector's "open in Build": Build gets the cell's rig, the link names the cell; "values → cell" writes
    /// Build's knobs into it through the core (a knob out of its range is refused)
    #[test]
    fn a_cell_opens_in_build_and_builds_values_go_back_into_it() {
        let mut host = host();
        select(&mut host, 1, "left");
        press(&mut host, false, |h| *h == Hit::Knob(KnobHit::Build));
        let Some(ToBuild::Play(opened, set)) = host.to_build.take() else { panic!("Build gets a rig: {}", host.status) };
        assert_eq!(opened.bundle.rig.subject, "character.keys");
        assert_eq!(set, [("tone_osc.freq".to_string(), 440.0)], "Build starts from the cell's values");
        assert_eq!(host.build_line().as_deref(), Some("row 2 together · left"));
        assert_eq!(host.status, "row 2 together · left: open in Build");
        // Build turned the keys: their gain and pitch go back, a knob the piece lacks is counted
        let positions = [("tone_osc.gain".to_string(), 0.65), ("tone_osc.freq".to_string(), 523.0), ("far_in.x".to_string(), 0.0)];
        assert!(host.values_to_cell(&positions), "{}", host.status);
        assert!(host.status.starts_with("row 2 together, left: took 2 knobs") && host.status.ends_with("(1 of Build's knobs are not the piece's)"), "{}", host.status);
        let Some(deck_show::Cell::Values { values, .. }) = host.shown().row("together").unwrap().cells.get("left") else { panic!() };
        assert_eq!(values.get("tone:osc.freq").and_then(|v| v.as_f64()), Some(523.0));
        assert!(!host.values_to_cell(&[("tone_osc.freq".to_string(), 9000.0)]), "out of range");
        assert!(host.status.contains("goes from"), "{}", host.status);
        // a cell without a piece opens nothing; a rig the deck lacks is the server's
        select(&mut host, 2, "left");
        assert!(host.open_in_build().is_err());
        host.build = None;
        assert!(!host.values_to_cell(&positions));
        assert!(host.status.starts_with("Build is not linked"));
    }

    /// the far nodes: in Build the linked cell's, towards the outlet its holder plays on; in Show, the row's pairs
    #[test]
    fn far_wires_follow_the_core_in_build_and_in_show() {
        let mut host = host();
        select(&mut host, 1, "left");
        host.ask_build();
        let wires = host.build_wires();
        assert_eq!(wires.iter().map(|w| (w.node.as_str(), &w.holder)).collect::<Vec<_>>(), [("far_in", &Holder::Outlet("right".into()))]);
        let row = host.row_links("together");
        assert_eq!(row.iter().map(|l| (l.from.as_str(), &l.holder)).collect::<Vec<_>>(), [("left", &Holder::Outlet("right".into())), ("right", &Holder::Outlet("left".into()))]);
        assert_eq!(host.row_links("keys-alone").iter().map(|l| &l.holder).collect::<Vec<_>>(), [&Holder::Away]);
    }
}
