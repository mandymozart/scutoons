//! The conductor's view of the decks' sync (#195): what it pushed (`scene.sync`) and each deck's last answer
//! (`deck.synced`) – the word over each deck's being (`READY`, `SYNC 3/7`, `MISSING 1`, `OFFLINE`, `WAITING`), the HUD's
//! line (`SYNC CATHEDRAL 2/3 READY`) and the cue's question: are all decks ready (`all_ready`)?
use protocol::{DeckSynced, SceneSync, SyncState};
use std::collections::HashMap;
use std::fmt::Write;

/// one deck's state for the push the conductor knows
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// pushed, no answer yet (or an answer to another scene)
    Waiting,
    Syncing { done: u64, total: u64 },
    Ready,
    Missing(usize),
    Offline,
}

impl View {
    /// the word over the deck's being (≤ 24 characters, a label)
    pub fn write(&self, out: &mut impl Write) -> std::fmt::Result {
        match self {
            View::Waiting => out.write_str("SYNC WAITING"),
            View::Syncing { done, total } => write!(out, "SYNC {done}/{total}"),
            View::Ready => out.write_str("READY"),
            View::Missing(n) => write!(out, "MISSING {n}"),
            View::Offline => out.write_str("OFFLINE"),
        }
    }

    /// its colour's hue (hsv): green ready, amber on the way, red when something is not there
    pub fn hue(&self) -> f32 {
        match self {
            View::Ready => 0.33,
            View::Syncing { .. } | View::Waiting => 0.12,
            View::Missing(_) | View::Offline => 0.0,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Board {
    /// the last `scene.sync` (this conductor's or another's, from the relay's snapshot)
    pub pushed: Option<SceneSync>,
    /// each deck's last answer to it, by client id
    pub synced: HashMap<String, DeckSynced>,
}

impl Board {
    /// a push (this conductor's, or one it learns of – the relay forgets the answers to the one before, so does this)
    pub fn pushed(&mut self, sync: SceneSync) {
        self.synced.clear();
        self.pushed = Some(sync);
    }

    /// a deck's answer – kept when it answers the push this board knows (or none is known yet)
    pub fn answer(&mut self, client: String, synced: DeckSynced) {
        if self.pushed.as_ref().map_or(true, |p| p.scene == synced.scene) {
            self.synced.insert(client, synced);
        }
    }

    /// a deck left
    pub fn gone(&mut self, client: &str) {
        self.synced.remove(client);
    }

    /// is `client` one the push is for (every deck, or the listed ones)?
    pub fn addressed(&self, client: &str) -> bool {
        self.pushed.as_ref().is_some_and(|p| p.decks.as_ref().map_or(true, |d| d.iter().any(|c| c == client)))
    }

    /// `client`'s state; None = nothing pushed, or the push isn't for it
    pub fn view(&self, client: &str) -> Option<View> {
        let p = self.pushed.as_ref().filter(|_| self.addressed(client))?;
        let Some(r) = self.synced.get(client).filter(|r| r.scene == p.scene) else { return Some(View::Waiting) };
        Some(match r.state {
            SyncState::Syncing => View::Syncing { done: r.done.unwrap_or(0), total: r.total.unwrap_or(0) },
            SyncState::Ready => View::Ready,
            SyncState::Missing => View::Missing(r.missing.len()),
            SyncState::Offline => View::Offline,
        })
    }

    /// of the decks there are (`decks`: client ids), how many the push is for and how many of those are ready
    pub fn count<'a>(&self, decks: impl IntoIterator<Item = &'a str>) -> (usize, usize) {
        let mut of = 0;
        let mut ready = 0;
        for d in decks.into_iter().filter(|d| self.addressed(d)) {
            of += 1;
            ready += usize::from(self.view(d) == Some(View::Ready));
        }
        (ready, of)
    }

    /// The cue's wait (#195): something was pushed, at least one deck it is for is there, and every one of them is
    /// ready. A cue that waits for the decks holds while this is false.
    pub fn all_ready<'a>(&self, decks: impl IntoIterator<Item = &'a str>) -> bool {
        let (ready, of) = self.count(decks);
        of > 0 && ready == of
    }

    /// the HUD's line: `SYNC CATHEDRAL 2/3 READY` (+ how many miss something / are offline); None = nothing pushed
    pub fn line<'a>(&self, decks: impl IntoIterator<Item = &'a str> + Clone) -> Option<String> {
        let p = self.pushed.as_ref()?;
        let (ready, of) = self.count(decks.clone());
        let bad = decks.into_iter().filter(|d| matches!(self.view(d), Some(View::Missing(_) | View::Offline))).count();
        let mut line = format!("SYNC {} {ready}/{of} READY", p.scene);
        if bad > 0 {
            let _ = write!(line, " {bad} NOT");
        }
        Some(line.to_uppercase().chars().take(40).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn push(decks: Option<&[&str]>) -> SceneSync {
        SceneSync {
            scene: "cathedral".into(),
            sets: Some(vec!["cathedral".into()]),
            characters: None,
            beings: None,
            decks: decks.map(|d| d.iter().map(|s| s.to_string()).collect()),
        }
    }

    fn answer(scene: &str, state: SyncState, done: u64, missing: usize) -> DeckSynced {
        DeckSynced {
            scene: scene.into(),
            state,
            done: Some(done),
            total: Some(4),
            bytes: None,
            ready: vec![],
            missing: (0..missing).map(|i| format!("character:c{i}")).collect(),
            why: None,
        }
    }

    #[test]
    fn each_deck_s_word_and_the_cue_s_wait() {
        let mut b = Board::default();
        assert_eq!(b.view("d1"), None, "nothing pushed");
        assert!(!b.all_ready(["d1"]));
        b.pushed(push(None));
        assert_eq!(b.view("d1"), Some(View::Waiting));
        b.answer("d1".into(), answer("cathedral", SyncState::Syncing, 1, 0));
        b.answer("d2".into(), answer("cathedral", SyncState::Ready, 4, 0));
        b.answer("d3".into(), answer("cathedral", SyncState::Missing, 4, 2));
        let word = |v: View| {
            let mut s = String::new();
            v.write(&mut s).unwrap();
            s
        };
        assert_eq!(word(b.view("d1").unwrap()), "SYNC 1/4");
        assert_eq!(word(b.view("d2").unwrap()), "READY");
        assert_eq!(word(b.view("d3").unwrap()), "MISSING 2");
        assert_eq!(b.line(["d1", "d2", "d3"]).unwrap(), "SYNC CATHEDRAL 1/3 READY 1 NOT");
        assert!(!b.all_ready(["d1", "d2", "d3"]));
        b.answer("d1".into(), answer("cathedral", SyncState::Ready, 4, 0));
        b.gone("d3");
        assert!(b.all_ready(["d1", "d2"]), "the decks there are, all ready");
        assert!(!b.all_ready(std::iter::empty()), "no deck: nothing to wait for is not ready");
        // an answer to another scene doesn't count; a new push forgets the old answers
        b.answer("d1".into(), answer("sermon", SyncState::Ready, 4, 0));
        assert_eq!(b.view("d1"), Some(View::Ready), "the old answer stays – the sermon's isn't kept");
        b.pushed(SceneSync { scene: "sermon".into(), ..push(None) });
        assert_eq!(b.view("d1"), Some(View::Waiting));
        assert!(!b.all_ready(["d1", "d2"]));
    }

    #[test]
    fn a_push_to_a_list() {
        let mut b = Board::default();
        b.pushed(push(Some(&["d1"])));
        assert_eq!(b.view("d2"), None, "not for it");
        b.answer("d1".into(), answer("cathedral", SyncState::Ready, 4, 0));
        assert!(b.all_ready(["d1", "d2"]), "only the listed decks count");
        assert_eq!(b.line(["d1", "d2"]).unwrap(), "SYNC CATHEDRAL 1/1 READY");
    }
}
