//! A module node's assets: the node's `assets` table names a medium per asset slot the module declares – a file
//! (`assets = { ir = "scutoons/irs/plate.wav" }`) or the content's `{ src, variants }` (`ir = { src = "….128.mp3",
//! variants = { low = "….96.mp3", high = "….320.mp3" } }`), paths CDN-root as the Library hands them. A `model` slot
//! holds a set: a list of them (`puff = ["scutoons/3d/a.glb", "scutoons/3d/b.glb"]`; one file is a set of one, an empty
//! list none) – each drawn shape picks one (space `drawn`). The check is here once: the slot is declared, every file is
//! of the kind the slot holds (a `pcm` slot a sound, an `image` slot a picture, a `model` slot a .glb), the paths stay
//! inside the CDN. Which file plays is the one pick rule's (deck-catalog `rendition`).
use deck_catalog::library::{family::kind_of_ext, Kind as FileKind};
use deck_catalog::rendition::Media;
use deck_catalog::AssetKind;

/// the node param that names the slot assets
pub const PARAM: &str = "assets";

/// One slot's asset as the node names it (a `model` slot's set: one per file, in the set's order).
#[derive(Debug, Clone, PartialEq)]
pub struct SlotAsset {
    /// the slot's index in the module's descriptor
    pub slot: usize,
    pub name: String,
    pub kind: AssetKind,
    pub media: Media,
}

/// The slot assets `params` names against the module's `slots` (name, kind); Err: every problem, in words.
pub fn of(params: &toml::Table, slots: &[(&str, AssetKind)]) -> Result<Vec<SlotAsset>, Vec<String>> {
    let Some(value) = params.get(PARAM) else { return Ok(Vec::new()) };
    let Some(table) = value.as_table() else { return Err(vec![format!("`{PARAM}` is a table: slot = a file or {{ src, variants }}")]) };
    let mut out = Vec::new();
    let mut problems = Vec::new();
    for (name, v) in table {
        let Some(slot) = slots.iter().position(|(n, _)| n == name) else {
            let have: Vec<&str> = slots.iter().map(|(n, _)| *n).collect();
            problems.push(match have.is_empty() {
                true => format!("asset '{name}': the module has no asset slots"),
                false => format!("asset '{name}': the module has no such slot (it has {})", have.join(", ")),
            });
            continue;
        };
        let kind = slots[slot].1;
        let set: Vec<&toml::Value> = match v {
            toml::Value::Array(list) if kind == AssetKind::Model => list.iter().collect(),
            toml::Value::Array(_) => {
                problems.push(format!("asset '{name}': only a model slot holds a list"));
                continue;
            }
            one => vec![one],
        };
        let before = problems.len();
        let mut named = Vec::with_capacity(set.len());
        for v in set {
            match media_of(v) {
                Ok(media) => {
                    for path in std::iter::once(&media.src).chain(media.variants.values()) {
                        if let Some(e) = path_problem(path, kind) {
                            problems.push(format!("asset '{name}': {e}"));
                        }
                    }
                    named.push(SlotAsset { slot, name: name.clone(), kind, media });
                }
                Err(e) => problems.push(format!("asset '{name}': {e}")),
            }
        }
        if problems.len() == before {
            out.extend(named);
        }
    }
    if problems.is_empty() {
        Ok(out)
    } else {
        Err(problems)
    }
}

fn media_of(v: &toml::Value) -> Result<Media, String> {
    const SHAPE: &str = "a file (a CDN path) or { src, variants = { low, medium, high } }";
    match v {
        toml::Value::String(s) => Ok(Media::new(s)),
        toml::Value::Table(t) => {
            if let Some(k) = t.keys().find(|k| !matches!(k.as_str(), "src" | "variants")) {
                return Err(format!("'{k}' is not part of {SHAPE}"));
            }
            let Some(src) = t.get("src").and_then(toml::Value::as_str) else { return Err(format!("no `src`: {SHAPE}")) };
            let Some(variants) = t.get("variants").map(|v| v.as_table().ok_or_else(|| format!("`variants` is a table: {SHAPE}"))).transpose()? else {
                return Ok(Media::new(src));
            };
            let mut pairs = Vec::new();
            for (level, path) in variants {
                if deck_catalog::rendition::Quality::parse(level).is_none() {
                    return Err(format!("'{level}' is no quality level (low, medium, high)"));
                }
                pairs.push((level.as_str(), path.as_str().ok_or_else(|| format!("variant '{level}' is a path"))?));
            }
            Ok(Media::of_variants(src, pairs))
        }
        _ => Err(SHAPE.to_string()),
    }
}

