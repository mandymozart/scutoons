//! A Live outlet in the browser (#465): a scene's `outlet` node shows the cells the conductor sends to this outlet, as
//! on the native deck (deck-native's live/). The decisions are deck-outlet's – where a message goes (route), when to
//! compose again (recompose), the preload's schedule (preload), the plane (compose), the HUD's words (status) – and the
//! relay client is deck-space's. This is only the browser's side of the three native threads, run from the frame loop:
//! the page's socket drives the relay client (client.ts `RelaySocket`), and the page fetches what the shelf asks for
//! (a chapter, a slice, a preloaded medium) and hands it back decoded.
//! Show media (#470, deck-outlet's `show`): a preload url `live/<toon>/<file>` is asked as `s:<id>` - the API's signed
//! link, which the page asks for with the deck's token (link.rs) or, in a signed-in viewer, with its user's sign-in
//! (#484: that deck is its user and is not paired) - and then fetched as `m:<id>` from that link, without the token. A
//! deck that is not paired and has no user answers the conductor why (the code to pair with).
//! Its proof (#478, deck-outlet's `proof`): asked as `r:` – the page fetches it with its reader's sign-in – and put in the
//! relay client's hellos; a reader not signed in gets none and the deck joins as a guest.
//! Pieces and a flash (#479, #475): a piece the deck does not carry is asked as `cr:<piece>` (deck-space `fetched.rs`:
//! the API's scene-content); a flash is answered by deck-show's `flashed.rs` – the show at its version is asked as
//! `sf:<kept name>` with the deck's token, its pieces through the same store, the `flash.report` goes to the conductor.
//!
//! Not yet in the browser: a preloaded .glb as the set (a set by name loads from the library), video (its still stands
//! in, as on the native deck).
use deck_outlet::compose::{Composer, Shelf};
use deck_outlet::frame::{Frame, SpaceTarget};
use deck_outlet::plan::MediaRef;
use deck_outlet::preload::Preload;
use deck_outlet::proof::{self, Renew};
use deck_outlet::recompose::Recompose;
use deck_outlet::route::{route, Input, Routed};
use deck_outlet::show::{show_path, signed_link, signed_link_request};
use deck_outlet::status::Status;
use deck_outlet::text::Dialog;
use deck_show::dialog::Shown;
use deck_show::flashed::Flashed;
use deck_space::chapter::Chapter;
use deck_space::fetched::Fetched;
use deck_space::piece::Found;
use deck_space::rig::address::Place;
use deck_space::relay::{Client, Options, Out};
use image::RgbaImage;
use protocol::{Cell, MediaItem, MediaReady, Message, Role, SpotMode};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{SyncSender, TrySendError};
use std::time::Duration;

/// what the slices are fetched at (as the native outlet)
const QUALITY: &str = "high";
/// the hosted relay's show (deck-native live/config.rs)
pub const HOSTED_RELAY: &str = "wss://live.scutoons.com/live/CATH";

/// What this deck has of a piece without asking: a system scene by its name (`scene:outlet`; #427: a sandbox never plays in a show)
fn carried(place: &Place) -> Option<Found> {
    deck_space::scene::builtin(&place.id).filter(|_| place.kind == "scene").map(|text| Found::Scene(text.to_string()))
}

/// find, for a cell: what the deck carries, else what it fetched – asked for when a place serves it (`Outlet::asks`)
pub fn find(fetched: &mut Fetched, place: &Place) -> Option<Found> {
    carried(place).or_else(|| fetched.find(place))
}

/// What the shelf has of a thing it was asked for.
enum Got<T> {
    Asked,
    Have(T),
    Failed(String),
}

fn waiting(what: &str) -> String {
    // "not preloaded" makes the recompose machine compose again when it arrives (deck-outlet recompose.rs)
    format!("{what} is not preloaded (yet)")
}

/// What the composer reads, as far as the page has fetched it; what is missing is asked for (`asks`) and the layer waits.
#[derive(Default)]
pub struct WebShelf {
    chapters: HashMap<String, Got<Chapter>>,
    pictures: HashMap<String, Got<RgbaImage>>,
    media: HashMap<String, Got<RgbaImage>>,
    /// the preload on: its items name the set spots' stills
    items: Vec<MediaItem>,
    /// what to fetch: the key (c: a chapter, p: a slice, m: a medium) and its path or URL
    asks: Vec<(String, String)>,
}

