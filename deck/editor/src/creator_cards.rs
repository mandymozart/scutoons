//! The creator's workspaces in the deck's switcher (#859, server/schema/tables/deck-link.json): read from the member's
//! newest creator row (GET /api/me/session?device=…: `other`, the creator's own state) – its open tabs, grouped by the
//! workspace each was last shown in, the front one's item first. Read once a switcher opens, in the background; signed
//! out, offline or with no creator row there are none. A card opens the creator in the browser on its tab's page.
use crate::switcher::CreatorCard;
use serde::Deserialize;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// where the creator lives (its tabs' pages are paths under it)
pub const CREATOR: &str = "https://creator.scutoons.com";

#[derive(Debug, Deserialize)]
struct Tab {
    address: String,
    #[serde(default)]
    href: String,
    #[serde(default)]
    workspace: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Tabs {
    #[serde(default)]
    tabs: Vec<Tab>,
    #[serde(default)]
    front: Option<String>,
}

#[derive(Debug, Deserialize)]
struct State {
    tabs: Tabs,
}

#[derive(Debug, Deserialize)]
struct Row {
    state: State,
}

#[derive(Debug, Deserialize)]
struct Answer {
    #[serde(default)]
    other: Option<Row>,
}

/// a creator's card and the page it opens
#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    pub name: String,
    pub item: String,
    pub href: String,
}

impl Card {
    pub fn shown(&self) -> CreatorCard {
        CreatorCard { name: self.name.clone(), item: Some(self.item.clone()) }
    }
}

/// a workspace's word as the creator's tabs name it: `sound` → Sound
fn word(id: &str) -> String {
    let mut c = id.chars();
    c.next().map_or_else(String::new, |f| f.to_uppercase().chain(c).collect())
}

/// the cards of a creator row's state (the answer of GET /api/me/session): one per workspace its tabs were last shown
/// in, in the tabs' order, the front tab's item on its card
pub fn parse(text: &str) -> Vec<Card> {
    let Ok(a) = serde_json::from_str::<Answer>(text) else { return Vec::new() };
    let Some(row) = a.other else { return Vec::new() };
    let tabs = row.state.tabs;
    let mut out: Vec<Card> = Vec::new();
    for t in &tabs.tabs {
        let ws = t.workspace.clone().filter(|w| !w.is_empty()).unwrap_or_else(|| t.address.split(':').next().unwrap_or_default().to_string());
        let name = word(&ws);
        let front = tabs.front.as_deref() == Some(t.address.as_str());
        match out.iter_mut().find(|c| c.name == name) {
            Some(c) if front => (c.item, c.href) = (t.address.clone(), t.href.clone()),
            Some(_) => {}
            None => out.push(Card { name, item: t.address.clone(), href: t.href.clone() }),
        }
    }
    out
}

/// this machine's device id for the session row: `deck-` and its name (8 to 64 letters, digits, - and _)
pub fn device() -> String {
    let name = std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).unwrap_or_default();
    let clean: String = name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').take(48).collect();
    format!("deck-editor-{}", if clean.is_empty() { "here".into() } else { clean.to_lowercase() })
}

/// the creator's cards, asked in the background
#[derive(Default)]
pub struct Loader {
    pub cards: Vec<Card>,
    rx: Option<Receiver<Vec<Card>>>,
}

impl Loader {
    /// a check run's cards from a file (the answer's JSON)
    pub fn from_file(path: &std::path::Path) -> Loader {
        Loader { cards: std::fs::read_to_string(path).map(|t| parse(&t)).unwrap_or_default(), rx: None }
    }

    /// ask again (a switcher opened); signed out, none
    pub fn ask(&mut self, api: &str, token: Option<String>) {
        let Some(token) = token else { return };
        if self.rx.is_some() {
            return;
        }
        let (tx, rx) = channel();
        let url = format!("{api}/me/session?device={}", device());
        std::thread::spawn(move || {
            let r = ureq::AgentBuilder::new().timeout(Duration::from_secs(4)).build().get(&url).set("Authorization", &format!("Bearer {token}")).call();
            let cards = r.ok().and_then(|r| r.into_string().ok()).map(|t| parse(&t)).unwrap_or_default();
            let _ = tx.send(cards);
        });
        self.rx = Some(rx);
    }

    /// the answer, once it came
    pub fn poll(&mut self) {
        if let Some(cards) = self.rx.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.cards = cards;
            self.rx = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_creator_row_gives_a_card_per_workspace_the_front_item_on_its_card() {
        let text = r#"{"available":true,"mine":null,"other":{"device":"abc12345","name":"Firefox · Windows","updated_at":"2026-11-24","state":{"v":1,
            "tabs":{"front":"character:giantpopstar","tabs":[
              {"address":"character:crow-fox","href":"/character/crow-fox","label":"crow-fox","workspace":"character"},
              {"address":"character:giantpopstar","href":"/character/giantpopstar","label":"giantpopstar","workspace":"character"},
              {"address":"toon:sandbox","href":"/toon/sandbox","label":"sandbox","workspace":"toon"},
              {"address":"module:arp","href":"/module/arp","label":"arp"}]}}}}"#;
        let cards = parse(text);
        assert_eq!(cards.iter().map(|c| (c.name.as_str(), c.item.as_str())).collect::<Vec<_>>(), [("Character", "character:giantpopstar"), ("Toon", "toon:sandbox"), ("Module", "module:arp")]);
        assert_eq!(cards[0].href, "/character/giantpopstar");
    }

    #[test]
    fn no_row_or_no_answer_no_cards() {
        assert!(parse(r#"{"available":true,"mine":null,"other":null}"#).is_empty());
        assert!(parse("not json").is_empty());
        let d = device();
        assert!(d.len() >= 8 && d.len() <= 64 && d.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'), "{d}");
    }
}
