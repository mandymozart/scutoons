//! The Library the server answers (`GET /api/library`, server/docs/API.md "The extended Library"): the places a member
//! asks and the query of each, a thing stepped into as the browser draws it. The server decides what an entry is, its
//! name, its picture and what a place holds; reading its answer (the generated types) is deck-space `library::served`.
use super::Entry;

/// a place of the Library (LIBRARY.md "places"): what the open item uses, the toon, everything, the unused files, what
/// was added lately
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Place {
    /// what the open item uses (`scene:nave`, `character:catpope`)
    Item(String),
    Toon,
    All,
    Unused,
    Recent,
}

impl Place {
    /// how a chip names it
    pub fn word(&self) -> &'static str {
        match self {
            Place::Item(_) => "this item",
            Place::Toon => "this toon",
            Place::All => "everything",
            Place::Unused => "unused",
            Place::Recent => "recently added",
        }
    }

    /// the places that can be asked with this open item and toon, in the chips' order
    pub fn offered(open: Option<&str>, toon: Option<&str>) -> Vec<Place> {
        let mut out: Vec<Place> = open.filter(|o| !o.starts_with("toon:")).map(|o| Place::Item(o.to_string())).into_iter().collect();
        if toon.is_some() {
            out.push(Place::Toon);
        }
        out.extend([Place::All, Place::Unused, Place::Recent]);
        out
    }

    /// where a member starts: what the open item uses, else the toon, else everything
    pub fn start(open: Option<&str>, toon: Option<&str>) -> Place {
        Place::offered(open, toon).swap_remove(0)
    }

    /// the query of its first page (`?` not included): the toon it is asked in, the place, the locale, one large page
    pub fn query(&self, toon: Option<&str>, locale: &str) -> String {
        let place = match self {
            Place::Item(open) => format!("item:{open}"),
            Place::Toon => "toon".into(),
            Place::All => "all".into(),
            Place::Unused => "unused".into(),
            Place::Recent => "recent".into(),
        };
        let toon = toon.filter(|_| *self != Place::All).map(|t| format!("toon={}&", encode(t))).unwrap_or_default();
        format!("{toon}place={}&locale={}&limit={PAGE}", encode(&place), encode(locale))
    }
}

/// entries asked per page (the server's most)
pub const PAGE: u32 = 5000;

/// the query of the page after one whose `next` cursor is `next` (none: it was the last)
pub fn next_query(first: &str, next: Option<&str>) -> Option<String> {
    next.map(|c| format!("{first}&cursor={}", encode(c)))
}

/// the query of a thing stepped into
pub fn step_query(address: &str, locale: &str) -> String {
    format!("at={}&locale={}", encode(address), encode(locale))
}

/// a query value: what is not unreserved is percent-encoded
pub fn encode(v: &str) -> String {
    v.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b':' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// one level inside a thing: how deep it hangs, its address, its kind's word (persona, look, mood, season, chapter), its
/// name, its parent and – a mood – its id
#[derive(Debug, Clone, PartialEq)]
pub struct Level {
    pub depth: usize,
    pub address: String,
    pub level: String,
    pub name: String,
    pub parent: String,
    pub mood: Option<String>,
}

/// a thing that uses the one stepped into: its address, whether it is an entry of the Library (a season is not), its
/// name, how it uses it (cast, credits, rig, reference)
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub address: String,
    pub thing: bool,
    pub name: String,
    pub role: String,
}

/// A thing stepped into, as the browser draws it: the thing, its levels, the things and the files it uses as entries,
/// what uses it.
#[derive(Debug, Clone, PartialEq)]
pub struct Inside {
    pub at: Entry,
    pub levels: Vec<Level>,
    pub uses: Vec<Entry>,
    pub used_in: Vec<Link>,
}

/// how deep each level hangs: the number of its parents among the levels (a mood's parent is its look, never a mood)
pub fn depths(levels: &[(String, String, bool)]) -> Vec<usize> {
    levels
        .iter()
        .map(|(_, parent, _)| {
            let mut n = 0;
            let mut up = parent.as_str();
            while let Some((_, p, _)) = levels.iter().find(|(a, _, mood)| a == up && !mood) {
                n += 1;
                if n > levels.len() {
                    break;
                }
                up = p.as_str();
            }
            n
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_places_and_their_queries() {
        assert_eq!(Place::offered(Some("scene:nave"), Some("catpope")), [Place::Item("scene:nave".into()), Place::Toon, Place::All, Place::Unused, Place::Recent]);
        assert_eq!(Place::start(None, Some("catpope")), Place::Toon);
        assert_eq!(Place::start(Some("toon:catpope"), None), Place::All);
        assert_eq!(Place::Item("scene:nave".into()).query(Some("catpope"), "en"), "toon=catpope&place=item:scene:nave&locale=en&limit=5000");
        assert_eq!(Place::All.query(Some("catpope"), "de"), "place=all&locale=de&limit=5000", "everything is every toon's");
        assert_eq!(step_query("character:cat pope", "en"), "at=character:cat%20pope&locale=en");
        assert_eq!(next_query("place=all", Some("a/b")).as_deref(), Some("place=all&cursor=a%2Fb"));
    }

    #[test]
    fn a_level_hangs_under_its_parents() {
        let l = |a: &str, p: &str, mood: bool| (a.to_string(), p.to_string(), mood);
        let levels = [l("character:c.human", "character:c", false), l("character:c.human.helmet", "character:c.human", false), l("character:c.human.helmet", "character:c.human.helmet", true)];
        assert_eq!(depths(&levels), [0, 1, 2], "a mood shares its look's address and hangs under it");
    }
}
