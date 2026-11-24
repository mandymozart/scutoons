//! One address format for everything a cable, a value or a stick names (#432, research/MODULE-WIRING.md §3), and the
//! rule of how far an address may reach. The one parser of the core: nothing else splits a path.
//!
//! ```text
//! address := ["/"] { place "/" } leaf
//! place   := <kind> ":" <id>              deck:left · scene:nave · character:catpope · item:incense (later toon:, chapter:, set: …)
//! leaf    := <track>:<node>.<port> · <track>:@gain · <track>:@preset · <track>:@open · being:<input> · policy:<port> · <node>.<port> · @<member>
//!            · spot:<name>
//! ```
//! `spot:<name>` is a point on a model (deck-catalog spot.rs): `character:catpope/spot:hand_l`, `item:lantern/spot:flame`,
//! and relative `spot:hand_l` inside the document that holds the model ([`SpotRef`]).
//! The last segment is always the leaf, a `/` separates levels; without a leading `/` an address is relative to the document
//! it stands in – so every path written before this file is a valid address unchanged. A bare `<node>` is its `out`.
//! `character:catpope#2` (a second copy of a holder on one stage) is reserved and refused for now.
//!
//! **Reach** (owner 2026-11-08, decisions/deck.md [rigs]: wide from the start – other scenes, characters and rigs, so linked
//! decks can affect each other): `resolve` is the one function that says what an address stands for in a document –
//! `Local` (the leaf, here) or `Far` (another place: the deck's link asks the named deck for it over the relay, far.rs,
//! #442). Widening or narrowing the reach changes this function, never a document. A rig holds relative addresses only
//! (it stays portable); the absolute address sits on a world node (world.rs `Source::Far`).
//!
//! Compiled names (`voice_voice.ratio`, `mix1.gain2`, `policy.heat`) are slots flatten produces, not addresses: the
//! run-time, the knobs, `Stream` and the relay keep them, and nothing on the audio or frame path parses an address.
use std::fmt;

/// the reserved scope of the being's look (`being:<input>`)
pub const BEING: &str = "being";
/// the reserved scope of the analyser on the rig's own sum (`sound:<name>`, #455; RULES A14: a scope says where it comes from)
pub const SOUND: &str = "sound";
/// the reserved scope of the show's clock (`clock:<name>`)
pub const CLOCK: &str = "clock";
/// the reserved scope of the scene camera's view of the rig's subject (`camera:<name>`): how sharp it is under the focus
/// (`focus`, 0…1, scene_camera.rs `in_focus`) and how far from the eye (`distance`, m) – outlets, wired on like any control
pub const CAMERA: &str = "camera";
/// the reserved scope of what starts a subject from outside (`trigger:sing`, 0 shut … 1 open): the player's held button or
/// key on the picked one, a story's text line of its character (space/sing.rs) – the stage sets it every frame
pub const TRIGGER: &str = "trigger";
/// a track's reserved member that opens its sequencers' gates (`voice:@open`): a route to it reaches the `open` input of
/// every walk, seq and events node of the track; a track that takes another over takes its routes to it along
pub const OPEN: &str = "open";
/// what a rig can read from outside its tracks: scope, name, the compiled slot it reads
pub const SOURCES: [(&str, &str, &str); 13] = [
    (SOUND, "level", "fft.level"),
    (SOUND, "bass", "fft.bass"),
    (SOUND, "mid", "fft.mid"),
    (SOUND, "high", "fft.high"),
    (CLOCK, "beat", "tempo.beat"),
    (CLOCK, "bar", "tempo.bar"),
    (CLOCK, "phase", "tempo.phase"),
    (CLOCK, "bar_phase", "tempo.bar_phase"),
    (CLOCK, "pulse", "tempo.pulse"),
    (CLOCK, "bpm", "tempo.bpm"),
    (CAMERA, "focus", "camera_focus.out"),
    (CAMERA, "distance", "camera_distance.out"),
    (TRIGGER, "sing", "trigger_sing.out"),
];

/// the compiled slot of a source
pub fn source_slot(scope: &str, name: &str) -> Option<&'static str> {
    SOURCES.iter().find(|(s, n, _)| *s == scope && *n == name).map(|(_, _, slot)| *slot)
}

/// the reserved scope of a code rig's policy node (`policy:<port>`), the node's id too (policy.rs `NODE`)
pub const POLICY: &str = "policy";

/// A level above the leaf: `<kind>:<id>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Place {
    pub kind: String,
    pub id: String,
}

impl Place {
    pub fn new(kind: &str, id: &str) -> Place {
        Place { kind: kind.to_string(), id: id.to_string() }
    }

    /// `<kind>:<id>`, the place alone (a show's piece, what an event is addressed to; SHOW.md §8, kind and id)
    pub fn parse(s: &str) -> Result<Place, String> {
        let Some((kind, id)) = s.split_once(':') else { return Err(format!("'{s}' is not a place (<kind>:<id>)")) };
        word(kind, "a place's kind")?;
        word(id, "a place's id")?;
        Ok(Place::new(kind, id))
    }

