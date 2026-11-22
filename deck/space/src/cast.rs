//! The stage's cast: what form each member takes and which picture it shows (#495, research/SCENES.md §9.4 the render
//! ladder). Owner 2026-11-12 [scenes]: *"We fall back to 2d in all cases"* – the 2D card is the floor that always works.
//!
//! - **Form:** a member stands as its 3D look (the being of its rig, #492) only where 3D exists for it: a character on a
//!   stage whose set is 3D (rung 1). Everywhere else it is a **card**, its picture standing at its holder's spot: a
//!   character on a stage with no 3D set (a still or a video backdrop – rungs 2 and 3 – or no set at all) and every item
//!   (an item has no being to draw). Either form is moved only by its own rig's `being:` wires (its look node's inputs).
//! - **Picture:** a character's card shows its `full` view in the mood its being reports, else the nearest mood it has,
//!   else its portrait (`<character>_<view>-<mood>`, agents/ASSETS.md); an item's card its item picture. Where a picture
//!   comes from is a `Picture`: scene-content's entry (`kind`, `id`) and field, the same words on every deck – the browser
//!   fetches what the core asks for, a native deck reads its folder by `Picture::stem`.
//! - **Placement (the issue's open points, chosen here):** the card's feet stand on the holder's spot (a spawn zone's
//!   member at its arrangement's offset), its height in metres is fixed per kind (`card_height`) – so it shrinks with
//!   distance as anything in the room does, through the camera's perspective – its width follows the picture's aspect.
//!   It stands upright and turns to the camera around the vertical (a standee, as the heads turn), so it is never seen
//!   edge-on.
//!
//! Pure (no space, no GPU): the headless host reports the same forms the decks draw.
use crate::scene::Scene;

/// what a member of the stage is (its look node's `who`: its place, `character:<id>` / `item:<id>`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Who {
    Character(String),
    Item(String),
}

impl Who {
    /// a place (`character:catpope`, `item:incense_burner`); None for any other kind
    pub fn parse(place: &str) -> Option<Who> {
        match place.split_once(':')? {
            ("character", id) if !id.is_empty() => Some(Who::Character(id.into())),
            ("item", id) if !id.is_empty() => Some(Who::Item(id.into())),
            _ => None,
        }
    }

    /// its place again (`character:<id>`, `item:<id>`)
    pub fn place(&self) -> String {
        match self {
            Who::Character(id) => format!("character:{id}"),
            Who::Item(id) => format!("item:{id}"),
        }
    }

    pub fn id(&self) -> &str {
        match self {
            Who::Character(id) | Who::Item(id) => id,
        }
    }

    /// the character named for a being's head ("" for an item)
    pub fn character(&self) -> &str {
        match self {
            Who::Character(id) => id,
            Who::Item(_) => "",
        }
    }
}

/// how a member stands on the stage (a holder may choose it: `form = "card"` / `"look"`)
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Form {
    /// its 3D look: the being of its rig (#492)
    Look,
    /// its 2D picture standing at its spot
    Card,
}

impl Form {
    pub fn word(self) -> &'static str {
        match self {
            Form::Look => "look",
            Form::Card => "card",
        }
    }

    /// a look node's `form` word; None for anything else
    pub fn parse(word: &str) -> Option<Form> {
        [Form::Look, Form::Card].into_iter().find(|f| f.word() == word)
    }
}

/// The form of `who` on a stage that is 3D (`stage_3d`: its set's 3D is shown or on its way) or not: its holder's
/// `chosen` form, else its character look's own (`look`, scene-content's `form`), else the stage's – a chosen `look`
/// stays 3D on a stage without a set, `card` is always a card – except for an item, which has no being.
pub fn form(who: &Who, stage_3d: bool, chosen: Option<Form>, look: Option<Form>) -> Form {
    match (who, chosen.or(look)) {
        (Who::Item(_), _) => Form::Card,
        (Who::Character(_), Some(f)) => f,
        (Who::Character(_), None) if stage_3d => Form::Look,
        _ => Form::Card,
    }
}

/// The character whose pictures and model draw a member that has neither, in 2D and 3D; the member keeps its own
/// address – only its drawing falls back (push.rs `Content::drawn_as`).
pub const DEFAULT_CHARACTER: &str = "mellowmantis";

/// Whether a scene's stage is 3D before its set has arrived: its space node names a set (rung 1). A stage with no set
/// (the neutral staging, a location whose set is not here yet – 2·3) is not; a still or a video backdrop will not be.
pub fn stage_3d(scene: &Scene) -> bool {
    scene.nodes.iter().any(|n| n.kind == "space" && n.params.get("set").and_then(|v| v.as_str()).is_some_and(|s| !s.is_empty()))
}

