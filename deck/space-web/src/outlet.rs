//! A Live outlet in the browser (#465): a scene's `outlet` node shows the cells the conductor sends to this outlet, as
//! on the native deck (deck-native's live/). The decisions are deck-outlet's – where a message goes (route), when to
//! compose again (recompose), the preload's schedule (preload), the plane (compose), the HUD's words (status) – and the
//! relay client is deck-space's. This is only the browser's side of the three native threads, run from the frame loop:
//! the page's socket drives the relay client (client.ts `RelaySocket`), and the page fetches what the shelf asks for
//! (a chapter, a slice, a preloaded medium) and hands it back decoded.
//! Show media (#470, deck-outlet's `show`): a preload url `live/<toon>/<file>` is asked as `s:<id>` - the API's signed
//! link, which the page asks for with the deck's token (link.rs) - and then fetched as `m:<id>` from that link, without
//! the token. A deck that is not paired answers the conductor why (the code to pair with).
//!
//! Not yet in the browser: a preloaded .glb as the set (a set by name loads from the library), video (its still stands
//! in, as on the native deck).
use deck_outlet::compose::{Composer, Shelf};
use deck_outlet::frame::{Frame, SpaceTarget};
use deck_outlet::plan::MediaRef;
use deck_outlet::preload::Preload;
use deck_outlet::recompose::Recompose;
use deck_outlet::route::{route, Input, Routed};
use deck_outlet::show::{show_path, signed_link, signed_link_request};
use deck_outlet::status::Status;
use deck_outlet::text::Dialog;
use deck_space::chapter::Chapter;
use deck_space::relay::{Client, Options, Out};
use image::RgbaImage;
use protocol::{MediaItem, MediaReady, Message, Role, SpotMode};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{SyncSender, TrySendError};
use std::time::Duration;

/// what the slices are fetched at (as the native outlet)
const QUALITY: &str = "high";
/// the hosted relay's show (deck-native live/config.rs)
pub const HOSTED_RELAY: &str = "wss://live.scutoons.com/live/CATH";

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
    /// the API base: show media's signed links (#470)
    api: String,
    /// why the deck has no token ("" = it has one - the page says, from DeckLink)
    pub unpaired: String,
}

impl Outlet {
    /// join `relay` (ws:// or wss://…/live/<code>) as output of `outlet`, its text in `locale`, show media's links from `api`;
    /// the planes go to `frames`
    pub fn new(relay: &str, outlet: &str, locale: &str, api: &str, size: (u32, u32), frames: SyncSender<Frame>) -> Result<Outlet, String> {
        let relay = if relay.trim().is_empty() { HOSTED_RELAY } else { relay.trim() };
        if outlet.is_empty() || !outlet.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err(format!("outlet '{outlet}': letters, digits, - and _ only"));
        }
        let mut o = Options::new(relay, Role::Output, Some(outlet));
        o.resume = true;
        o.messages = true;
        o.nick = Some("browser".into());
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
            api: api.trim_end_matches('/').to_owned(),
            unpaired: "the deck is not paired yet".into(),
        })
    }

    /// what the relay client asks of the page's socket; the messages are routed here (`now`: the page's clock, s)
    pub fn take(&mut self, now: Duration) -> Vec<Out> {
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
                Routed::Joined(_) => {}
                Routed::Compose(i) => self.inputs.push(i),
                Routed::Preload(p) => self.preload.job(p, now),
                // a cell's piece (#91) loads its scene on the native deck (live/piece.rs); the browser has no scene switch
                // yet and plays the composed cell only
                Routed::Piece(_) => {}
                // a flash (#91): the native deck answers it (live/flash.rs: the show kept, its pieces checked, `flash.report`);
                // the browser deck keeps no show yet and does not answer – the conductor's list has it without a report
                Routed::Flash(_) => {}
                Routed::Refused { code, message } => self.status.problems_relay = Some(format!("RELAY {code}: {message}")),
            }
        }
    }

    /// what the page should fetch: key, path or URL, …
    pub fn asks(&mut self) -> Vec<(String, String)> {
        std::mem::take(&mut self.composer.shelf.asks)
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

    pub fn shelf(&mut self) -> &mut WebShelf {
        &mut self.composer.shelf
    }

    /// One frame at `now`: the preload's turn, the composer's turn (one plane at most), the plane to the stage.
    pub fn frame(&mut self, now: Duration, size: (u32, u32)) {
        self.preload_turn(now);
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
        let lines = self.status.lines(&self.name);
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