    /// a rig's subject as its place: `character.catpope` / `character.catpope.pope` → `character:catpope`, `item.x` → `item:x`
    pub fn of_subject(subject: &str) -> Option<Place> {
        let mut it = subject.split('.');
        match (it.next(), it.next()) {
            (Some(kind), Some(id)) if !kind.is_empty() && !id.is_empty() => Some(Place::new(kind, id)),
            _ => None,
        }
    }
}

impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind, self.id)
    }
}

/// `*` in an address's id: every one of the kind (`character:*`, SHOW.md §8); never a source (many matches are fine only
/// for a target)
pub const EVERY: &str = "*";
/// the kind whose ids go down to persona and look (`character:catpope.mountaintiger.ghost`, SHOW.md §8); the word is the
/// address book schema's `AddressKind` (server/schema/api/address-book.schema.json, generated: the one list of the catalog's
/// kinds of place, #477)
pub const CHARACTER: &str = protocol::address_book::AddressKind::Character.as_str();

/// `address` is `reference` or a dot down from it; a shared prefix (`a.b` vs `a.bc`) is not below.
pub fn at_or_below(reference: &str, address: &str) -> bool {
    address.strip_prefix(reference).is_some_and(|rest| rest.is_empty() || rest.starts_with('.'))
}

/// What a show's address names on the ladder (SHOW.md §8, the workshop's kind and id; #91): `character:*` every one of a
/// kind, `character:yellowmouse` every persona and look of it, `character:catpope.mountaintiger.ghost` only that look.
/// Persona and look exist for characters only; dots as rig subjects spell them. Who picks up goes down: [`Target::picks_up`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub kind: String,
    /// none = [`EVERY`]
    pub id: Option<String>,
    /// persona, then look
    pub below: Vec<String>,
}

impl Target {
    pub fn parse(s: &str) -> Result<Target, String> {
        let Some((kind, rest)) = s.split_once(':') else { return Err(format!("'{s}' is not an address (<kind>:<id>)")) };
        word(kind, "an address's kind")?;
        if rest == EVERY {
            return Ok(Target { kind: kind.to_string(), id: None, below: vec![] });
        }
        let mut parts = rest.split('.');
        let id = word(parts.next().unwrap_or_default(), "an address's id")?.to_string();
        let below = parts.map(|p| word(p, "a persona or look").map(str::to_string)).collect::<Result<Vec<_>, _>>()?;
        if !below.is_empty() && kind != CHARACTER {
            return Err(format!("'{s}': persona and look exist for characters only"));
        }
        if below.len() > 2 {
            return Err(format!("'{s}': a character goes down to persona and look, no further"));
        }
        if id == EVERY {
            return Err(format!("'{s}': '*' stands alone after the kind"));
        }
        Ok(Target { kind: kind.to_string(), id: Some(id), below })
    }

    pub fn picks_up(&self, playing: &str) -> bool {
        let Ok(there) = Target::parse(playing) else { return false };
        self.kind == there.kind && (self.id.is_none() || at_or_below(&self.to_string(), playing))
    }

    /// What an address asks of who picks it up: a whole word, or `<scope>:<word>` (`close_doors`, `dance:samba`) – or a
    /// playing scene's cast word, `join:<who>` / `leave:<who>` (`CastWord`).
    pub fn action(s: &str) -> Result<(), String> {
        if let Some(c) = CastWord::parse(s) {
            return c.map(drop);
        }
        match s.split_once(':') {
            Some((scope, name)) => word(scope, "an action's scope").and(word(name, "an action")).map(drop),
            None => word(s, "an action").map(drop),
        }
    }
}

/// the reserved action a playing scene takes to let someone appear on its stage (#516, SCENE-EVENTS.md §3.1): `join:<who>`
pub const JOIN: &str = "join";
/// the reserved action a playing scene takes to let someone disappear from its stage: `leave:<who>`
pub const LEAVE: &str = "leave";

/// A playing scene's cast word (#516; owner 2026-11-13 [scenes]: "a join without an action is just appear"): sent to the
/// scene (`scene:stage > join:character:catpope`), it names who as a filler's reference – `<kind>:<id>`, a character down
/// to persona and look – so its argument is an address, not a word. What it does is the core's (scene_play.rs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CastWord {
    /// appears at the first free spot whose placeholder admits it
    Join(String),
    /// disappears; its spot is free again
    Leave(String),
}

