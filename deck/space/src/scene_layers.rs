//! A scene's parallax layers in an editor (deck-catalog `Layer`): the default stack added at once or a layer alone,
//! a picture filled into one (a library drop), its depth as a knob, its anchor, taken out. Every change goes through
//! `SceneEdit::change`; where a layer stands in the 3D is deck-space `space::layers`.
use crate::scene_edit::{Line, Part, SceneEdit};
use crate::scene_video::VideoKnob;
use deck_catalog::layer::{default_stack, order, Anchor, Layer, MOST};

/// the depth knob's range, metres behind the pivot
pub const NEAREST_KNOB: f32 = -5.0;
pub const DEEPEST_KNOB: f32 = 60.0;

/// where a layer stands, in words
pub fn depth_words(depth: f32) -> String {
    if depth < 0.0 {
        format!("{} m in front", -depth)
    } else {
        format!("{depth} m behind")
    }
}

/// a layer's name in the tree: its picture's file, else what place it is
pub fn layer_name(l: &Layer) -> String {
    match l.image.as_deref() {
        Some(p) => p.rsplit('/').next().unwrap_or(p).to_string(),
        None => format!("empty {}", l.anchor.word()),
    }
}

impl SceneEdit {
    pub(crate) fn layer_lines(&self) -> Vec<Line> {
        let layers = &self.doc.stage.layers;
        order(layers)
            .into_iter()
            .map(|i| {
                let l = &layers[i];
                let edge = if l.anchor == Anchor::Full { String::new() } else { format!(" · {}", l.anchor.word()) };
                Line { depth: 1, label: layer_name(l), detail: format!("{}{edge}", depth_words(l.depth)), part: Part::Layer(i) }
            })
            .collect()
    }

    fn room(&self, n: usize) -> Result<(), String> {
        if self.doc.stage.layers.len() + n > MOST {
            return Err(format!("a stage takes at most {MOST} layers"));
        }
        Ok(())
    }

    /// The default parallax stack (a far matte, three layers behind and around the 3D, edge strips in front), empty:
    /// how many places it added.
    pub fn add_stack(&mut self) -> Result<usize, String> {
        let stack = default_stack();
        self.room(stack.len())?;
        let n = stack.len();
        self.change(|d| {
            d.stage.layers.extend(stack);
            Ok(())
        })?;
        Ok(n)
    }

    /// An empty layer `depth` metres behind the pivot covering `anchor`'s part: its place in `stage.layers`.
    pub fn add_layer(&mut self, depth: f32, anchor: Anchor) -> Result<usize, String> {
        self.room(1)?;
        self.change(|d| {
            d.stage.layers.push(Layer::empty(depth, anchor));
            Ok(())
        })?;
        Ok(self.doc.stage.layers.len() - 1)
    }

    /// Layer `l`'s picture (a media path), or none: an empty place again.
    pub fn fill_layer(&mut self, l: usize, picture: Option<&str>) -> Result<(), String> {
        self.change(|d| {
            d.stage.layers.get_mut(l).ok_or("no such layer")?.image = picture.map(String::from);
            Ok(())
        })
    }

    /// the empty place a dropped picture goes to when it is let go on none: the farthest one
    pub fn empty_layer(&self) -> Option<usize> {
        order(&self.doc.stage.layers).into_iter().find(|&i| self.doc.stage.layers[i].image.is_none())
    }

    /// Layer `l` covers `anchor`'s part of the view.
    pub fn anchor_layer(&mut self, l: usize, anchor: Anchor) -> Result<(), String> {
        self.change(|d| {
            d.stage.layers.get_mut(l).ok_or("no such layer")?.anchor = anchor;
            Ok(())
        })
    }

    /// layer `l`'s knobs: its depth
    pub fn layer_knobs(&self, l: usize) -> Vec<VideoKnob> {
        let Some(layer) = self.doc.stage.layers.get(l) else {
            return Vec::new();
        };
        vec![VideoKnob { param: "depth".into(), min: NEAREST_KNOB, max: DEEPEST_KNOB, log: false, unit: "m".into(), value: layer.depth, set: false }]
    }