fn path_problem(path: &str, kind: AssetKind) -> Option<String> {
    let segments: Vec<&str> = path.split('/').collect();
    if path.is_empty() || segments.len() < 2 || segments.iter().any(|s| s.is_empty() || *s == "." || *s == ".." || s.contains(['\\', ':'])) {
        return Some(format!("'{path}' is no CDN path (`<folder>/…/<file>`)"));
    }
    let ext = path.rsplit_once('.').map_or("", |(_, e)| e);
    let fits = match kind {
        AssetKind::Pcm => kind_of_ext(ext) == FileKind::Audio,
        AssetKind::Image => kind_of_ext(ext) == FileKind::Image,
        AssetKind::Bytes => true,
        AssetKind::Model => ext.eq_ignore_ascii_case("glb"),
    };
    let what = match kind {
        AssetKind::Pcm => "sound",
        AssetKind::Model => ".glb",
        _ => "picture",
    };
    (!fits).then(|| format!("'{path}' is no {what} file"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use deck_catalog::rendition::Quality;

    const SLOTS: [(&str, AssetKind); 3] = [("ir", AssetKind::Pcm), ("look", AssetKind::Image), ("puff", AssetKind::Model)];

    fn params(toml: &str) -> toml::Table {
        toml.parse().unwrap()
    }

    #[test]
    fn a_file_or_src_and_variants_per_slot() {
        let got = of(&params(r#"assets = { ir = "scutoons/irs/plate.wav" }"#), &SLOTS).unwrap();
        assert_eq!(got, [SlotAsset { slot: 0, name: "ir".into(), kind: AssetKind::Pcm, media: Media::new("scutoons/irs/plate.wav") }]);
        let got = of(&params("[assets.look]\nsrc = \"c/a.webp\"\nvariants = { high = \"c/a.3x.webp\" }\n"), &SLOTS).unwrap();
        assert_eq!((got[0].slot, got[0].media.pick(Quality::High)), (1, "c/a.3x.webp"));
        assert_eq!(of(&params("x = 1"), &SLOTS).unwrap(), []);
    }

    #[test]
    fn the_check_names_slot_kind_and_path() {
        let bad = |t: &str| of(&params(t), &SLOTS).unwrap_err().join(" | ");
        assert_eq!(bad(r#"assets = { room = "a/b.wav" }"#), "asset 'room': the module has no such slot (it has ir, look, puff)");
        assert_eq!(bad(r#"assets = { puff = "a/b.gltf" }"#), "asset 'puff': 'a/b.gltf' is no .glb file");
        assert_eq!(of(&params(r#"assets = { puff = "scutoons/3d/cloud-puff.GLB" }"#), &SLOTS).unwrap()[0].kind, AssetKind::Model);
        assert_eq!(bad(r#"assets = { ir = "a/b.webp" }"#), "asset 'ir': 'a/b.webp' is no sound file");
        assert_eq!(bad(r#"assets = { look = "a/b.wav" }"#), "asset 'look': 'a/b.wav' is no picture file");
        assert_eq!(bad(r#"assets = { ir = "../etc/x.wav" }"#), "asset 'ir': '../etc/x.wav' is no CDN path (`<folder>/…/<file>`)");
        assert_eq!(bad(r#"assets = { ir = { src = "a/b.wav", variants = { ultra = "a/c.wav" } } }"#), "asset 'ir': 'ultra' is no quality level (low, medium, high)");
        assert_eq!(bad(r#"assets = { ir = { path = "a/b.wav" } }"#), "asset 'ir': 'path' is not part of a file (a CDN path) or { src, variants = { low, medium, high } }");
        assert_eq!(bad("assets = 3"), "`assets` is a table: slot = a file or { src, variants }");
        assert_eq!(of(&params(r#"assets = { ir = "a/b.wav" }"#), &[]).unwrap_err(), ["asset 'ir': the module has no asset slots"]);
        assert_eq!(bad(r#"assets = { ir = ["a/b.wav"] }"#), "asset 'ir': only a model slot holds a list");
        assert_eq!(bad(r#"assets = { puff = ["a/b.glb", "a/c.png"] }"#), "asset 'puff': 'a/c.png' is no .glb file");
    }

    /// a model slot holds a set, in its order; one file is a set of one, an empty list none
    #[test]
    fn a_model_slot_holds_a_set() {
        let got = of(&params(r#"assets = { puff = ["scutoons/3d/a.glb", { src = "scutoons/3d/b.glb" }] }"#), &SLOTS).unwrap();
        let files: Vec<(usize, &str)> = got.iter().map(|a| (a.slot, a.media.src.as_str())).collect();
        assert_eq!(files, [(2, "scutoons/3d/a.glb"), (2, "scutoons/3d/b.glb")]);
        assert_eq!(of(&params(r#"assets = { puff = "scutoons/3d/a.glb" }"#), &SLOTS).unwrap().len(), 1);
        assert_eq!(of(&params("assets = { puff = [] }"), &SLOTS).unwrap(), []);
    }
}