impl CastWord {
    /// None: not a cast word (another action); else the word or why its `<who>` is no one
    pub fn parse(action: &str) -> Option<Result<CastWord, String>> {
        let (verb, who) = action.split_once(':')?;
        let make = match verb {
            JOIN => CastWord::Join,
            LEAVE => CastWord::Leave,
            _ => return None,
        };
        Some(match Target::parse(who) {
            Ok(t) if t.id.is_some() => Ok(make(who.to_string())),
            Ok(_) => Err(format!("'{action}': {verb} names one, not every {}", t_kind(who))),
            Err(e) => Err(format!("'{action}': {verb} names who as <kind>:<id> – {e}")),
        })
    }

    /// who it names (`character:catpope`)
    pub fn who(&self) -> &str {
        match self {
            CastWord::Join(w) | CastWord::Leave(w) => w,
        }
    }
}

/// the reserved action that cuts a scene's camera to one of its shots: `shot:<id>`; as a place, a storyboard's shot
/// (`ShotAddress`) – both name a shot: the camera's in a scene, the storyboard's in a chapter
pub const SHOT: &str = "shot";

/// A storyboard's shot as a place: `shot:<toon>:<chapter>:<shot-id>` (`shot:catpope:3:sb-25-1`; the grammar is the
/// storyboard's, `@scutoons/content/storyboard` `shotAddress`). The deck opens it on its stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShotAddress {
    pub toon: String,
    pub chapter: u32,
    pub shot: String,
}

impl ShotAddress {
    pub fn parse(s: &str) -> Result<ShotAddress, String> {
        let bad = || format!("'{s}' is not a shot (shot:<toon>:<chapter>:sb-<id>)");
        let mut parts = s.splitn(4, ':');
        let (Some(SHOT), Some(toon), Some(chapter), Some(shot)) = (parts.next(), parts.next(), parts.next(), parts.next()) else { return Err(bad()) };
        let numbered = !chapter.is_empty() && chapter.bytes().all(|b| b.is_ascii_digit()) && (chapter == "0" || !chapter.starts_with('0'));
        if !numbered || !shot.starts_with("sb-") || shot.len() == 3 || shot.contains(char::is_whitespace) {
            return Err(bad());
        }
        word(toon, "a shot's toon")?;
        Ok(ShotAddress { toon: toon.to_string(), chapter: chapter.parse().map_err(|_| bad())?, shot: shot.to_string() })
    }
}
/// the reserved action that plays one of a scene camera's moves (and its chain): `move:<id>`
pub const MOVE: &str = "move";

/// A playing scene's camera word (scene_camera.rs): `scene:nave > shot:wide`, `scene:nave > move:intro`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CameraWord {
    Shot(String),
    Move(String),
}

impl CameraWord {
    /// None: not a camera word; else the word or why its id is no id
    pub fn parse(action: &str) -> Option<Result<CameraWord, String>> {
        let (verb, id) = action.split_once(':')?;
        let make = match verb {
            SHOT => CameraWord::Shot,
            MOVE => CameraWord::Move,
            _ => return None,
        };
        Some(word(id, &format!("the {verb}'s id")).map(|id| make(id.to_string())))
    }
}

/// A show row's phone camera word (LIVE-CAMERAS.md §5a): `scene:duet > camera` makes a phone the scene's primary camera,
/// `scene:duet > camera:close` that shot's. `camera` as an action is the conductor's, never a deck's: it switches the phone,
/// not the picture (the relay forces it, deck-show cameras.rs).
pub struct PhoneCamera;

impl PhoneCamera {
    /// None: not a phone camera word; else the camera's id (live_cameras.rs `camera_id`) or why it is none
    pub fn parse(address: &str, action: &str) -> Option<Result<String, String>> {
        let shot = match action.split_once(':') {
            None if action == CAMERA => None,
            Some((CAMERA, shot)) => Some(shot),
            _ => return None,
        };
        let scene = Target::parse(address).ok().filter(|t| t.kind == "scene" && t.id.is_some());
        Some(match (scene, shot.map(|s| word(s, "the shot's id"))) {
            (None, _) => Err(format!("'{address} > {action}': a phone takes a scene's camera, scene:<id> > camera")),
            (_, Some(Err(e))) => Err(e),
            (Some(_), shot) => Ok(crate::live_cameras::camera_id(address, shot.and_then(Result::ok))),
        })
    }
}

/// the reserved scope of a named point on a model: `spot:<name>` – a spot of the model the address lands in (a leaf,
/// [`Leaf::Spot`]; a place says where a thing is, a spot is a point on that thing) and, in a camera's aim, a spot of the
/// scene's set (glb.rs `spots`). No track is called so.
pub const SPOT: &str = "spot";

/// A spot on a model as a light, an emitter, a sound source, a target or a cable's end names it: the spot `name` on
/// the model of `holder` (`character:catpope/spot:hand_l`, `item:lantern/spot:flame`), or of the document the address
/// stands in (`spot:hand_l`). Places before the holder (`/deck:left/scene:nave/…`) stay in `address`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpotRef {
    pub address: Address,
    pub name: String,
}

