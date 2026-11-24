//! The drop plan (2026-11-21 [rigs], research/outlets-on-models-and-node-view.md §3.2): what a Library card of each kind
//! becomes where it is let go – a cell's piece in a Show row, a track or a node of a rig on the 3D stage, a slot or a layer
//! of a scene – or the sentence why not. The one table: `Purpose::takes` answers through it, the editor only asks.
use crate::library::Kind;

/// where a card is let go
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// a Show row (the rows grid or its node view)
    ShowRow,
    /// a rig on the 3D stage (there is no 2D rig view)
    Rig,
    /// a scene's slots, layers and Modules in Build
    Scene,
}

/// what a reference node shows of its asset
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preview {
    Picture,
    Playable,
    FirstFrame,
    Model,
    Plain,
}

/// what a dropped card becomes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// a cell's piece on the outlet column under the pointer, or a new cell in a free column
    Piece,
    /// an editor-only node in the row's canvas, never played
    Reference(Preview),
    /// a track playing the module or instrument
    Track,
    /// a nested rig as a track
    NestedRig,
    /// a world node with its `being:` and spot inputs
    WorldNode,
    /// a body on the node under the pointer, its spots its port anchors
    Body,
    /// a 2D body or sketch on a node
    Sketch,
    /// a sampler track with the file as its parameter
    Sampler,
    /// a picture source node
    PictureSource,
    /// a scene slot's character or item
    Slot,
    /// a scene's parallax layer
    Layer,
    /// the Modules of the scene's camera or a member
    Modules,
}

impl Outcome {
    /// a drop of it lands today; the others are planned and refused until their ticket is built
    pub fn built(self) -> bool {
        // TODO(#717) Reference; TODO(#445) Body; TODO(#335) Sketch; the other rig nodes have no ticket yet
        matches!(self, Outcome::Piece | Outcome::Track | Outcome::Slot | Outcome::Layer | Outcome::Modules)
    }

    /// what it is called in a sentence
    pub fn words(self) -> &'static str {
        match self {
            Outcome::Piece => "a cell's piece",
            Outcome::Reference(_) => "a reference node",
            Outcome::Track => "a track",
            Outcome::NestedRig => "a nested rig",
            Outcome::WorldNode => "a world node",
            Outcome::Body => "a body on a node",
            Outcome::Sketch => "a sketch on a node",
            Outcome::Sampler => "a sampler track",
            Outcome::PictureSource => "a picture source",
            Outcome::Slot => "a slot's piece",
            Outcome::Layer => "a layer",
            Outcome::Modules => "a member's Modules",
        }
    }
}

impl View {
    fn at(self) -> &'static str {
        match self {
            View::ShowRow => "in a Show row",
            View::Rig => "on a rig",
            View::Scene => "on a scene",
        }
    }
}

/// what `kind` let go on `view` becomes, else the sentence why not
pub fn plan(view: View, kind: Kind) -> Result<Outcome, String> {
    use Kind::*;
    use Outcome::*;
    let why = match (view, kind) {
        (_, Toon | Show | Generator | Kanon | Person) => "a drop of one does nothing yet: a double click opens it",
        (View::ShowRow, Scene | Character | Item | Place) => return Ok(Piece),
        (View::ShowRow, Model) => return Ok(Reference(Preview::Model)),
        (View::ShowRow, Image) => return Ok(Reference(Preview::Picture)),
        (View::ShowRow, Audio) => return Ok(Reference(Preview::Playable)),
        (View::ShowRow, Video) => return Ok(Reference(Preview::FirstFrame)),
        (View::ShowRow, File | Animation) => return Ok(Reference(Preview::Plain)),
        (View::ShowRow, Module | Instrument | Rig) => "a row holds pieces: scenes, characters, items and places",

        (View::Rig, Module | Instrument) => return Ok(Track),
        (View::Rig, Kind::Rig) => return Ok(NestedRig),
        (View::Rig, Character | Item) => return Ok(WorldNode),
        (View::Rig, Model) => return Ok(Body),
        (View::Rig, Image) => return Ok(Sketch),
        (View::Rig, Audio) => return Ok(Sampler),
        (View::Rig, Video) => return Ok(PictureSource),
        (View::Rig, Kind::Scene | Place) => "a show's row plays it, a rig plays modules and instruments",
        (View::Rig, File) => "a rig takes no plain file: drop it in a Show row as a reference",
        (View::Rig, Animation) => "an animation plays on a character's model: pick it as the character's idle",

        (View::Scene, Character | Item) => return Ok(Slot),
        (View::Scene, Image) => return Ok(Layer),
        (View::Scene, Module) => return Ok(Modules),
        (View::Scene, _) => "its slots take characters and items, its layers pictures, the Modules of its camera and members modules",
    };
    Err(format!("{} cannot be dropped {}: {why}", kind.word(), view.at()))
}