impl WebShelf {
    fn chapter(&mut self, toon: &str, n: u32) -> Result<&Chapter, String> {
        let key = format!("c:{toon}/{n}");
        match self.chapters.get(&key) {
            Some(Got::Have(_)) => {}
            Some(Got::Failed(e)) => return Err(format!("{toon} {n}: {e}")),
            Some(Got::Asked) => return Err(waiting(&format!("{toon} {n}"))),
            None => {
                self.chapters.insert(key.clone(), Got::Asked);
                self.asks.push((key, format!("/library/toons/{toon}/chapters/{n}")));
                return Err(waiting(&format!("{toon} {n}")));
            }
        }
        match self.chapters.get(&key) {
            Some(Got::Have(c)) => Ok(c),
            _ => unreachable!(),
        }
    }

    fn item_image(&self, id: &str) -> Result<RgbaImage, String> {
        match self.media.get(id) {
            Some(Got::Have(img)) => Ok(img.clone()),
            Some(Got::Failed(e)) => Err(format!("'{id}': {e}")),
            _ => Err(waiting(&format!("'{id}'"))),
        }
    }

    /// the page fetched `key`: a chapter's text
    pub fn give_text(&mut self, key: &str, text: &str) {
        let got = serde_json::from_str::<Chapter>(text).map_or_else(|e| Got::Failed(format!("not a chapter ({e})")), Got::Have);
        self.chapters.insert(key.to_owned(), got);
    }

    /// the page fetched and decoded `key`: a slice or a medium
    pub fn give_image(&mut self, key: &str, img: RgbaImage) {
        self.images(key).insert(key[2..].to_owned(), Got::Have(img));
    }

    pub fn failed(&mut self, key: &str, why: String) {
        if key.starts_with("c:") {
            self.chapters.insert(key.to_owned(), Got::Failed(why));
        } else {
            self.images(key).insert(key[2..].to_owned(), Got::Failed(why));
        }
    }

    fn images(&mut self, key: &str) -> &mut HashMap<String, Got<RgbaImage>> {
        if key.starts_with("p:") {
            &mut self.pictures
        } else {
            &mut self.media
        }
    }
}

impl Shelf for WebShelf {
    fn slices(&mut self, toon: &str, n: u32) -> Result<usize, String> {
        Ok(self.chapter(toon, n)?.slices().count())
    }

    fn picture(&mut self, toon: &str, n: u32, index: usize) -> Result<RgbaImage, String> {
        let c = self.chapter(toon, n)?;
        let count = c.slices().count();
        let s = c.slices().nth(index).ok_or_else(|| format!("{toon} {n} has {count} slices, not {}", index + 1))?;
        let path = format!("{toon}/{}", s.image_path(QUALITY).ok_or("a slice without an image")?);
        match self.pictures.get(&path) {
            Some(Got::Have(img)) => Ok(img.clone()),
            Some(Got::Failed(e)) => Err(format!("{path}: {e}")),
            Some(Got::Asked) => Err(waiting(&path)),
            None => {
                self.pictures.insert(path.clone(), Got::Asked);
                self.asks.push((format!("p:{path}"), path.clone()));
                Err(waiting(&path))
            }
        }
    }

    fn dialog(&mut self, toon: &str, n: u32, index: usize) -> Result<&[Dialog], String> {
        Ok(self.chapter(toon, n)?.slices().nth(index).map_or(&[][..], |s| &s.dialog[..]))
    }

    fn image(&mut self, media: &MediaRef) -> Result<RgbaImage, String> {
        match media {
            MediaRef::Item(id) => self.item_image(id),
            MediaRef::Set { set, spot, mode } => {
                let item = self.items.iter().find(|i| i.set.as_deref() == Some(set) && i.spot == *spot && i.mode == Some(*mode) && i.url.is_some());
                match item {
                    Some(i) => self.item_image(&i.id.clone()),
                    None => Err(format!("set {set} {} ({}) is not on this deck - no set data yet (#193)", spot.as_deref().unwrap_or("default"), mode.as_str())),
                }
            }
        }
    }

    fn set(&mut self, media: &MediaRef) -> Result<PathBuf, String> {
        match media {
            // the space loads a set by its name from the library (stage-assets.ts `fetchSet`)
            MediaRef::Set { set, .. } => Ok(PathBuf::from(set)),
            MediaRef::Item(id) => Err(format!("'{id}': a preloaded 3D file is not played in the browser yet")),
        }
    }
}

