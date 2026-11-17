//! A scene's 2D layers in an editor (deck-catalog `Layer`): one added by its picture's media path, its depth and horizon
//! as knobs (unset = the matte, centred), its picture typed anew, taken out. Every change goes through `SceneEdit::change`.
use crate::scene_edit::{Line, Part, SceneEdit};
use crate::scene_video::VideoKnob;
use deck_catalog::layer::{Layer, MOST, NEAREST};

/// the deepest a depth knob turns to, metres; past it a layer is as good as the matte
pub const DEEPEST: f32 = 60.0;
/// a knob's range in front of the pivot, metres
const FRONT: f32 = -5.0;

/// what a layer is in a line: the matte, so far behind, so far in front
pub fn depth_words(depth: Option<f32>) -> String {
    match depth {
        None => "matte".into(),
        Some(d) if d < 0.0 => format!("{} m in front", -d),
        Some(d) => format!("{d} m behind"),
    }
}

impl SceneEdit {
    pub(crate) fn layer_lines(&self) -> Vec<Line> {
        let layers = &self.doc.stage.layers;
        deck_catalog::layer::order(layers)
            .into_iter()
            .map(|i| {
                let l = &layers[i];
                let name = l.image.rsplit('/').next().unwrap_or(&l.image).to_string();
                Line { depth: 1, label: name, detail: depth_words(l.depth), part: Part::Layer(i) }
            })
            .collect()
    }

    /// A layer as typed (`<media path> [depth] [horizon]`): its place in `stage.layers`. Err: the core's words.
    pub fn add_layer(&mut self, typed: &str) -> Result<usize, String> {
        let l = Layer::typed(typed)?;
        if self.doc.stage.layers.len() >= MOST {
            return Err(format!("a stage takes at most {MOST} layers"));
        }
        self.change(|d| {
            d.stage.layers.push(l);
            Ok(())
        })?;
        Ok(self.doc.stage.layers.len() - 1)
    }

    /// Layer `l` typed anew.
    pub fn set_layer(&mut self, l: usize, typed: &str) -> Result<(), String> {
        let next = Layer::typed(typed)?;
        self.change(|d| {
            *d.stage.layers.get_mut(l).ok_or("no such layer")? = next;
            Ok(())
        })
    }

    /// layer `l`'s knobs: its depth (unset = the matte) and its horizon (unset = centred)
    pub fn layer_knobs(&self, l: usize) -> Vec<VideoKnob> {
        let Some(layer) = self.doc.stage.layers.get(l) else {
            return Vec::new();
        };
        let knob = |param: &str, min: f32, max: f32, unit: &str, set: Option<f32>, own: f32| VideoKnob { param: param.into(), min, max, log: false, unit: unit.into(), value: set.unwrap_or(own), set: set.is_some() };
        vec![knob("depth", FRONT.max(NEAREST), DEEPEST, "m", layer.depth, DEEPEST), knob("horizon", 0.0, 1.0, "", layer.horizon, 0.5)]
    }

    /// Layer `l`'s knob `param` (`depth`, `horizon`) to `value`. Err: no such knob, a value outside its range.
    pub fn turn_layer(&mut self, l: usize, param: &str, value: f32) -> Result<(), String> {
        let k = self.layer_knobs(l).into_iter().find(|k| k.param == param).ok_or_else(|| format!("a layer has no knob {param}"))?;
        if !value.is_finite() || value < k.min || value > k.max {
            return Err(format!("a layer's {param} goes from {} to {}", k.min, k.max));
        }
        self.change(|d| {
            let layer = d.stage.layers.get_mut(l).ok_or("no such layer")?;
            match param {
                "depth" => layer.depth = Some(value),
                _ => layer.horizon = Some(value),
            }
            Ok(())
        })
    }

    /// layer `l`'s knob `param` unset: the matte again, or centred
    pub fn clear_layer(&mut self, l: usize, param: &str) {
        if let Some(layer) = self.doc.stage.layers.get_mut(l) {
            match param {
                "depth" => layer.depth = None,
                _ => layer.horizon = None,
            }
        }
    }

    /// layer `l` taken out of the stage
    pub fn remove_layer(&mut self, l: usize) -> Result<(), String> {
        self.change(|d| {
            (l < d.stage.layers.len()).then(|| d.stage.layers.remove(l)).map(|_| ()).ok_or_else(|| "no such layer".to_string())
        })
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

    #[test]
    fn layers_added_turned_and_taken_out_in_the_tree_far_to_near() {
        let mut e = solo();
        let fog = e.add_layer("layers/fog.png -1").unwrap();
        let sky = e.add_layer("layers/sky.png").unwrap();
        let hills = e.add_layer("layers/hills.png 12 0.6").unwrap();
        let lines: Vec<(String, String)> = e.tree().into_iter().filter(|l| matches!(l.part, Part::Layer(_))).map(|l| (l.label, l.detail)).collect();
        assert_eq!(lines, [("sky.png".into(), "matte".into()), ("hills.png".into(), "12 m behind".into()), ("fog.png".into(), "1 m in front".into())]);
        let k = e.layer_knobs(sky);
        assert_eq!((k[0].param.as_str(), k[0].set, k[0].value), ("depth", false, DEEPEST), "the matte: depth unset");
        e.turn_layer(sky, "depth", 30.0).unwrap();
        assert_eq!(e.doc.stage.layers[sky].depth, Some(30.0));
        assert!(e.turn_layer(sky, "depth", 99.0).unwrap_err().contains("goes from"));
        assert!(e.turn_layer(sky, "blur", 1.0).is_err(), "no blur: cheap depth only");
        e.clear_layer(sky, "depth");
        assert_eq!(e.doc.stage.layers[sky].depth, None);
        e.turn_layer(hills, "horizon", 0.3).unwrap();
        assert_eq!(e.doc.stage.layers[hills].horizon, Some(0.3));
        e.set_layer(fog, "layers/mist.png -2").unwrap();
        assert_eq!(e.doc.stage.layers[fog].image, "layers/mist.png");
        assert!(e.set_layer(fog, "").is_err());
        e.remove_layer(fog).unwrap();
        assert_eq!(e.doc.stage.layers.len(), 2);
        assert!(e.remove_layer(9).is_err());
        assert!(e.changed());
    }

    #[test]
    fn the_played_scene_carries_its_layers_and_their_pictures() {
        let mut e = solo();
        e.add_layer("layers/hills.png 8").unwrap();
        let scene = e.resolved(&core_rigs, &no_sets).unwrap().scene;
        assert_eq!(scene.stage.layers.len(), 1);
        assert_eq!(scene.stage.media(), ["layers/hills.png"]);
        let text = toml::to_string(&e.doc).unwrap();
        assert!(text.contains("[[stage.layers]]"), "{text}");
        assert_eq!(toml::from_str::<SceneDocument>(&text).unwrap().stage.layers, e.doc.stage.layers);
    }

    #[test]
    fn at_most_so_many() {
        let mut e = solo();
        for i in 0..MOST {
            e.add_layer(&format!("layers/{i}.png {i}")).unwrap();
        }
        assert!(e.add_layer("layers/one-more.png").unwrap_err().contains("at most"));
    }
}