impl SpotRef {
    pub fn parse(s: &str) -> Result<SpotRef, String> {
        let address = Address::parse(s)?;
        let Leaf::Spot(name) = &address.leaf else { return Err(format!("'{s}' names no spot ({SPOT}:<name>, character:<id>/{SPOT}:<name>, item:<id>/{SPOT}:<name>)")) };
        let item = protocol::address_book::AddressKind::Item.as_str();
        if let Some(holder) = address.places.last().filter(|p| p.kind != CHARACTER && p.kind != item) {
            return Err(format!("'{s}': a spot is on a character's or an item's model, not on a {}", holder.kind));
        }
        Ok(SpotRef { name: name.clone(), address })
    }

    /// whose model: none = the document's own
    pub fn holder(&self) -> Option<&Place> {
        self.address.places.last()
    }
}

/// What a scene camera's shot aims at, or a follow keeps in the shot (scene_camera.rs): a holder of the scene by its id
/// (`duet_1`), who fills one by its place (`character:catpope`, `item:incense_burner`), or a spot of the set (`spot:altar`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Aim {
    Holder(String),
    Who(String),
    Spot(String),
}

impl Aim {
    pub fn parse(s: &str) -> Result<Aim, String> {
        let Some((kind, id)) = s.split_once(':') else { return word(s, "a holder's id").map(|h| Aim::Holder(h.to_string())) };
        match kind {
            SPOT => word(id, "a spot's name").map(|n| Aim::Spot(n.to_string())),
            k if k == CHARACTER || k == protocol::address_book::AddressKind::Item.as_str() => match Target::parse(s) {
                Ok(t) if t.id.is_some() && t.below.is_empty() => Ok(Aim::Who(s.to_string())),
                Ok(_) => Err(format!("'{s}': a camera aims at one {k}, by its id")),
                Err(e) => Err(e),
            },
            _ => Err(format!("'{s}': a camera aims at a holder, character:<id>, item:<id> or {SPOT}:<name>")),
        }
    }
}

/// the kind of `<kind>:…`
fn t_kind(s: &str) -> &str {
    s.split_once(':').map_or(s, |(k, _)| k)
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind, self.id.as_deref().unwrap_or(EVERY))?;
        for b in &self.below {
            write!(f, ".{b}")?;
        }
        Ok(())
    }
}

/// The last segment: what is named inside the document the address lands in.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Leaf {
    /// `<track>:<node>.<port>` – a port of a node inside a track; `track` none: `<node>.<port>`, the document's own node
    /// (a scene's `fft.level`, a compiled slot). A bare node is its `out`.
    Port { track: Option<String>, node: String, port: String },
    /// `<track>:@<member>` – a track's reserved member (`gain`: its strip, `preset`: what it plays)
    Member { track: String, member: String },
    /// `being:<input>` – an input of the being's look (hue, glow, light, pulse, bounce)
    Being(String),
    /// `sound:<name>`, `clock:<name>`, `camera:<name>`, `trigger:<name>` – what a rig reads from outside its tracks
    /// (`SOURCES`), a source only
    Source(String, String),
    /// `policy:<port>` – a port or parameter of a code rig's policy node
    Policy(String),
    /// `@<member>` – a holder's own state, read only (`character:catpope/@energy`)
    State(String),
    /// `spot:<name>` – a named point on the model (`character:catpope/spot:hand_l`), a position, never a slot
    Spot(String),
}

impl Leaf {
    /// `<track>:<node>.<port>` → the compiled slot `<track>_<node>.<port>`; `policy:<port>` → `policy.<port>`; a port of
    /// the document's own node as it is. None for the rest: they are no slot of the played scene.
    pub fn slot(&self) -> Option<String> {
        match self {
            Leaf::Port { track: Some(t), node, port } => Some(format!("{t}_{node}.{port}")),
            Leaf::Port { track: None, node, port } => Some(format!("{node}.{port}")),
            Leaf::Policy(port) => Some(format!("{POLICY}.{port}")),
            Leaf::Source(scope, name) => source_slot(scope, name).map(String::from),
            Leaf::Member { .. } | Leaf::Being(_) | Leaf::State(_) | Leaf::Spot(_) => None,
        }
    }
}

impl fmt::Display for Leaf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Leaf::Port { track: Some(t), node, port } => write!(f, "{t}:{node}.{port}"),
            Leaf::Port { track: None, node, port } => write!(f, "{node}.{port}"),
            Leaf::Member { track, member } => write!(f, "{track}:@{member}"),
            Leaf::Being(input) => write!(f, "{BEING}:{input}"),
            Leaf::Source(scope, name) => write!(f, "{scope}:{name}"),
            Leaf::Policy(port) => write!(f, "{POLICY}:{port}"),
            Leaf::State(member) => write!(f, "@{member}"),
            Leaf::Spot(name) => write!(f, "{SPOT}:{name}"),
        }
    }
}