/// a character's card picture: its whole figure
pub const CARD_VIEW: &str = "full";
/// a head's picture, and a card's last resort
pub const PORTRAIT_VIEW: &str = "portrait";
/// an item's picture (one, no moods)
pub const ITEM_VIEW: &str = "picture";
/// a picture named by its media path, not by an entry of scene-content
pub const MEDIA: &str = "media";

/// a card's height in metres: a character stands as tall as a person, an item as a thing on a table
pub fn card_height(who: &Who) -> f32 {
    match who {
        Who::Character(_) => 1.7,
        Who::Item(_) => 0.6,
    }
}

/// A picture the stage asks for: scene-content's entry (`kind`: `characters` / `items`, `id`), the view and the mood
/// (an item's: none).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    pub kind: &'static str,
    pub id: String,
    pub view: &'static str,
    pub mood: String,
}

impl Picture {
    /// a character's view in a mood
    pub fn character(id: &str, view: &'static str, mood: &str) -> Picture {
        Picture { kind: "characters", id: id.into(), view, mood: mood.into() }
    }

    /// an item's picture
    pub fn item(id: &str) -> Picture {
        Picture { kind: "items", id: id.into(), view: ITEM_VIEW, mood: String::new() }
    }

    /// a picture by its media path (a stage background's image)
    pub fn media(path: &str) -> Picture {
        Picture { kind: MEDIA, id: path.into(), view: "", mood: String::new() }
    }

    /// the field of scene-content's entry that holds it: a character's `portraits` / `full` (by mood), an item's `picture`
    pub fn field(&self) -> &'static str {
        if self.view == PORTRAIT_VIEW {
            "portraits"
        } else {
            self.view
        }
    }

    /// its file's name without the extension (agents/ASSETS.md): `<character>_<view>-<mood>`, an item's `<id>`
    pub fn stem(&self) -> String {
        if self.kind == "items" || self.kind == MEDIA {
            self.id.clone()
        } else {
            format!("{}_{}-{}", self.id, self.view, self.mood)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_character_is_3d_only_on_a_3d_stage_an_item_never() {
        let pope = Who::parse("character:catpope").unwrap();
        let burner = Who::parse("item:incense_burner").unwrap();
        assert_eq!((form(&pope, true, None, None), form(&pope, false, None, None)), (Form::Look, Form::Card));
        assert_eq!((form(&burner, true, None, None), form(&burner, false, None, None)), (Form::Card, Form::Card));
        assert_eq!(Who::parse("location:dom"), None);
        assert_eq!((pope.character(), burner.character(), burner.id()), ("catpope", "", "incense_burner"));
    }

    #[test]
    fn a_chosen_form_wins_over_the_stage() {
        let pope = Who::parse("character:catpope").unwrap();
        let burner = Who::parse("item:incense_burner").unwrap();
        assert_eq!(form(&pope, true, Some(Form::Card), None), Form::Card, "a card on a 3D stage");
        assert_eq!(form(&pope, false, Some(Form::Look), None), Form::Look, "a look on a stage without a set");
        assert_eq!(form(&burner, true, Some(Form::Look), Some(Form::Look)), Form::Card, "an item has no being to draw");
        assert_eq!((Form::parse("card"), Form::parse("look"), Form::parse("Card")), (Some(Form::Card), Some(Form::Look), None));
    }

    #[test]
    fn the_holder_wins_over_the_look_and_the_look_over_the_stage() {
        let pope = Who::parse("character:catpope").unwrap();
        assert_eq!(form(&pope, true, None, Some(Form::Card)), Form::Card, "the look's card on a 3D stage");
        assert_eq!(form(&pope, false, None, Some(Form::Look)), Form::Look, "the look's 3D on a stage without a set");
        assert_eq!(form(&pope, true, Some(Form::Look), Some(Form::Card)), Form::Look, "the holder overrides the look");
        assert_eq!(form(&pope, false, Some(Form::Card), Some(Form::Look)), Form::Card);
    }

    #[test]
    fn a_stage_is_3d_when_its_space_names_a_set() {
        let scene = |space: &str| Scene::parse(&format!("[[node]]\nid = \"space\"\nkind = \"space\"\n{space}")).unwrap();
        assert!(stage_3d(&scene("set = \"cathedral\"")));
        assert!(!stage_3d(&scene("")));
        assert!(!stage_3d(&scene("set = \"\"")));
    }

    #[test]
    fn pictures_are_named_as_the_assets_are() {
        let full = Picture::character("catpope", CARD_VIEW, "happy");
        assert_eq!((full.stem(), full.field()), ("catpope_full-happy".to_string(), "full"));
        let head = Picture::character("catpope", PORTRAIT_VIEW, "sad");
        assert_eq!((head.stem(), head.field()), ("catpope_portrait-sad".to_string(), "portraits"));
        let item = Picture::item("incense_burner");
        assert_eq!((item.stem(), item.field(), item.mood.as_str()), ("incense_burner".to_string(), "picture", ""));
    }
}
