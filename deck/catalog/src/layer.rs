//! A scene's 2D layers around its 3D, as in film: a matte (infinitely far), parallax layers behind the 3D and in front of
//! it. Each is a picture with transparency, cover-fit and repeated mirrored to the sides, moving against the 3D by its
//! depth as the camera turns or pans (deck-core `parallax`). Depth is cheap: flat textured quads, nothing more.
use serde::{Deserialize, Serialize};

use crate::background::share;

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    /// the picture's media path
    pub image: String,
    /// metres behind the camera's pivot; below 0 in front of it (drawn over the 3D); none = the matte, infinitely far
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<f32>,
    /// the picture's horizon, a share of its height from the top (0–1); none = centred
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizon: Option<f32>,
}

/// the nearest a front layer may come, metres in front of the pivot
pub const NEAREST: f32 = -20.0;
/// the most layers a stage takes
pub const MOST: usize = 12;

impl Layer {
    /// over the 3D
    pub fn front(&self) -> bool {
        self.depth.is_some_and(|d| d < 0.0)
    }

    /// What a person typed: `<media path> [depth] [horizon]`; a depth of `matte` (or none) is infinitely far.
    pub fn typed(text: &str) -> Result<Layer, String> {
        let mut words = text.split_whitespace();
        let image = words.next().ok_or("a layer is a picture's media path, then its depth in metres (below 0 in front)")?.to_string();
        let number = |w: &str, what: &str| w.parse::<f32>().map_err(|_| format!("'{w}' is no {what}"));
        let depth = match words.next() {
            None | Some("matte") => None,
            Some(w) => Some(number(w, "depth in metres")?),
        };
        let horizon = words.next().map(|w| number(w, "horizon (0–1)")).transpose()?;
        if let Some(w) = words.next() {
            return Err(format!("'{w}': a layer is a path, a depth and a horizon"));
        }
        let l = Layer { image, depth, horizon };
        match l.check() {
            Some(e) => Err(e),
            None => Ok(l),
        }
    }

    /// as a person reads it (and types it back)
    pub fn text(&self) -> String {
        let depth = self.depth.map_or("matte".to_string(), |d| format!("{d}"));
        match self.horizon {
            Some(h) => format!("{} {depth} {h}", self.image),
            None if self.depth.is_some() => format!("{} {depth}", self.image),
            None => self.image.clone(),
        }
    }

    /// what is wrong with it; None = well formed
    pub fn check(&self) -> Option<String> {
        if self.image.trim().is_empty() {
            return Some("the layer's media path is empty".into());
        }
        if let Some(d) = self.depth {
            if !d.is_finite() || d < NEAREST {
                return Some(format!("a layer's depth is at least {NEAREST} m, not {d}"));
            }
        }
        self.horizon.and_then(|h| share("horizon", h))
    }
}

/// The order they are drawn in: far to near – the matte first, then by depth; a front layer after the 3D.
pub fn order(layers: &[Layer]) -> Vec<usize> {
    let mut i: Vec<usize> = (0..layers.len()).collect();
    i.sort_by(|&a, &b| layers[b].depth.unwrap_or(f32::INFINITY).total_cmp(&layers[a].depth.unwrap_or(f32::INFINITY)).then(a.cmp(&b)));
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(image: &str, depth: Option<f32>) -> Layer {
        Layer { image: image.into(), depth, horizon: None }
    }

    #[test]
    fn far_to_near_the_matte_first() {
        let layers = [at("hills", Some(8.0)), at("fog", Some(-1.0)), at("sky", None), at("trees", Some(2.0)), at("mist", Some(8.0))];
        assert_eq!(order(&layers), [2, 0, 4, 3, 1], "sky, hills and mist (as written), trees, then the fog in front");
        assert!(layers[1].front() && !layers[0].front() && !layers[2].front());
    }

    #[test]
    fn typed_and_read_back() {
        assert_eq!(Layer::typed("layers/sky.png").unwrap(), at("layers/sky.png", None));
        assert_eq!(Layer::typed("layers/hills.png 12").unwrap(), at("layers/hills.png", Some(12.0)));
        let fog = Layer::typed("layers/fog.png -1.5 0.4").unwrap();
        assert_eq!(fog, Layer { horizon: Some(0.4), ..at("layers/fog.png", Some(-1.5)) });
        for l in [at("a.png", None), at("a.png", Some(3.0)), fog.clone()] {
            assert_eq!(Layer::typed(&l.text()).unwrap(), l, "{}", l.text());
        }
        assert_eq!(Layer::typed("a.png matte 0.5").unwrap().horizon, Some(0.5));
        assert!(Layer::typed("").is_err());
        assert!(Layer::typed("a.png far").unwrap_err().contains("no depth"));
        assert!(Layer::typed("a.png -50").unwrap_err().contains("at least"));
        assert!(Layer::typed("a.png 2 1.5").unwrap_err().contains("horizon goes from 0 to 1"));
        assert!(Layer::typed("a.png 2 0.5 x").is_err());
        assert!(serde_json::from_str::<Layer>(r#"{"image":"a.png","blur":2}"#).is_err(), "unknown fields are refused");
    }
}