/// A parsed address. `Display` writes it back (a bare node as `node.out`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Address {
    /// a leading `/`: from the root of everything, not from the document
    pub absolute: bool,
    pub places: Vec<Place>,
    pub leaf: Leaf,
}

impl Address {
    /// The one parser. `Err` says what is wrong with the text, without the text itself (the caller names it).
    pub fn parse(s: &str) -> Result<Address, String> {
        if s.is_empty() {
            return Err("an empty address".into());
        }
        let (absolute, rest) = match s.strip_prefix('/') {
            Some(r) => (true, r),
            None => (false, s),
        };
        let mut segments: Vec<&str> = rest.split('/').collect();
        let last = segments.pop().expect("split gives one");
        let mut places = Vec::new();
        for p in segments {
            places.push(Place::parse(p)?);
        }
        if absolute && places.is_empty() {
            return Err("an absolute address names at least one place before its leaf (/deck:<id>/…)".into());
        }
        let leaf = leaf(last)?;
        Ok(Address { absolute, places, leaf })
    }

    /// a relative leaf (`<track>:<node>.<port>` and the other leaf forms), as a rig's cables, values and sticks hold:
    /// a place or a leading `/` is refused
    pub fn leaf(s: &str) -> Result<Leaf, String> {
        let a = Address::parse(s)?;
        if !a.is_relative() {
            return Err("a place in it – here only a relative address (<track>:<node>.<port>, being:<input>, policy:<port>)".into());
        }
        Ok(a.leaf)
    }

    /// no place, no leading `/`: it stands in its own document
    pub fn is_relative(&self) -> bool {
        !self.absolute && self.places.is_empty()
    }

    /// The address with `place` as its holder – the last place, whose leaf it is (`/deck:pc/scene:demo/character:catpope/…`
    /// → `/deck:pc/scene:demo/item:incense/…`); with no place, `place` is the only one. The deck and scene before it stay
    /// (#477: a far world node's address, a pick from the address book).
    pub fn with_holder(&self, place: Place) -> Address {
        let mut places = self.places.clone();
        match places.last_mut() {
            Some(last) => *last = place,
            None => places.push(place),
        }
        Address { places, ..self.clone() }
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.absolute {
            f.write_str("/")?;
        }
        for p in &self.places {
            write!(f, "{p}/")?;
        }
        write!(f, "{}", self.leaf)
    }
}

/// a leaf's text
fn leaf(s: &str) -> Result<Leaf, String> {
    if let Some(member) = s.strip_prefix('@') {
        word(member, "a state's member")?;
        return Ok(Leaf::State(member.to_string()));
    }
    match s.split_once(':') {
        None => {
            let (node, port) = node_port(s);
            word(node, "a node")?;
            word(port, "a port")?;
            Ok(Leaf::Port { track: None, node: node.to_string(), port: port.to_string() })
        }
        Some((scope, rest)) => {
            word(scope, "a scope")?;
            if scope == BEING {
                word(rest, "the being's input")?;
                return Ok(Leaf::Being(rest.to_string()));
            }
            if scope == SOUND || scope == CLOCK || scope == CAMERA || scope == TRIGGER {
                word(rest, "the source's name")?;
                if source_slot(scope, rest).is_none() {
                    let names: Vec<&str> = SOURCES.iter().filter(|(s, ..)| *s == scope).map(|(_, n, _)| *n).collect();
                    return Err(format!("{scope} has no '{rest}' ({})", names.join(", ")));
                }
                return Ok(Leaf::Source(scope.to_string(), rest.to_string()));
            }
            if scope == POLICY {
                word(rest, "the policy's port")?;
                return Ok(Leaf::Policy(rest.to_string()));
            }
            if scope == SPOT {
                deck_catalog::spot::name(rest)?;
                return Ok(Leaf::Spot(rest.to_string()));
            }
            if let Some(member) = rest.strip_prefix('@') {
                word(member, "a track's member")?;
                return Ok(Leaf::Member { track: scope.to_string(), member: member.to_string() });
            }
            let (node, port) = node_port(rest);
            word(node, "a node")?;
            word(port, "a port")?;
            Ok(Leaf::Port { track: Some(scope.to_string()), node: node.to_string(), port: port.to_string() })
        }
    }
}

/// one name of the grammar: not empty, none of the grammar's own characters, no whitespace, `#` reserved (also a scene
/// document's holder ids, spots and fillers' levels, scene_document.rs)
pub(crate) fn word<'a>(s: &'a str, what: &str) -> Result<&'a str, String> {
    if s.is_empty() {
        return Err(format!("{what} is missing"));
    }
    if let Some(c) = s.chars().find(|c| matches!(c, '/' | ':' | '.' | '@' | '#') || c.is_whitespace()) {
        return Err(if c == '#' { format!("'#' in {what} is reserved (a second copy of a holder, later)") } else { format!("'{c}' in {what} '{s}'") });
    }
    Ok(s)
}