/// whether `kind` let go on `view` lands today: the plan's outcome when it is built, else the sentence why not
pub fn lands(view: View, kind: Kind) -> Result<Outcome, String> {
    let outcome = plan(view, kind)?;
    if outcome.built() {
        Ok(outcome)
    } else {
        Err(format!("{} {} become {}: not built yet", kind.word(), view.at(), outcome.words()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Kind;

    fn table(view: View) -> Vec<(Kind, Result<Outcome, String>)> {
        Kind::ALL.iter().map(|&k| (k, plan(view, k))).collect()
    }

    #[test]
    fn a_show_row_takes_pieces_and_holds_raw_media_as_references() {
        let ok = |k| plan(View::ShowRow, k);
        for k in [Kind::Scene, Kind::Character, Kind::Item, Kind::Place] {
            assert_eq!(ok(k), Ok(Outcome::Piece), "{k:?}");
        }
        assert_eq!(ok(Kind::Model), Ok(Outcome::Reference(Preview::Model)));
        assert_eq!(ok(Kind::Image), Ok(Outcome::Reference(Preview::Picture)));
        assert_eq!(ok(Kind::Audio), Ok(Outcome::Reference(Preview::Playable)));
        assert_eq!(ok(Kind::Video), Ok(Outcome::Reference(Preview::FirstFrame)));
        assert_eq!(ok(Kind::File), Ok(Outcome::Reference(Preview::Plain)));
        assert_eq!(ok(Kind::Module), Err("modules cannot be dropped in a Show row: a row holds pieces: scenes, characters, items and places".into()));
        assert_eq!(ok(Kind::Instrument), Err("instruments cannot be dropped in a Show row: a row holds pieces: scenes, characters, items and places".into()));
        assert_eq!(ok(Kind::Rig), Err("rigs cannot be dropped in a Show row: a row holds pieces: scenes, characters, items and places".into()));
        assert_eq!(table(View::ShowRow).iter().filter(|(_, r)| r.is_err()).count(), 8);
        assert_eq!(ok(Kind::Show), Err("shows cannot be dropped in a Show row: a drop of one does nothing yet: a double click opens it".into()));
    }

    #[test]
    fn a_rig_takes_tracks_world_nodes_and_media_on_its_nodes() {
        let ok = |k| plan(View::Rig, k);
        assert_eq!(ok(Kind::Module), Ok(Outcome::Track));
        assert_eq!(ok(Kind::Instrument), Ok(Outcome::Track));
        assert_eq!(ok(Kind::Rig), Ok(Outcome::NestedRig));
        assert_eq!(ok(Kind::Character), Ok(Outcome::WorldNode));
        assert_eq!(ok(Kind::Item), Ok(Outcome::WorldNode));
        assert_eq!(ok(Kind::Model), Ok(Outcome::Body));
        assert_eq!(ok(Kind::Image), Ok(Outcome::Sketch));
        assert_eq!(ok(Kind::Audio), Ok(Outcome::Sampler));
        assert_eq!(ok(Kind::Video), Ok(Outcome::PictureSource));
        assert_eq!(ok(Kind::Scene), Err("scenes cannot be dropped on a rig: a show's row plays it, a rig plays modules and instruments".into()));
        assert_eq!(ok(Kind::Place), Err("places cannot be dropped on a rig: a show's row plays it, a rig plays modules and instruments".into()));
        assert_eq!(ok(Kind::File), Err("files cannot be dropped on a rig: a rig takes no plain file: drop it in a Show row as a reference".into()));
    }

    #[test]
    fn a_scene_takes_slots_layers_and_modules() {
        let ok = |k| plan(View::Scene, k);
        assert_eq!(ok(Kind::Character), Ok(Outcome::Slot));
        assert_eq!(ok(Kind::Item), Ok(Outcome::Slot));
        assert_eq!(ok(Kind::Image), Ok(Outcome::Layer));
        assert_eq!(ok(Kind::Module), Ok(Outcome::Modules));
        let refused: Vec<Kind> = table(View::Scene).into_iter().filter(|(_, r)| r.is_err()).map(|(k, _)| k).collect();
        assert_eq!(refused, [Kind::Toon, Kind::Scene, Kind::Show, Kind::Place, Kind::Instrument, Kind::Rig, Kind::Generator, Kind::Kanon, Kind::Person, Kind::Audio, Kind::Video, Kind::Model, Kind::Animation, Kind::File]);
        assert_eq!(ok(Kind::Audio), Err("audio cannot be dropped on a scene: its slots take characters and items, its layers pictures, the Modules of its camera and members modules".into()));
    }

    #[test]
    fn a_planned_outcome_not_built_yet_is_refused_in_its_words() {
        assert_eq!(lands(View::ShowRow, Kind::Place), Ok(Outcome::Piece));
        assert_eq!(lands(View::Rig, Kind::Module), Ok(Outcome::Track));
        assert_eq!(lands(View::ShowRow, Kind::Image), Err("images in a Show row become a reference node: not built yet".into()));
        assert_eq!(lands(View::Rig, Kind::Character), Err("characters on a rig become a world node: not built yet".into()));
        assert_eq!(lands(View::Rig, Kind::Model), Err("3d models on a rig become a body on a node: not built yet".into()));
        assert_eq!(lands(View::Rig, Kind::Scene), plan(View::Rig, Kind::Scene).map(|_| Outcome::Track), "a refusal stays the plan's");
        for view in [View::ShowRow, View::Rig, View::Scene] {
            for (k, r) in table(view) {
                assert_eq!(lands(view, k).is_ok(), r.as_ref().is_ok_and(|o| o.built()), "{view:?} {k:?}");
            }
        }
    }
}