/// #478: what the Live page chose on this device – the page keeps it as text (`SceneView::live_kept`), the core reads it
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Kept {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

/// The outlet in the page: the relay client, the composer and the two machines, on the page's clock.
pub struct Outlet {
    pub name: String,
    pub relay: Client,
    composer: Composer<WebShelf>,
    recompose: Recompose,
    preload: Preload,
    inputs: Vec<Input>,
    frames: SyncSender<Frame>,
    pending: Option<Frame>,
    /// the preload's media on their way (by id) and what they answered
    flying: HashSet<String>,
    ready: Vec<String>,
    why: HashMap<String, String>,
    loading: bool,
    pub status: Status,
    said: Vec<String>,
    /// the space node's new target (the scene view applies it)
    pub space: Option<SpaceTarget>,
    /// #473: the cells for the deck's piece side (deck-space piece.rs `Side`; the scene view runs it)
    pub pieces: Vec<Cell>,
    /// the API base: show media's signed links (#470)
    api: String,
    /// why the deck has no token ("" = it has one - the page says, from DeckLink)
    pub unpaired: String,
    /// #479: the pieces the deck does not carry, fetched (the piece side and a flash's check read it)
    pub fetched: Fetched,
    /// #475: the flash side
    flash: Flashed,
    /// #478: when it asks for its proof (`r:`)
    renew: Renew,
    /// #478: the name chosen on this device's Live page (what `Kept` keeps) – not the preset, so a reader who signs in later
    /// is named after them
    pub named: Option<String>,
}

impl Outlet {
    /// join `relay` (ws:// or wss://…/live/<code>) as output of `outlet`, its text in `locale`, show media's links from `api`;
    /// the planes go to `frames`. #478: `kept` – what the Live page chose on this device (`Kept`, the page keeps the text):
    /// its name; `reader` – the signed-in reader's name ("" = nobody): the name when none is kept. The outlet the address
    /// names wins over the role kept on the device (owner 2026-11-11: a QR code can lead into a specific outlet, an
    /// offline entry point); the Live page still moves the deck to another role during the session.
    pub fn new(relay: &str, outlet: &str, kept: &str, reader: &str, locale: &str, api: &str, size: (u32, u32), frames: SyncSender<Frame>) -> Result<Outlet, String> {
        let relay = if relay.trim().is_empty() { HOSTED_RELAY } else { relay.trim() };
        if outlet.is_empty() || !outlet.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err(format!("outlet '{outlet}': letters, digits, - and _ only"));
        }
        let kept: Kept = serde_json::from_str(kept).unwrap_or_default();
        let named = kept.name.clone().filter(|n| !n.trim().is_empty());
        let mut o = Options::new(relay, Role::Output, Some(outlet).filter(|g| !g.is_empty()));
        o.resume = true;
        o.messages = true;
        o.nick = Some(deck_ui::live::preset_name(kept.name.as_deref(), Some(reader), "browser"));
        o.device = Some("browser".into());
        // #478: the first hello waits for the proof (`take` asks for it at once, `proof` hands the answer on)
        o.wait_proof = true;
        let relay = Client::new(o)?;
        let status = Status { link: format!("JOINING {}", relay.url()), ..Default::default() };
        Ok(Outlet {
            name: outlet.to_owned(),
            relay,
            composer: Composer::new(WebShelf::default(), size),
            recompose: Recompose::new(locale, size),
            preload: Preload::default(),
            inputs: Vec::new(),
            frames,
            pending: None,
            flying: HashSet::new(),
            ready: Vec::new(),
            why: HashMap::new(),
            loading: false,
            status,
            said: Vec::new(),
            space: None,
            pieces: Vec::new(),
            api: api.trim_end_matches('/').to_owned(),
            unpaired: "the deck is not paired yet".into(),
            fetched: Fetched::default(),
            flash: {
                let mut flash = Flashed::new(outlet, "browser");
                flash.set_locale(locale);
                flash
            },
            renew: Renew::default(),
            named,
        })
    }

    /// what the relay client asks of the page's socket; the messages are routed here (`now`: the page's clock, s)
    pub fn take(&mut self, now: Duration) -> Vec<Out> {
        if self.renew.due(now.as_secs_f64()) {
            let ask = ("r:".to_string(), proof::request(&self.api, self.relay.code()));
            self.composer.shelf.asks.push(ask);
        }
        let mut out = Vec::new();
        for o in self.relay.take() {
            match o {
                Out::Message(e) => self.hear(e.message, now),
                Out::Status(s) => {
                    self.status.link = self.relay.line();
                    out.push(Out::Status(s));
                }
                o => out.push(o),
            }
        }
        out
    }

    fn hear(&mut self, m: Message, now: Duration) {
        for r in route(m, &self.name) {
            match r {
                Routed::Joined(id) => self.flash.joined(&id),
                Routed::Compose(i) => self.inputs.push(i),
                Routed::Preload(p) => self.preload.job(p, now),
                Routed::Piece(c) => self.pieces.push(c),
                Routed::Flash(f) => self.flash.flash(&f, &self.api, &self.unpaired, &mut self.fetched),
                Routed::Refused { code, message } => self.status.problems_relay = Some(format!("RELAY {code}: {message}")),
                // #478: a role taken – its cells from now (the black of no role is routed to the piece side too)
                Routed::Moved(role) => self.name = role.unwrap_or_default(),
            }
        }
    }

    /// The deck loaded a scene in place (#473): the planes go to its stage, the cell on is composed again for it.
    pub fn rewire(&mut self, frames: SyncSender<Frame>) {
        self.frames = frames;
        self.pending = None;
        self.recompose.again();
    }

    /// what the page should fetch: key, path or URL, …
    pub fn asks(&mut self) -> Vec<(String, String)> {
        let mut asks = std::mem::take(&mut self.composer.shelf.asks);
        asks.extend(self.fetched.asks().into_iter().map(|(place, path)| (format!("cr:{place}"), path)));
        asks.extend(self.flash.ask().map(|(kept, url)| (format!("sf:{kept}"), url)));
        asks
    }

    /// The page's answer to an ask that is no medium – a fetched piece (`cr:`), a flashed show (`sf:`): the API's status
    /// and body, or why the fetch failed. false = the key is neither.
    pub fn answered(&mut self, key: &str, answer: Result<(u16, &str), String>) -> bool {
        if let Some(piece) = key.strip_prefix("cr:") {
            if let Ok(place) = Place::parse(piece) {
                self.fetched.give(&place, answer.map(|(_, body)| body));
            }
            true
        } else if let Some(kept) = key.strip_prefix("sf:") {
            self.flash.document(kept, answer);
            true
        } else {
            false
        }
    }

    /// #481: what the deck says to the person at it about a flash – a dialog, a toast, or nothing (the scene view shows it)
    pub fn shown(&self) -> Option<Shown> {
        self.flash.shown()
    }

    /// #481: the flash side and the pieces it asks for, for the taps and keys of the deck's UI (deck-show `face.rs`)
    pub fn flash_parts(&mut self) -> (&mut Flashed, &mut Fetched) {
        (&mut self.flash, &mut self.fetched)
    }

    /// the flash side's turn: its reports go to the conductor and the HUD's FLASH lines
    fn flash_turn(&mut self, now: Duration) {
        self.flash.turn(now.as_secs_f32(), &mut self.fetched, carried);
        for report in self.flash.reports() {
            self.status.flash = Some(deck_show::ready::line(&report));
            self.status.flash_lacks = report.lacks.clone().unwrap_or_default();
            self.status.flash_notice = self.flash.notice();
            self.relay.send_message(Message::FlashReport(report));
        }
    }

    /// the page fetched a medium of the preload (decoded when it is a picture)
    pub fn medium(&mut self, id: &str, why: Option<String>) {
        if self.flying.remove(id) {
            match why {
                None => self.ready.push(id.to_owned()),
                Some(w) => {
                    self.why.insert(id.to_owned(), w);
                }
            }
        }
        // a chapter or a slice arrived: a cell waiting for it is composed again
        self.inputs.push(Input::Media);
    }

    /// The API's answer to a show medium's `s:<id>` -> its signed link is fetched (`m:<id>`), or the medium is missing
    pub fn signed(&mut self, id: &str, status: u16, body: &str) {
        let body = serde_json::from_str(body).unwrap_or(serde_json::Value::Null);
        match signed_link(status, &body) {
            Ok(url) => self.composer.shelf.asks.push((format!("m:{id}"), url)),
            Err(why) => {
                self.composer.shelf.failed(&format!("m:{id}"), why.clone());
                self.medium(id, Some(why));
            }
        }
    }

    /// #478: the API's answer to the `r:` ask (`status` 0: no answer) at `now` – the proof for the relay client's hellos
    pub fn proof(&mut self, status: u16, body: &str, now: Duration) {
        let body = serde_json::from_str(body).unwrap_or(serde_json::Value::Null);
        let answer = proof::read(status, &body);
        // #481: who is at the deck (whether they may reject a flash) is what the proof states
        if let Some(roles) = proof::account(&answer) {
            self.flash.set_account(&roles);
        }
        self.relay.proof_answered(self.renew.answered(now.as_secs_f64(), answer));
    }

    /// #481: who is signed in changed (the page says so): the proof is asked again at once – a reader who signed out is a
    /// guest from the answer on, one who signed in is who they are
    pub fn proof_again(&mut self) {
        self.renew.again();
    }

    pub fn shelf(&mut self) -> &mut WebShelf {
        &mut self.composer.shelf
    }

    /// One frame at `now`: the preload's turn, the composer's turn (one plane at most), the plane to the stage.
    pub fn frame(&mut self, now: Duration, size: (u32, u32)) {
        self.preload_turn(now);
        self.flash_turn(now);
        if let Some(f) = self.pending.take() {
            if let Err(TrySendError::Full(f)) = self.frames.try_send(f) {
                self.pending = Some(f);
            }
        }
        let mut todo: Option<bool> = None;
        let inputs: Vec<Input> = std::mem::take(&mut self.inputs);
        for i in inputs.into_iter().map(Some).chain([None]) {
            if let Some(c) = self.recompose.step(i, size, now, |m| m == "space") {
                todo = Some(todo.unwrap_or(false) || c.fresh);
            }
        }
        if let Some(fresh) = todo {
            self.composer.size = self.recompose.size();
            let t = js_sys::Date::now();
            let (plan, slice, scroll) = (self.recompose.plan(), self.recompose.slice(), self.recompose.scroll());
            let out = if fresh { self.composer.compose(plan, slice, scroll) } else { self.composer.scroll_step(plan, slice, scroll) };
            let done = self.recompose.composed(now, (js_sys::Date::now() - t) as f32, out.frame.backdrop, &out.problems, &out.space);
            if done.space.is_some() {
                self.space = done.space;
            }
            self.status.cell = done.cell;
            self.status.problems = out.problems;
            self.pending = Some(out.frame);
        }
        let s = self.relay.live();
        let lines = self.status.lines(&if s.joined { deck_ui::live::who(&s) } else { self.name.clone() });
        if lines != self.said {
            web_sys::console::log_1(&lines.join("\n").into());
            self.said = lines;
        }
    }

    /// the preload: a job that is due is asked of the page; when every medium answered, `media.ready` goes out
    fn preload_turn(&mut self, now: Duration) {
        if self.loading {
            if !self.flying.is_empty() {
                return;
            }
            self.loading = false;
            let items = &self.composer.shelf.items;
            let missing: Vec<String> = items.iter().map(|i| i.id.clone()).filter(|id| !self.ready.contains(id)).collect();
            let why: HashMap<String, String> = std::mem::take(&mut self.why).into_iter().filter(|(id, _)| missing.contains(id)).collect();
            let ready = MediaReady { outlet: self.name.clone(), ready: std::mem::take(&mut self.ready), missing, why: (!why.is_empty()).then_some(why) };
            self.status.media = self.preload.done(&ready, now);
            self.relay.send_message(Message::MediaReady(ready));
            self.inputs.push(Input::Media);
            return;
        }
        let Some(job) = self.preload.due(now) else { return };
        self.status.media = self.preload.loading_line();
        self.loading = true;
        self.ready.clear();
        self.why.clear();
        for item in &job.items {
            match (&item.url, item.mode) {
                // a set's 3D: the space loads it by name from the library
                (_, Some(SpotMode::Model3d)) if item.set.is_some() => self.ready.push(item.id.clone()),
                (Some(_), _) if matches!(self.composer.shelf.media.get(&item.id), Some(Got::Have(_))) => self.ready.push(item.id.clone()),
                // a show medium without a token: the conductor hears why (the code to pair with)
                (Some(url), _) if show_path(url).is_some() && !self.unpaired.is_empty() => {
                    self.why.insert(item.id.clone(), format!("show medium: {}", self.unpaired));
                }
                (Some(url), _) => {
                    self.flying.insert(item.id.clone());
                    self.composer.shelf.media.insert(item.id.clone(), Got::Asked);
                    let ask = match show_path(url) {
                        Some(path) => (format!("s:{}", item.id), signed_link_request(&self.api, path)),
                        None => (format!("m:{}", item.id), url.clone()),
                    };
                    self.composer.shelf.asks.push(ask);
                }
                (None, _) => {
                    self.why.insert(item.id.clone(), "no URL and no set data yet (#193)".into());
                }
            }
        }
        self.composer.shelf.items = job.items;
    }
}