/// `"node.port"` → (node, port); a bare `"node"` → (node, "out"). The leaf rule alone, for the paths inside an instrument
/// and the scene's links, which are leaves of the document's own nodes.
pub fn node_port(r: &str) -> (&str, &str) {
    match r.split_once('.') {
        Some((n, p)) => (n, p),
        None => (r, "out"),
    }
}

/// How far an address may reach from the document it stands in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// this document alone – the rig on the empty rigging stage
    Here,
    /// everything: other scenes, characters, rigs and linked decks (the owner's choice from the start)
    Wide,
}

/// The document an address is read in.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Context {
    /// the document's own place, as a scene would name it (a rig: `Place::of_subject`); none for a scene's own graph
    pub own: Option<Place>,
}

/// What an address stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    /// the leaf, in this document
    Local(Leaf),
    /// another place: a valid address, and nothing carries its events into this deck yet (#442)
    Far { places: Vec<Place>, leaf: Leaf },
}

impl Resolved {
    pub fn is_far(&self) -> bool {
        matches!(self, Resolved::Far { .. })
    }
}

/// The reach rule. A relative address is local; `<own>/<leaf>` is local too (the document naming itself); an absolute
/// address is far even through its own place, since a document does not know which deck and scene hold it. `Reach::Here`
/// refuses what is far.
pub fn resolve(a: &Address, context: &Context, reach: Reach) -> Result<Resolved, String> {
    let local = a.is_relative() || (!a.absolute && a.places.len() == 1 && context.own.as_ref() == Some(&a.places[0]));
    if local {
        return Ok(Resolved::Local(a.leaf.clone()));
    }
    match reach {
        Reach::Here => Err(format!("'{a}' is out of reach here – only this document (the rigging stage)")),
        Reach::Wide => Ok(Resolved::Far { places: a.places.clone(), leaf: a.leaf.clone() }),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_shot_is_a_toons_chapter_and_its_storyboard_id() {
        use super::ShotAddress;
        assert_eq!(ShotAddress::parse("shot:catpope:3:sb-25-1"), Ok(ShotAddress { toon: "catpope".into(), chapter: 3, shot: "sb-25-1".into() }));
        assert_eq!(ShotAddress::parse("shot:catpope:0:sb-a").map(|s| s.chapter), Ok(0));
        for bad in ["shot:wide", "shot:catpope:03:sb-1", "shot:catpope:3:wide", "shot:catpope:3:sb-", "scene:catpope:3:sb-1", "shot::3:sb-1"] {
            assert!(ShotAddress::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn at_or_below_is_the_reference_or_a_dot_down_from_it() {
        use super::at_or_below;
        assert!(at_or_below("character:catpope", "character:catpope"), "exact match");
        assert!(at_or_below("character:catpope", "character:catpope.pope"), "a child below");
        assert!(at_or_below("character:catpope", "character:catpope.pope.ghost"), "the root reaches down to a look");
        assert!(at_or_below("character:catpope.pope", "character:catpope.pope.ghost"));
        assert!(!at_or_below("a/b", "a/bc") && !at_or_below("character:catpope", "character:catpopes"), "a shared prefix is a sibling");
        assert!(!at_or_below("character:catpopes", "character:catpope.pope"));
        assert!(!at_or_below("character:catpope.pope", "character:catpope"), "the root is not below its persona");
        assert!(!at_or_below("character:catpope", "item:catpope"));
    }

    #[test]
    fn a_shows_address_names_kind_id_persona_and_look() {
        use super::Target;
        let t = |s: &str| Target::parse(s);
        assert_eq!(t("character:*").unwrap().id, None);
        assert_eq!(t("character:catpope.mountaintiger.ghost").unwrap().below, ["mountaintiger", "ghost"]);
        assert_eq!(t("character:catpope.mountaintiger.ghost").unwrap().to_string(), "character:catpope.mountaintiger.ghost");
        assert!(t("location:nave.west").unwrap_err().contains("characters only"));
        assert!(t("character:a.b.c.d").unwrap_err().contains("no further"));
        assert!(t("character:*.ghost").is_err() && t("yellowmouse").is_err() && t("character:").is_err());
        assert!(t("character:yellowmouse").unwrap().picks_up("character:yellowmouse.ghost"));
        assert!(!t("character:yellowmouse").unwrap().picks_up("character:yellowmouseling"));
        assert!(!t("character:yellowmouse.ghost").unwrap().picks_up("character:yellowmouse"), "a look reaches only that look");
        assert!(t("location:*").unwrap().picks_up("location:nave") && !t("location:*").unwrap().picks_up("character:nave"));
        assert!(Target::action("dance:samba").is_ok() && Target::action("close_doors").is_ok());
        assert!(Target::action("dance samba").is_err() && Target::action("").is_err() && Target::action("a:b:c").is_err());
    }

    /// #516: a scene's cast words name who by address; everything else stays a word or `scope:word`
    #[test]
    fn a_scenes_cast_words_name_who() {
        use super::{CastWord, Target};
        assert_eq!(CastWord::parse("join:character:catpope"), Some(Ok(CastWord::Join("character:catpope".into()))));
        assert_eq!(CastWord::parse("leave:item:incense"), Some(Ok(CastWord::Leave("item:incense".into()))));
        assert_eq!(CastWord::parse("join:character:catpope.pope").unwrap().unwrap().who(), "character:catpope.pope");
        assert!(CastWord::parse("join:catpope").unwrap().unwrap_err().contains("<kind>:<id>"));
        assert!(CastWord::parse("join:character:*").unwrap().unwrap_err().contains("names one"));
        assert_eq!(CastWord::parse("dance:samba"), None);
        assert_eq!(CastWord::parse("join"), None, "a bare word is another action");
        assert!(Target::action("join:character:catpope").is_ok() && Target::action("join:nobody").is_err());
    }

    use super::*;

    fn port(track: Option<&str>, node: &str, port: &str) -> Leaf {
        Leaf::Port { track: track.map(str::to_string), node: node.to_string(), port: port.to_string() }
    }

    #[test]
    fn a_place_alone_is_kind_and_id() {
        assert_eq!(Place::parse("character:yellowmouse"), Ok(Place::new("character", "yellowmouse")));
        assert!(Place::parse("yellowmouse").is_err());
        assert!(Place::parse("scene:").is_err());
        assert!(Place::parse("a:b/c").is_err());
    }

    /// every path written today is an address unchanged
    #[test]
    fn todays_paths_parse_as_relative_leaves() {
        let cases = [
            ("voice:voice.ratio", port(Some("voice"), "voice", "ratio")),
            ("drone:mix", port(Some("drone"), "mix", "out")),
            ("voice:@gain", Leaf::Member { track: "voice".into(), member: "gain".into() }),
            ("voice:@preset", Leaf::Member { track: "voice".into(), member: "preset".into() }),
            ("being:hue", Leaf::Being("hue".into())),
            ("policy:heat", Leaf::Policy("heat".into())),
            ("fft.level", port(None, "fft", "level")),
            ("melody", port(None, "melody", "out")),
            ("voice_voice.ratio", port(None, "voice_voice", "ratio")),
            ("@energy", Leaf::State("energy".into())),
        ];
        for (text, want) in cases {
            let a = Address::parse(text).unwrap_or_else(|e| panic!("{text}: {e}"));
            assert!(a.is_relative(), "{text}");
            assert_eq!(a.leaf, want, "{text}");
            assert_eq!(Address::leaf(text).unwrap(), want);
        }
        assert_eq!(Address::parse("drone:mix").unwrap().to_string(), "drone:mix.out", "a bare node writes back as its out");
        assert_eq!(node_port("fft.bass"), ("fft", "bass"));
        assert_eq!(node_port("music"), ("music", "out"));
    }

    #[test]
    fn places_and_the_root() {
        let a = Address::parse("character:catpope/analog:synth.volume").unwrap();
        assert_eq!((a.absolute, a.places.as_slice(), &a.leaf), (false, &[Place::new("character", "catpope")][..], &port(Some("analog"), "synth", "volume")));
        let a = Address::parse("/deck:left/scene:nave/character:catpope/@energy").unwrap();
        assert!(a.absolute && a.places.len() == 3 && a.leaf == Leaf::State("energy".into()));
        assert_eq!(a.to_string(), "/deck:left/scene:nave/character:catpope/@energy");
        assert_eq!(Address::parse("item:incense/hiss:noise.gain").unwrap().leaf, port(Some("hiss"), "noise", "gain"));
        assert!(Address::leaf("character:catpope/analog:synth.volume").unwrap_err().contains("relative"));
    }

    /// #477: a pick from the address book is the holder; the deck and scene before it and the leaf stay
    #[test]
    fn a_new_holder_takes_the_last_place() {
        let holder = |a: &str, p: &str| Address::parse(a).unwrap().with_holder(Place::parse(p).unwrap()).to_string();
        assert_eq!(holder("/deck:pc/scene:demo/character:catpope/analog:seq.notes", "item:incense"), "/deck:pc/scene:demo/item:incense/analog:seq.notes");
        assert_eq!(holder("character:catpope/@energy", "character:boy"), "character:boy/@energy");
        assert_eq!(holder("analog:seq.notes", "item:incense"), "item:incense/analog:seq.notes");
        assert_eq!(super::CHARACTER, "character", "the generated kind word");
    }

    #[test]
    fn what_is_refused_and_why() {
        for (text, says) in [
            ("", "empty"),
            ("/fft.level", "at least one place"),
            ("nave/fft.level", "not a place"),
            ("scene:/fft.level", "id is missing"),
            ("voice:", "a node is missing"),
            ("voice:voice.", "a port is missing"),
            ("voice voice.ratio", "' '"),
            ("character:catpope#2/@energy", "reserved"),
            ("being:", "the being's input is missing"),
            ("@", "member is missing"),
            ("a.b.c", "'.' in a port"),
        ] {
            let e = Address::parse(text).unwrap_err();
            assert!(e.contains(says), "{text:?}: {e}");
        }
    }

    #[test]
    fn a_spot_on_a_model_is_named_by_its_holder_and_its_name() {
        let hand = SpotRef::parse("character:catpope/spot:hand_l").unwrap();
        assert_eq!((hand.holder(), hand.name.as_str()), (Some(&Place::new("character", "catpope")), "hand_l"));
        assert_eq!(hand.address.leaf, Leaf::Spot("hand_l".into()));
        assert_eq!(hand.address.to_string(), "character:catpope/spot:hand_l", "it writes back as it was");
        let flame = SpotRef::parse("item:lantern/spot:flame").unwrap();
        assert_eq!((flame.holder().unwrap().to_string(), flame.name.as_str()), ("item:lantern".to_string(), "flame"));
        let own = SpotRef::parse("spot:hand_l").unwrap();
        assert!(own.holder().is_none() && own.address.is_relative(), "inside the document that holds the model");
        assert_eq!(Address::leaf("spot:hand_l").unwrap(), Leaf::Spot("hand_l".into()));
        let far = SpotRef::parse("/deck:left/scene:nave/character:catpope/spot:head").unwrap();
        assert_eq!((far.holder().unwrap().id.as_str(), far.address.places.len(), far.address.absolute), ("catpope", 3, true));
        assert_eq!(Address::leaf("spot:flame").unwrap().slot(), None, "a position, no slot of the played scene");
        for (text, says) in [
            ("character:catpope/spot:", "is no spot name"),
            ("character:catpope/spot:Hand", "is no spot name"),
            ("character:catpope/spot:hand.l", "is no spot name"),
            ("character:catpope/spot:hand_l/extra", "names no spot"),
            ("spot:hand_l/voice:osc.out", "names no spot"),
            ("scene:nave/spot:altar", "not on a scene"),
            ("deck:left/spot:lens", "not on a deck"),
            ("character:catpope/@energy", "names no spot"),
            ("character:catpope", "names no spot"),
            ("character:catpope.pope/spot:hand_l", "'.' in a place's id"),
        ] {
            let e = SpotRef::parse(text).unwrap_err();
            assert!(e.contains(says), "{text:?}: {e}");
        }
        let ctx = Context { own: Place::of_subject("character.catpope") };
        assert_eq!(resolve(&hand.address, &ctx, Reach::Wide).unwrap(), Resolved::Local(Leaf::Spot("hand_l".into())), "its own model");
        assert!(resolve(&flame.address, &ctx, Reach::Wide).unwrap().is_far());
        assert_eq!(Aim::parse("spot:altar"), Ok(Aim::Spot("altar".into())), "a camera's aim still names a set's spot");
    }

    #[test]
    fn slots_are_the_compiled_names() {
        assert_eq!(Address::leaf("voice:voice.ratio").unwrap().slot().as_deref(), Some("voice_voice.ratio"));
        assert_eq!(Address::leaf("policy:heat").unwrap().slot().as_deref(), Some("policy.heat"));
        assert_eq!(Address::leaf("fft.level").unwrap().slot().as_deref(), Some("fft.level"));
        assert_eq!(Address::leaf("voice:@gain").unwrap().slot(), None);
        assert_eq!(Address::leaf("being:hue").unwrap().slot(), None);
    }

    /// the reach rule: relative and the own place are local; other places are far and wide reach takes them, here refuses
    #[test]
    fn reach_is_one_rule() {
        let ctx = Context { own: Place::of_subject("character.catpope.pope") };
        assert_eq!(ctx.own, Some(Place::new("character", "catpope")));
        let local = |t: &str| resolve(&Address::parse(t).unwrap(), &ctx, Reach::Wide).unwrap();
        assert_eq!(local("analog:synth.volume"), Resolved::Local(port(Some("analog"), "synth", "volume")));
        assert_eq!(local("character:catpope/analog:synth.volume"), Resolved::Local(port(Some("analog"), "synth", "volume")));
        let far = Address::parse("/deck:left/scene:nave/character:catpope/analog:seq.notes").unwrap();
        assert!(resolve(&far, &ctx, Reach::Wide).unwrap().is_far(), "absolute: the rig does not know its deck");
        assert!(resolve(&Address::parse("item:incense/hiss:noise.gain").unwrap(), &ctx, Reach::Wide).unwrap().is_far());
        let e = resolve(&far, &ctx, Reach::Here).unwrap_err();
        assert!(e.contains("out of reach here"), "{e}");
        assert_eq!(resolve(&Address::parse("fft.level").unwrap(), &Context::default(), Reach::Here).unwrap(), Resolved::Local(port(None, "fft", "level")));
    }
}