    /// Layer `l` moved to `depth` metres behind the pivot (it is fitted to the view there anew). Err: outside the knob's
    /// range.
    pub fn move_layer(&mut self, l: usize, depth: f32) -> Result<(), String> {
        if !depth.is_finite() || !(NEAREST_KNOB..=DEEPEST_KNOB).contains(&depth) {
            return Err(format!("a layer's depth goes from {NEAREST_KNOB} to {DEEPEST_KNOB} m"));
        }
        self.change(|d| {
            d.stage.layers.get_mut(l).ok_or("no such layer")?.depth = depth;
            Ok(())
        })
    }

    /// layer `l` taken out of the stage
    pub fn remove_layer(&mut self, l: usize) -> Result<(), String> {
        self.change(|d| (l < d.stage.layers.len()).then(|| d.stage.layers.remove(l)).map(|_| ()).ok_or_else(|| "no such layer".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene_document::SceneDocument;
    use crate::scene_resolve::{core_rigs, no_sets};

    fn solo() -> SceneEdit {
        let mut d = SceneDocument::new("vista", "Vista", vec!["catpope".into()]);
        d.recast(&["character:catpope".into()]);
        SceneEdit::open(d, None, &core_rigs, &no_sets).expect("it plays").0
    }

    fn lines(e: &SceneEdit) -> Vec<(String, String)> {
        e.tree().into_iter().filter(|l| matches!(l.part, Part::Layer(_))).map(|l| (l.label, l.detail)).collect()
    }

    #[test]
    fn the_default_stack_filled_moved_and_taken_out() {
        let mut e = solo();
        assert_eq!(e.add_stack(), Ok(7));
        let l = lines(&e);
        assert_eq!(l.first(), Some(&("empty full".to_string(), "40 m behind".to_string())), "the matte first");
        assert_eq!(l.last(), Some(&("empty bottom".to_string(), "1.5 m in front · bottom".to_string())));
        let matte = e.empty_layer().unwrap();
        assert_eq!(e.doc.stage.layers[matte].depth, 40.0, "a drop on no layer fills the farthest empty one");
        e.fill_layer(matte, Some("layers/sky.png")).unwrap();
        assert_eq!(lines(&e)[0].0, "sky.png");
        assert_eq!(e.doc.stage.layers[e.empty_layer().unwrap()].depth, 12.0);
        e.move_layer(matte, 30.0).unwrap();
        assert!(e.move_layer(matte, 99.0).unwrap_err().contains("goes from"));
        e.anchor_layer(matte, Anchor::Top).unwrap();
        assert_eq!(e.doc.stage.layers[matte].anchor, Anchor::Top);
        e.remove_layer(matte).unwrap();
        assert_eq!(e.doc.stage.layers.len(), 6);
        assert!(e.add_stack().unwrap_err().contains("at most"), "a second stack would pass the most");
        assert_eq!(e.add_layer(2.0, Anchor::Right), Ok(6));
    }

    #[test]
    fn the_played_scene_carries_its_layers_and_their_pictures() {
        let mut e = solo();
        let l = e.add_layer(8.0, Anchor::Full).unwrap();
        e.fill_layer(l, Some("layers/hills.png")).unwrap();
        e.add_layer(-1.0, Anchor::Left).unwrap();
        let scene = e.resolved(&core_rigs, &no_sets).unwrap().scene;
        assert_eq!(scene.stage.layers.len(), 2);
        assert_eq!(scene.stage.media(), ["layers/hills.png"], "an empty place asks for nothing");
        let text = toml::to_string(&e.doc).unwrap();
        assert!(text.contains("[[stage.layers]]"), "{text}");
        assert_eq!(toml::from_str::<SceneDocument>(&text).unwrap().stage.layers, e.doc.stage.layers);
    }
}
