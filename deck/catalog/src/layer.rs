//! A scene's parallax layers: flat pictures standing in the 3D at their depths, between and around its objects, as in
//! film – a far matte, layers behind and around the 3D, strips at the frame's edges in front. Each faces the scene's
//! camera and is fitted to its part of that camera's view at its depth (deck-space `space::layers`), so the parallax is
//! the camera's own perspective. A layer without a picture is an empty place in the stack, waiting for one.
use serde::{Deserialize, Serialize};

/// the part of the view a layer covers
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Anchor {
    #[default]
    Full,
    Left,
    Right,
    Top,
    Bottom,
}

impl Anchor {
    pub const ALL: [Anchor; 5] = [Anchor::Full, Anchor::Left, Anchor::Right, Anchor::Top, Anchor::Bottom];

    pub fn word(self) -> &'static str {
        match self {
            Anchor::Full => "full",
            Anchor::Left => "left",
            Anchor::Right => "right",
            Anchor::Top => "top",
            Anchor::Bottom => "bottom",
        }
    }
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    /// the picture's media path (a PNG with transparency, or a video – `background::is_video`); none = an empty place
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// metres behind the camera's pivot; below 0 in front of it
    pub depth: f32,
    /// the part of the view it covers: all of it, or a strip at an edge
    #[serde(default, skip_serializing_if = "is_full")]
    pub anchor: Anchor,
}

fn is_full(a: &Anchor) -> bool {
    *a == Anchor::Full
}

/// the nearest a layer may stand, metres in front of the pivot
pub const NEAREST: f32 = -20.0;
/// the farthest, metres behind it
pub const FARTHEST: f32 = 150.0;
/// the most layers a stage takes
pub const MOST: usize = 12;
/// an edge strip's share of the view across it
pub const STRIP: f32 = 0.3;

impl Layer {
    pub fn empty(depth: f32, anchor: Anchor) -> Layer {
        Layer { image: None, depth, anchor }
    }

    /// what is wrong with it; None = well formed
    pub fn check(&self) -> Option<String> {
        if self.image.as_deref().is_some_and(|p| p.trim().is_empty()) {
            return Some("the layer's media path is empty".into());
        }
        (!self.depth.is_finite() || !(NEAREST..=FARTHEST).contains(&self.depth)).then(|| format!("a layer stands from {NEAREST} to {FARTHEST} m, not {}", self.depth))
    }
}

/// The default parallax stack, from the camera's view: a far matte, three layers behind and around the 3D, strips at
/// the left, the right and the bottom in front – all empty, for pictures to be dropped in.
pub fn default_stack() -> Vec<Layer> {
    let mut out: Vec<Layer> = [40.0, 12.0, 4.0, 1.0].into_iter().map(|d| Layer::empty(d, Anchor::Full)).collect();
    out.extend([Anchor::Left, Anchor::Right, Anchor::Bottom].map(|a| Layer::empty(-1.5, a)));
    out
}

/// the order they are drawn in from the camera's view: far to near, as written where they stand alike
pub fn order(layers: &[Layer]) -> Vec<usize> {
    let mut i: Vec<usize> = (0..layers.len()).collect();
    i.sort_by(|&a, &b| layers[b].depth.total_cmp(&layers[a].depth).then(a.cmp(&b)));
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_stack_far_to_near() {
        let s = default_stack();
        assert!(s.len() <= MOST && s.iter().all(|l| l.check().is_none() && l.image.is_none()));
        let o = order(&s);
        assert_eq!(s[o[0]].depth, 40.0, "the matte first");
        assert!(o.windows(2).all(|w| s[w[0]].depth >= s[w[1]].depth));
        assert_eq!(s.iter().filter(|l| l.depth < 0.0).map(|l| l.anchor).collect::<Vec<_>>(), [Anchor::Left, Anchor::Right, Anchor::Bottom], "edge strips in front");
    }

    #[test]
    fn checked_and_read() {
        assert!(Layer::empty(200.0, Anchor::Full).check().unwrap().contains("from -20 to 150"));
        assert!(Layer { image: Some(" ".into()), ..Layer::empty(1.0, Anchor::Top) }.check().is_some());
        let l: Layer = serde_json::from_str(r#"{"image":"layers/fern.png","depth":-1,"anchor":"left"}"#).unwrap();
        assert_eq!(l, Layer { image: Some("layers/fern.png".into()), depth: -1.0, anchor: Anchor::Left });
        assert_eq!(serde_json::to_string(&Layer::empty(3.0, Anchor::Full)).unwrap(), r#"{"depth":3.0}"#);
        assert!(serde_json::from_str::<Layer>(r#"{"depth":1,"blur":2}"#).is_err(), "unknown fields are refused");
    }
}
