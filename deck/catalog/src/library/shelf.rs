//! The Library's shelf: the families with what the open item uses, and the one rule of what a view shows – the
//! scope (used by the open item, the toon, all, unused), the search, the kind, the places (missing credits, one
//! item), what a picker's slot takes, the sort and the groups (in All the open item's files pinned on
//! top, then the unused, then one group per other item, then the toon-wide ones, all but the first folded until a
//! search). The uses are data the caller read from the open item's document; the shelf links nothing.
use super::family::{self, Family, Frame, Origin};
use super::Kind;
use serde::Serialize;
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::collections::BTreeMap;

/// one use of a file by an item, read from its document
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Use {
    /// the file (any version's path)
    pub path: String,
    /// the item's address (`chapter:catpope:3`, `character:catpope`)
    pub item: String,
    /// the part in words, as the caller names it (`slice 4 · layer 2`, `mood happy`)
    pub part: String,
    /// where a click on it leads in the item (the caller's anchor)
    pub at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scope {
    /// what the open item uses, wherever the file lies
    #[default]
    Context,
    Toon,
    All,
    Unused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Grouping {
    #[default]
    Use,
    Kind,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sort {
    /// numbers as numbers: slice 2 before slice 10
    #[default]
    Name,
    /// the newest first
    Recent,
    /// the biggest first
    Size,
    /// the most used first
    Uses,
}

/// a place of the browser beyond the scope: missing credits, one item's files (what was added lately is the server's
/// place, served.rs)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Special {
    Credits,
    /// the families lacking a version they should have (shown on demand)
    Missing,
    Item(String),
    /// the files made for the characters of one cast (a season's: `season:catpope:1`)
    Cast(String),
}

/// one cast: whose files belong to it (a toon season's cast, read from its document by the caller)
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Cast {
    /// the cast's address (`season:catpope:1`)
    pub address: String,
    /// the characters in it (`catpope`, or a persona `catpope.biker`)
    pub characters: Vec<String>,
}

/// what a slot takes: its kinds, a square picture (a mood's portrait)
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Fit {
    pub kinds: Vec<Kind>,
    pub square: bool,
}

/// why a family does not fit a slot (the caller words it)
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Misfit {
    pub wants: Vec<Kind>,
    pub got: Kind,
    /// the slot wants a square picture and this one is not
    pub square: bool,
}

/// A square slot takes a picture whose sides differ by at most this share.
pub const SQUARE_TOLERANCE: f64 = 0.02;

impl Misfit {
    /// why it does not fit, in words (the sentence a refused drop shows)
    pub fn sentence(&self) -> String {
        if self.square {
            return "a square picture fits here: this one is not square".into();
        }
        let wants: Vec<&str> = self.wants.iter().map(|k| k.word()).collect();
        format!("{} fit here, not {}", wants.join(" or "), self.got.word())
    }
}

impl Fit {
    /// whether `f` fits, else what is wrong (unknown sizes fit a square slot)
    pub fn takes(&self, f: &Family) -> Result<(), Misfit> {
        let kind_ok = self.kinds.is_empty() || self.kinds.contains(&f.kind);
        let square_ok = !self.square
            || match (f.width, f.height) {
                (Some(w), Some(h)) => (f64::from(w) - f64::from(h)).abs() <= f64::from(w.max(h)) * SQUARE_TOLERANCE,
                _ => true,
            };
        if kind_ok && square_ok {
            Ok(())
        } else {
            Err(Misfit { wants: self.kinds.clone(), got: f.kind, square: kind_ok && !square_ok })
        }
    }

    /// the kind a picker opens on: its one kind, else all
    pub fn start_kind(&self) -> Option<Kind> {
        (self.kinds.len() == 1).then(|| self.kinds[0])
    }
}

/// what a view asks for
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Query {
    pub scope: Scope,
    pub grouping: Grouping,
    pub sort: Sort,
    /// every word in the key, the title, a tag or a version's label
    pub query: String,
    pub kind: Option<Kind>,
    pub special: Option<Special>,
    pub origin: Option<Origin>,
    /// a picker: only what fits is shown
    pub fit: Option<Fit>,
    /// only what is made for this character (the deck's "related")
    pub related: Option<String>,
    /// every file on its own instead of the families' stacks
    pub single: bool,
    /// the kinds the workspace lists at all (none = every kind): the rest are not counted anywhere
    pub within: Vec<Kind>,
}

/// the open item and what it uses
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Context {
    /// the open item's address; None on Home
    pub open: Option<String>,
    pub toon: Option<String>,
    /// no use index for this item: its files are the ones in its folder (its home)
    pub by_home: bool,
    pub uses: Vec<Use>,
    /// (main file, part file) links read from documents: frames
    pub links: Vec<(String, String)>,
    /// the casts the places offer as a facet
    pub casts: Vec<Cast>,
}

/// what a group of the grid is
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GroupKind {
    /// the open item's files (framed, on top)
    Open,
    Unused,
    /// one other item's files (`item` is its address)
    Item,
    /// the toon's files no item claims
    Toon,
    /// everything else (other toons, the universe's shared files)
    Other,
    /// one kind's files (`item` is its id)
    Kind,
    All,
}

/// one cell of a group: one family, or a frame's families together
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Cell {
    pub keys: Vec<String>,
    pub frame: Option<Kind>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Group {
    /// stable: what the caller keeps a group's fold under
    pub key: String,
    pub what: GroupKind,
    /// the item's address or the kind's id
    pub item: Option<String>,
    pub pinned: bool,
    /// how it starts: in All every group but the pinned one is folded (the first one open when none is pinned); a
    /// search unfolds them all
    pub folded: bool,
    pub count: usize,
    pub cells: Vec<Cell>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScopeCounts {
    pub context: usize,
    pub toon: usize,
    pub all: usize,
    pub unused: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Specials {
    /// None: the registry's answer carries no credits
    pub credits: Option<usize>,
    pub missing: usize,
    /// every item with files, its address and how many
    pub items: Vec<(String, usize)>,
    /// every cast with files made for its characters, its address and how many (the context's order)
    pub casts: Vec<(String, usize)>,
}

/// what a view shows
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct View {
    pub scopes: ScopeCounts,
    /// the kinds within the scope, search and places, in `Kind::ALL`'s order
    pub kinds: Vec<(Kind, usize)>,
    pub specials: Specials,
    pub origins: Vec<(Origin, usize)>,
    pub groups: Vec<Group>,
    /// every shown key in the order shown (the selection's order)
    pub order: Vec<String>,
    pub total: usize,
}

/// Natural order: digit runs compared as numbers, the rest without case.
pub fn natural(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (x.peek().copied(), y.peek().copied()) {
            (None, None) => return a.cmp(b),
            (None, _) => return Ordering::Less,
            (_, None) => return Ordering::Greater,
            (Some(p), Some(q)) if p.is_ascii_digit() && q.is_ascii_digit() => {
                let run = |it: &mut std::iter::Peekable<std::str::Chars>| {
                    let mut s = String::new();
                    while let Some(c) = it.peek().copied().filter(char::is_ascii_digit) {
                        s.push(c);
                        it.next();
                    }
                    s
                };
                let (m, n) = (run(&mut x), run(&mut y));
                let (m, n) = (m.trim_start_matches('0'), n.trim_start_matches('0'));
                let o = m.len().cmp(&n.len()).then_with(|| m.cmp(n));
                if o != Ordering::Equal {
                    return o;
                }
            }
            (Some(p), Some(q)) => {
                let o = p.to_lowercase().cmp(q.to_lowercase());
                if o != Ordering::Equal {
                    return o;
                }
                x.next();
                y.next();
            }
        }
    }
}

/// `2026-11-20 10:31:05` / `2026-11-20T10:31:05Z` (UTC) in ms since 1970
pub fn epoch_ms(stamp: &str) -> Option<f64> {
    let n = |s: Option<&str>| s.and_then(|v| v.parse::<i64>().ok());
    let mut parts = stamp.get(..10)?.split('-');
    let (y, m, d) = (n(parts.next())?, n(parts.next())?, n(parts.next())?);
    let time = stamp.get(11..19).unwrap_or("00:00:00");
    let mut t = time.split(':');
    let (h, mi, s) = (n(t.next()).unwrap_or(0), n(t.next()).unwrap_or(0), n(t.next()).unwrap_or(0));
    // days from the civil date (Howard Hinnant's algorithm)
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(((days * 86_400 + h * 3600 + mi * 60 + s) * 1000) as f64)
}

/// The item a file's folder belongs to (where an upload without a slot lands, and the group of a file no use names):
/// `<toon>/s<n>/c<m>/` a chapter, `<toon>/s<n>/` a season, `characters/<id>[-persona]/` a character, `…/items/` the
/// catalog, `…/map/` the map, `<toon>/` the toon.
pub fn home_of(key: &str) -> Option<String> {
    if let Some(c) = family::character_of(key) {
        return Some(format!("character:{c}"));
    }
    let parts: Vec<&str> = key.split('/').collect();
    if parts.len() < 2 {
        return None;
    }
    fn numbered(s: &str, p: char) -> Option<&str> {
        s.strip_prefix(p).filter(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
    }
    let folders = &parts[..parts.len() - 1];
    if folders.contains(&"items") {
        return Some("catalog".into());
    }
    if folders.contains(&"map") {
        return Some("map".into());
    }
    let toon = folders[0];
    if toon == "characters" || toon == "shared" {
        return None;
    }
    match (folders.get(1).and_then(|s| numbered(s, 's')), folders.get(2).and_then(|s| numbered(s, 'c'))) {
        (Some(_), Some(c)) => Some(format!("chapter:{toon}:{c}")),
        (Some(s), None) => Some(format!("season:{toon}:{s}")),
        _ => Some(format!("toon:{toon}")),
    }
}

/// the toon an item address belongs to
fn toon_of_item(address: &str) -> Option<&str> {
    let (kind, rest) = address.split_once(':')?;
    matches!(kind, "toon" | "season" | "chapter" | "storyboard").then(|| rest.split(':').next().unwrap_or(rest))
}

/// The families and what the open item uses: the one place a Library view is computed.
#[derive(Debug, Clone, Default)]
pub struct Shelf {
    families: Vec<Family>,
    singles: Vec<Family>,
    context: Context,
    frames: Vec<Frame>,
    /// per family index (stacked), the uses that name one of its files
    uses_of: Vec<Vec<usize>>,
    uses_of_single: Vec<Vec<usize>>,
    /// a family's key and every version's path → its index (stacked)
    by_path: BTreeMap<String, usize>,
}

fn index(families: &[Family], uses: &[Use]) -> Vec<Vec<usize>> {
    let mut by_path: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, f) in families.iter().enumerate() {
        for v in &f.versions {
            by_path.insert(v.path.as_str(), i);
        }
    }
    let mut out = vec![Vec::new(); families.len()];
    for (u, use_) in uses.iter().enumerate() {
        if let Some(&i) = by_path.get(use_.path.as_str()) {
            out[i].push(u);
        }
    }
    out
}

impl Shelf {
    pub fn new(families: Vec<Family>) -> Shelf {
        let singles = family::singles(&families);
        let mut by_path = BTreeMap::new();
        for (i, f) in families.iter().enumerate() {
            by_path.insert(f.key.clone(), i);
            for v in &f.versions {
                by_path.insert(v.path.clone(), i);
            }
        }
        let mut s = Shelf { families, singles, by_path, ..Shelf::default() };
        s.set_context(Context::default());
        s
    }

    pub fn families(&self, single: bool) -> &[Family] {
        if single {
            &self.singles
        } else {
            &self.families
        }
    }

    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }

    pub fn set_context(&mut self, context: Context) {
        self.uses_of = index(&self.families, &context.uses);
        self.uses_of_single = index(&self.singles, &context.uses);
        self.frames = family::frames(&self.families, &context.links);
        self.context = context;
    }

    /// the family a file (any version's path) or a key belongs to
    pub fn family_of(&self, path: &str) -> Option<&Family> {
        self.by_path.get(path).map(|&i| &self.families[i]).or_else(|| self.singles.iter().find(|f| f.key == path))
    }

    /// the family of a key or a file as the view shows it: single levels, every file its own
    pub fn family_in(&self, path: &str, single: bool) -> Option<&Family> {
        if single {
            self.singles.iter().find(|f| f.key == path)
        } else {
            self.family_of(path)
        }
    }

    /// Where a new file lands without a slot of its own: the open item's folder (a chapter's is where its files already
    /// are: its season is not in its address), else the toon's; None = the person picks a folder.
    pub fn upload_folder(&self) -> Option<String> {
        let open = self.context.open.as_deref().unwrap_or("");
        let parts: Vec<&str> = open.split(':').collect();
        let own = match parts.as_slice() {
            ["toon", t] => Some(format!("{t}/")),
            ["season", t, n] => Some(format!("{t}/s{n}/")),
            ["character", c] => Some(format!("characters/{c}/")),
            ["chapter" | "storyboard", ..] => self.families.iter().find(|f| at_home(open, &f.key)).map(|f| format!("{}/", f.folder)),
            _ => None,
        };
        own.or_else(|| self.context.toon.as_ref().map(|t| format!("{t}/")))
    }

    /// A new file's path: the slot's own path with the file's extension, else the upload folder and the file's name in
    /// lower case, every other character a `-`.
    pub fn upload_path(&self, file_name: &str, slot_path: Option<&str>) -> Option<String> {
        let ext = file_name.rfind('.').map_or("", |i| &file_name[i..]);
        if let Some(slot) = slot_path.filter(|s| !s.is_empty()) {
            let stem = slot.rfind('.').filter(|&i| i > slot.rfind('/').map_or(0, |s| s + 1)).map_or(slot, |i| &slot[..i]);
            return Some(format!("{stem}{}", ext.to_lowercase()));
        }
        let name: String = file_name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') { c } else { '-' }).collect();
        self.upload_folder().map(|f| format!("{f}{name}"))
    }

    /// the uses of a family (stacked or single)
    pub fn uses(&self, key: &str) -> Vec<&Use> {
        let pick = |list: &[Family], idx: &[Vec<usize>]| list.iter().position(|f| f.key == key).map(|i| idx[i].iter().map(|&u| &self.context.uses[u]).collect::<Vec<_>>());
        pick(&self.families, &self.uses_of).or_else(|| pick(&self.singles, &self.uses_of_single)).unwrap_or_default()
    }

    fn in_context(&self, f: &Family, uses: &[usize]) -> bool {
        let Some(open) = self.context.open.as_deref() else { return false };
        if self.context.by_home {
            return at_home(open, &f.key);
        }
        uses.iter().any(|&u| self.context.uses[u].item == open)
    }

    fn in_toon(&self, f: &Family, uses: &[usize]) -> bool {
        let Some(toon) = self.context.toon.as_deref() else { return true };
        f.key.starts_with(&format!("{toon}/")) || uses.iter().any(|&u| toon_of_item(&self.context.uses[u].item) == Some(toon))
    }

    fn unused(f: &Family, uses: &[usize]) -> bool {
        f.usages == 0 && uses.is_empty()
    }

    fn in_scope(&self, scope: Scope, f: &Family, uses: &[usize]) -> bool {
        match scope {
            Scope::Context => self.in_context(f, uses),
            Scope::Toon => self.in_toon(f, uses),
            Scope::All => true,
            Scope::Unused => Self::unused(f, uses),
        }
    }

    /// the item a family is grouped under in All when the open item does not use it: the first other item a use
    /// names, else its folder's home
    fn item_of(&self, f: &Family, uses: &[usize]) -> Option<String> {
        let open = self.context.open.as_deref();
        uses.iter().map(|&u| &self.context.uses[u].item).find(|i| Some(i.as_str()) != open).cloned().or_else(|| home_of(&f.key).filter(|h| Some(h.as_str()) != open))
    }

    fn matches(f: &Family, query: &str) -> bool {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        if words.is_empty() {
            return true;
        }
        let labels: Vec<&str> = f.versions.iter().map(|v| v.label.as_str()).collect();
        let hay = format!("{} {} {} {}", f.key, f.title, f.tags.join(" "), labels.join(" ")).to_lowercase();
        words.iter().all(|w| hay.contains(w.as_str()))
    }

    fn special(&self, s: &Special, f: &Family, uses: &[usize]) -> bool {
        match s {
            Special::Credits => f.credits == Some(false),
            Special::Missing => !f.missing.is_empty(),
            Special::Item(item) => self.items_of(f, uses).iter().any(|i| i == item),
            Special::Cast(cast) => self.context.casts.iter().find(|c| &c.address == cast).is_some_and(|c| Self::in_cast(f, c)),
        }
    }

    /// whether a family is made for one of the cast's characters
    fn in_cast(f: &Family, cast: &Cast) -> bool {
        cast.characters.iter().any(|c| crate::made_for(&f.made_for, c))
    }

    /// every item a family counts for in the places: the items its uses name and its home
    fn items_of(&self, f: &Family, uses: &[usize]) -> Vec<String> {
        let mut out: Vec<String> = uses.iter().map(|&u| self.context.uses[u].item.clone()).collect();
        out.extend(home_of(&f.key).filter(|h| self.context.by_home || Some(h.as_str()) != self.context.open.as_deref()));
        out.sort();
        out.dedup();
        out
    }

    fn cmp(sort: Sort, a: &Family, b: &Family) -> Ordering {
        let name = || natural(&a.key, &b.key);
        match sort {
            Sort::Name => name(),
            Sort::Recent => b.updated.as_deref().and_then(epoch_ms).unwrap_or(0.0).total_cmp(&a.updated.as_deref().and_then(epoch_ms).unwrap_or(0.0)).then_with(name),
            Sort::Size => b.size.cmp(&a.size).then_with(name),
            Sort::Uses => b.usages.cmp(&a.usages).then_with(name),
        }
    }

    /// the view `q` asks for
    pub fn view(&self, q: &Query) -> View {
        let list = self.families(q.single);
        let idx = if q.single { &self.uses_of_single } else { &self.uses_of };
        // a frame's parts come with their main, whatever their own kind
        let part_of_listed = |key: &str| !q.single && self.frames.iter().any(|fr| q.within.contains(&fr.kind) && fr.parts.iter().any(|p| p == key));
        let all: Vec<usize> = (0..list.len()).filter(|&i| q.within.is_empty() || q.within.contains(&list[i].kind) || part_of_listed(&list[i].key)).collect();
        let scopes = ScopeCounts {
            context: all.iter().filter(|&&i| self.in_context(&list[i], &idx[i])).count(),
            toon: all.iter().filter(|&&i| self.in_toon(&list[i], &idx[i])).count(),
            all: all.len(),
            unused: all.iter().filter(|&&i| Self::unused(&list[i], &idx[i])).count(),
        };
        let scoped: Vec<usize> = all.iter().copied().filter(|&i| self.in_scope(q.scope, &list[i], &idx[i])).collect();
        let fits = |i: usize| q.fit.as_ref().is_none_or(|fit| fit.takes(&list[i]).is_ok());
        let related = |i: usize| q.related.as_deref().is_none_or(|c| crate::made_for(&list[i].made_for, c.strip_prefix("character:").unwrap_or(c)));
        let searched: Vec<usize> = scoped.iter().copied().filter(|&i| Self::matches(&list[i], &q.query) && fits(i) && related(i)).collect();
        let specials = Specials {
            credits: all.iter().any(|&i| list[i].credits.is_some()).then(|| searched.iter().filter(|&&i| list[i].credits == Some(false)).count()),
            missing: searched.iter().filter(|&&i| !list[i].missing.is_empty()).count(),
            items: {
                let mut counts: BTreeMap<String, usize> = BTreeMap::new();
                for &i in &searched {
                    for item in self.items_of(&list[i], &idx[i]) {
                        *counts.entry(item).or_default() += 1;
                    }
                }
                let mut v: Vec<(String, usize)> = counts.into_iter().collect();
                v.sort_by(|a, b| natural(&a.0, &b.0));
                v
            },
            casts: self
                .context
                .casts
                .iter()
                .map(|c| (c.address.clone(), searched.iter().filter(|&&i| Self::in_cast(&list[i], c)).count()))
                .filter(|(_, n)| *n > 0)
                .collect(),
        };
        let placed: Vec<usize> = searched.iter().copied().filter(|&i| q.special.as_ref().is_none_or(|s| self.special(s, &list[i], &idx[i]))).collect();
        let origins = Origin::ALL.iter().map(|&o| (o, placed.iter().filter(|&&i| list[i].origin == o).count())).collect();
        let kinds = Kind::ALL.iter().map(|&k| (k, placed.iter().filter(|&&i| list[i].kind == k).count())).filter(|(_, n)| *n > 0).collect();
        let shown: Vec<usize> = placed.into_iter().filter(|&i| q.kind.is_none_or(|k| list[i].kind == k) && q.origin.is_none_or(|o| list[i].origin == o)).collect();
        let groups = self.groups(list, idx, &shown, q);
        let order = groups.iter().flat_map(|g| g.cells.iter().flat_map(|c| c.keys.iter().cloned())).collect();
        View { scopes, kinds, specials, origins, groups, order, total: shown.len() }
    }

    fn groups(&self, list: &[Family], idx: &[Vec<usize>], shown: &[usize], q: &Query) -> Vec<Group> {
        if shown.is_empty() {
            return Vec::new();
        }
        let group = |key: String, what: GroupKind, item: Option<String>, pinned: bool, members: Vec<usize>| {
            let mut members = members;
            members.sort_by(|&a, &b| Self::cmp(q.sort, &list[a], &list[b]));
            let cells = self.cells(list, &members, q.single);
            Group { key, what, item, pinned, folded: false, count: members.len(), cells }
        };
        let mut groups: Vec<Group> = if q.scope == Scope::Context {
            vec![group("open".into(), GroupKind::Open, self.context.open.clone(), true, shown.to_vec())]
        } else if q.grouping == Grouping::Kind {
            Kind::ALL
                .iter()
                .filter_map(|&k| {
                    let members: Vec<usize> = shown.iter().copied().filter(|&i| list[i].kind == k).collect();
                    (!members.is_empty()).then(|| group(format!("kind:{}", k.id()), GroupKind::Kind, Some(k.id().into()), false, members))
                })
                .collect()
        } else if q.scope == Scope::Unused {
            vec![group("unused".into(), GroupKind::Unused, None, false, shown.to_vec())]
        } else if q.grouping == Grouping::None {
            vec![group("all".into(), GroupKind::All, None, false, shown.to_vec())]
        } else {
            let mut open = Vec::new();
            let mut unused = Vec::new();
            let mut items: BTreeMap<String, Vec<usize>> = BTreeMap::new();
            let mut toon = Vec::new();
            let mut other = Vec::new();
            let toon_item = self.context.toon.as_ref().map(|t| format!("toon:{t}"));
            let toon_prefix = self.context.toon.as_ref().map(|t| format!("{t}/"));
            for &i in shown {
                let (f, uses) = (&list[i], &idx[i]);
                if self.in_context(f, uses) {
                    open.push(i);
                } else if Self::unused(f, uses) {
                    unused.push(i);
                } else {
                    // the open item's folder holds files it does not use: they count as the toon's
                    match self.item_of(f, uses) {
                        Some(item) if Some(&item) == toon_item.as_ref() => toon.push(i),
                        Some(item) => items.entry(item).or_default().push(i),
                        None if toon_prefix.as_ref().is_some_and(|p| f.key.starts_with(p.as_str())) => toon.push(i),
                        None => other.push(i),
                    }
                }
            }
            let mut out = Vec::new();
            if !open.is_empty() {
                out.push(group("open".into(), GroupKind::Open, self.context.open.clone(), true, open));
            }
            if !unused.is_empty() {
                out.push(group("unused".into(), GroupKind::Unused, None, false, unused));
            }
            let mut named: Vec<(String, Vec<usize>)> = items.into_iter().collect();
            named.sort_by(|a, b| natural(&a.0, &b.0));
            for (item, members) in named {
                out.push(group(format!("item:{item}"), GroupKind::Item, Some(item), false, members));
            }
            if !toon.is_empty() {
                out.push(group("toon".into(), GroupKind::Toon, self.context.toon.clone().map(|t| format!("toon:{t}")), false, toon));
            }
            if !other.is_empty() {
                out.push(group("other".into(), GroupKind::Other, None, false, other));
            }
            out
        };
        let folds = matches!(q.scope, Scope::All | Scope::Toon) && q.query.trim().is_empty() && groups.len() > 1;
        let any_pinned = groups.iter().any(|g| g.pinned);
        for (n, g) in groups.iter_mut().enumerate() {
            g.folded = folds && !g.pinned && (any_pinned || n > 0);
        }
        groups
    }

    /// a group's families as cells: a frame's parts follow its main when both are in the group
    fn cells(&self, list: &[Family], members: &[usize], single: bool) -> Vec<Cell> {
        let keys: Vec<&str> = members.iter().map(|&i| list[i].key.as_str()).collect();
        let frames: Vec<&Frame> = if single { Vec::new() } else { self.frames.iter().filter(|fr| keys.contains(&fr.main.as_str())).collect() };
        let in_frame = |k: &str| frames.iter().any(|fr| fr.parts.iter().any(|p| p == k));
        let mut out = Vec::new();
        for k in keys.iter().copied() {
            if in_frame(k) {
                continue;
            }
            match frames.iter().find(|fr| fr.main == k) {
                Some(fr) => {
                    let mut ks = vec![k.to_string()];
                    ks.extend(fr.parts.iter().filter(|p| keys.contains(&p.as_str())).cloned());
                    out.push(Cell { keys: ks, frame: Some(fr.kind) });
                }
                None => out.push(Cell { keys: vec![k.to_string()], frame: None }),
            }
        }
        out
    }
}

/// whether a file lies in the open item's folder: a toon's or a season's folder holds its chapters' files too; a
/// storyboard's files are its chapter's
pub fn at_home(open: &str, key: &str) -> bool {
    let open = open.strip_prefix("storyboard:").map_or_else(|| open.to_string(), |r| format!("chapter:{r}"));
    let parts: Vec<&str> = open.split(':').collect();
    match parts.as_slice() {
        ["toon", t] => key.starts_with(&format!("{t}/")),
        ["season", t, n] => key.starts_with(&format!("{t}/s{n}/")),
        _ => home_of(key).is_some_and(|h| h == open),
    }
}

/// The next selection after a click: a plain click one, Ctrl / ⌘ adds or takes one, Shift the run from the anchor to it
/// in the order shown.
pub fn select(chosen: &[String], anchor: Option<&str>, id: &str, order: &[String], shift: bool, toggle: bool) -> (Vec<String>, Option<String>) {
    if let (true, Some(a)) = (shift, anchor) {
        if let (Some(from), Some(to)) = (order.iter().position(|k| k == a), order.iter().position(|k| k == id)) {
            let mut out = chosen.to_vec();
            for k in &order[from.min(to)..=from.max(to)] {
                if !out.contains(k) {
                    out.push(k.clone());
                }
            }
            return (out, Some(a.to_string()));
        }
    }
    if toggle {
        let mut out = chosen.to_vec();
        match out.iter().position(|k| k == id) {
            Some(p) => {
                out.remove(p);
            }
            None => out.push(id.to_string()),
        }
        return (out, Some(id.to_string()));
    }
    (vec![id.to_string()], Some(id.to_string()))
}

// ── the wire: JSON in and out, for the wasm bridge (space-web `Library`) ─────────────────────────────────────────

fn kinds_of(v: &Value) -> Vec<Kind> {
    v.as_array().map(|a| a.iter().filter_map(|k| k.as_str().and_then(Kind::of_id)).collect()).unwrap_or_default()
}

/// `{ kinds: ["image"], square? }`
pub fn fit_of(v: &Value) -> Option<Fit> {
    v.is_object().then(|| Fit { kinds: kinds_of(&v["kinds"]), square: v["square"] == Value::Bool(true) })
}

/// a view's query as JSON: `{ scope, grouping, sort, query, kind, special: "recent" | "credits" | "item:<address>" | "cast:<address>",
/// origin, fit, related, single, now, within }`
pub fn query_of(v: &Value) -> Query {
    let s = |k: &str| v[k].as_str().unwrap_or("");
    Query {
        scope: match s("scope") {
            "toon" => Scope::Toon,
            "all" => Scope::All,
            "unused" => Scope::Unused,
            _ => Scope::Context,
        },
        grouping: match s("grouping") {
            "kind" => Grouping::Kind,
            "none" => Grouping::None,
            _ => Grouping::Use,
        },
        sort: match s("sort") {
            "recent" => Sort::Recent,
            "size" => Sort::Size,
            "uses" => Sort::Uses,
            _ => Sort::Name,
        },
        query: s("query").to_string(),
        kind: Kind::of_id(s("kind")),
        special: match s("special") {
            "credits" => Some(Special::Credits),
            "missing" => Some(Special::Missing),
            x => x.strip_prefix("item:").map(|i| Special::Item(i.to_string())).or_else(|| x.strip_prefix("cast:").map(|c| Special::Cast(c.to_string()))),
        },
        origin: Origin::of_id(s("origin")),
        fit: fit_of(&v["fit"]),
        related: v["related"].as_str().filter(|r| !r.is_empty()).map(str::to_string),
        single: v["single"] == Value::Bool(true),
        within: kinds_of(&v["within"]),
    }
}

/// the open item and its uses as JSON: `{ open, toon, byHome, uses: [{ path, item, part, at }], links: [[main, part]],
/// casts: [{ address, characters }] }`
pub fn context_of(v: &Value) -> Context {
    let s = |x: &Value| x.as_str().map(str::to_string);
    Context {
        open: s(&v["open"]),
        toon: s(&v["toon"]),
        by_home: v["byHome"] == Value::Bool(true),
        uses: v["uses"]
            .as_array()
            .map(|a| a.iter().filter_map(|u| Some(Use { path: s(&u["path"])?, item: s(&u["item"])?, part: s(&u["part"]).unwrap_or_default(), at: s(&u["at"]).unwrap_or_default() })).collect())
            .unwrap_or_default(),
        links: v["links"].as_array().map(|a| a.iter().filter_map(|l| Some((s(&l[0])?, s(&l[1])?))).collect()).unwrap_or_default(),
        casts: v["casts"]
            .as_array()
            .map(|a| a.iter().filter_map(|c| Some(Cast { address: s(&c["address"])?, characters: c["characters"].as_array().map(|x| x.iter().filter_map(|n| n.as_str().map(str::to_string)).collect()).unwrap_or_default() })).collect())
            .unwrap_or_default(),
    }
}

impl Shelf {
    /// `GET /api/assets`' answer (`{ items }`) or its items
    pub fn from_json(text: &str) -> Result<Shelf, String> {
        let v: Value = serde_json::from_str(text).map_err(|e| format!("the assets' answer: {e}"))?;
        let items = v.as_array().or_else(|| v["items"].as_array()).ok_or("the assets' answer has no items")?;
        Ok(Shelf::new(family::families(items)))
    }

    /// the families as JSON (stacked or single)
    pub fn families_json(&self, single: bool) -> String {
        serde_json::to_string(self.families(single)).unwrap_or_else(|_| "[]".into())
    }

    pub fn context_json(&mut self, text: &str) -> Result<(), String> {
        let v: Value = serde_json::from_str(text).map_err(|e| format!("the context: {e}"))?;
        self.set_context(context_of(&v));
        Ok(())
    }

    pub fn view_json(&self, text: &str) -> Result<String, String> {
        let v: Value = serde_json::from_str(text).map_err(|e| format!("the query: {e}"))?;
        serde_json::to_string(&self.view(&query_of(&v))).map_err(|e| e.to_string())
    }

    /// whether the family of `path` fits `{ kinds, square }`: `{ ok: true }` or `{ ok: false, wants, got, square }`;
    /// `{ ok: false, unknown: true }` for a file the shelf does not hold
    pub fn takes_json(&self, path: &str, fit: &str) -> String {
        let fit = serde_json::from_str::<Value>(fit).ok().and_then(|v| fit_of(&v)).unwrap_or_default();
        match self.family_of(path).map(|f| fit.takes(f)) {
            Some(Ok(())) => json!({ "ok": true }),
            Some(Err(m)) => json!({ "ok": false, "wants": m.wants, "got": m.got, "square": m.square }),
            None => json!({ "ok": false, "unknown": true }),
        }
        .to_string()
    }

    /// the uses of a family as JSON `[{ path, item, part, at }]`
    pub fn uses_json(&self, key: &str) -> String {
        serde_json::to_string(&self.uses(key)).unwrap_or_else(|_| "[]".into())
    }

    /// `{ name, slot? }` → the new file's path, or null
    pub fn upload_path_json(&self, text: &str) -> String {
        let v: Value = serde_json::from_str(text).unwrap_or(Value::Null);
        json!(self.upload_path(v["name"].as_str().unwrap_or(""), v["slot"].as_str())).to_string()
    }

    /// the frames as JSON `[{ main, kind, parts }]`
    pub fn frames_json(&self) -> String {
        serde_json::to_string(&self.frames).unwrap_or_else(|_| "[]".into())
    }
}

/// `{ chosen, anchor, id, order, shift, toggle }` → `{ chosen, anchor }`
pub fn select_json(text: &str) -> Result<String, String> {
    let v: Value = serde_json::from_str(text).map_err(|e| format!("the selection: {e}"))?;
    let strings = |x: &Value| x.as_array().map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect::<Vec<_>>()).unwrap_or_default();
    let (chosen, anchor) = select(&strings(&v["chosen"]), v["anchor"].as_str(), v["id"].as_str().unwrap_or(""), &strings(&v["order"]), v["shift"] == Value::Bool(true), v["toggle"] == Value::Bool(true));
    Ok(json!({ "chosen": chosen, "anchor": anchor }).to_string())
}

/// `{ kinds, square }` → the kind a picker opens on (`"image"`, or null)
pub fn start_kind_json(fit: &str) -> String {
    let kind = serde_json::from_str::<Value>(fit).ok().and_then(|v| fit_of(&v)).and_then(|f| f.start_kind());
    json!(kind).to_string()
}
