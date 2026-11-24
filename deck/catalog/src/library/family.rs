//! Asset families: the files that differ only by a version marker (`.2x`, `.480p`, `.900w`, `.poster`, `.128`) or by
//! their format are one family – one tile, its versions a stack behind it (agents/ASSETS.md "Grouping", "Variants are
//! suffixes"). A video's subtitles (`speak.en.vtt` beside `speak.mp4`) and the parts a document links to a model stand
//! with it in a frame; the Library makes no link of its own: a frame is the name contract or a link the caller read.
use super::{Kind, Preview};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// where a file came from
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    Uploaded,
    Imported,
    Generated,
}

impl Origin {
    pub const ALL: [Origin; 3] = [Origin::Uploaded, Origin::Imported, Origin::Generated];

    pub fn id(self) -> &'static str {
        match self {
            Origin::Uploaded => "uploaded",
            Origin::Imported => "imported",
            Origin::Generated => "generated",
        }
    }

    pub fn of_id(id: &str) -> Option<Origin> {
        Origin::ALL.into_iter().find(|o| o.id() == id)
    }
}

/// the audio levels the content reads (96 low, 128 medium, 320 high) and the rates a scan also accepts
const AUDIO_RATES: [&str; 7] = ["64", "96", "128", "160", "192", "256", "320"];
/// the subtitle formats a video's frame takes by name
const SUBTITLES: [&str; 2] = ["vtt", "srt"];

/// a file's kind by its extension
pub fn kind_of_ext(ext: &str) -> Kind {
    match ext.to_ascii_lowercase().as_str() {
        "webp" | "png" | "jpg" | "jpeg" | "gif" | "svg" | "avif" => Kind::Image,
        "mp4" | "webm" | "mov" | "mkv" | "m4v" => Kind::Video,
        "mp3" | "ogg" | "wav" | "flac" | "m4a" | "aac" | "opus" => Kind::Audio,
        "glb" | "gltf" => Kind::Model,
        _ => Kind::File,
    }
}

/// the registry's asset kind as the Library's
pub fn kind_of_asset(kind: &str) -> Kind {
    match kind {
        "image" => Kind::Image,
        "video" => Kind::Video,
        "audio" => Kind::Audio,
        "model" => Kind::Model,
        "animation" => Kind::Animation,
        _ => Kind::File,
    }
}

/// a file's family name and its version: `catpope/s1/c1/x.480p.mp4` → base `catpope/s1/c1/x`, label `480p`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionName {
    pub base: String,
    /// `source` (no marker), `2x` `3x`, `480p` …, `<n>w`, `poster`, `<n>k` (audio)
    pub label: String,
    pub ext: String,
    pub kind: Kind,
}

pub fn version_name(path: &str) -> VersionName {
    let slash = path.rfind('/').map_or(0, |i| i + 1);
    let (stem, ext) = match path.rfind('.') {
        Some(dot) if dot > slash => (&path[..dot], path[dot + 1..].to_ascii_lowercase()),
        _ => (path, String::new()),
    };
    let kind = kind_of_ext(&ext);
    if let Some(inner) = stem.rfind('.').filter(|&i| i > slash) {
        let marker = stem[inner + 1..].to_ascii_lowercase();
        let digits = |s: &str, unit: char| s.strip_suffix(unit).is_some_and(|n| (3..=4).contains(&n.len()) && n.bytes().all(|b| b.is_ascii_digit()));
        let found = if marker == "2x" || marker == "3x" || digits(&marker, 'p') || digits(&marker, 'w') || marker == "poster" {
            Some(marker)
        } else if kind == Kind::Audio && AUDIO_RATES.contains(&marker.as_str()) {
            Some(format!("{marker}k"))
        } else {
            None
        };
        if let Some(label) = found {
            return VersionName { base: stem[..inner].to_string(), label, ext, kind };
        }
    }
    VersionName { base: stem.to_string(), label: "source".into(), ext, kind }
}

/// the order versions are listed in (the stack, the details' table): the source, video levels down, widths down, the
/// poster, the densities, audio rates down
pub fn version_rank(label: &str) -> f64 {
    let num = |s: &str, unit: char| s.strip_suffix(unit).and_then(|n| n.parse::<f64>().ok());
    if label == "source" {
        0.0
    } else if let Some(p) = num(label, 'p') {
        100.0 - p / 100.0
    } else if let Some(w) = num(label, 'w') {
        200.0 - w / 100.0
    } else if label == "poster" {
        300.0
    } else if let Some(x) = num(label, 'x') {
        400.0 + x
    } else if let Some(k) = num(label, 'k') {
        500.0 - k / 100.0
    } else {
        900.0
    }
}

/// one file of a family
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Version {
    pub label: String,
    pub path: String,
    pub ext: String,
    pub kind: Kind,
    pub size: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration: Option<f64>,
    pub quality: Option<String>,
    pub origin: Origin,
    /// the registry asset the file belongs to
    pub asset: String,
}

/// one tile: a family of files
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Family {
    /// the CDN-root path without marker and extension
    pub key: String,
    /// the key's last part
    pub name: String,
    /// the folder the files are in (`catpope/s1/c1`)
    pub folder: String,
    pub title: String,
    /// the leading kind: a video's poster and stills are part of the video
    pub kind: Kind,
    /// the registry assets folded in (usually one)
    pub assets: Vec<String>,
    pub versions: Vec<Version>,
    /// labels a family of this kind and place is expected to have and doesn't (shown on demand only)
    pub missing: Vec<String>,
    /// references of its files (content + catalog, the registry's count)
    pub usages: u32,
    pub size: u64,
    pub duration: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub origin: Origin,
    pub tags: Vec<String>,
    pub updated: Option<String>,
    /// the registry's version number (the highest of its assets)
    pub version: u32,
    /// whether it carries a credits line; None = the answer does not say
    pub credits: Option<bool>,
    /// the deck's rule: a picture its smallest image, a model a still the stage renders, else its kind
    pub preview: Preview,
    /// the file Use hands on: the version a slot of this kind takes by default
    pub main: String,
    /// the characters it is made for (a file in `characters/<id>`): the deck's "related"
    pub made_for: Vec<String>,
}

pub(crate) const LEAD: [Kind; 5] = [Kind::Video, Kind::Model, Kind::Audio, Kind::Image, Kind::File];
const VIDEO_LEVELS: [(&str, u32); 3] = [("480p", 480), ("720p", 720), ("1080p", 1080)];

/// a chapter's folder (`<toon>/s<n>/c<n>`): its images are slices and key visuals, delivered as 1x / 2x / 3x
fn chapter_folder(folder: &str) -> bool {
    let parts: Vec<&str> = folder.rsplit('/').take(2).collect();
    let numbered = |s: &str, p: char| s.strip_prefix(p).is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
    parts.len() == 2 && numbered(parts[0], 'c') && numbered(parts[1], 's')
}

/// which versions a family should have (agents/ASSETS.md; RULES I3): video levels up to the source's short side and a
/// poster, the three audio levels, a chapter image's 2x / 3x
pub fn expected_versions(kind: Kind, folder: &str, versions: &[Version]) -> Vec<String> {
    match kind {
        Kind::Video => {
            let top = versions.iter().filter(|v| v.kind == Kind::Video).filter_map(|v| Some(v.width?.min(v.height?))).max().unwrap_or(u32::MAX);
            VIDEO_LEVELS.iter().filter(|(_, side)| *side <= top || *side == 480).map(|(l, _)| l.to_string()).chain(["poster".to_string()]).collect()
        }
        Kind::Audio => ["96k", "128k", "320k"].map(String::from).to_vec(),
        Kind::Image if chapter_folder(folder) || versions.iter().any(|v| v.label == "2x" || v.label == "3x") => vec!["2x".into(), "3x".into()],
        _ => Vec::new(),
    }
}

fn title_of(name: &str) -> String {
    name.split(['-', '_', ' ']).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ")
}

/// the version a slot of the kind takes: a video's 720p (the default level), an audio's 128k, an image's 1x, else the source
fn main_of(kind: Kind, versions: &[Version]) -> String {
    let files: Vec<(&str, &str, Kind, &str)> = versions.iter().map(|v| (v.label.as_str(), v.ext.as_str(), v.kind, v.path.as_str())).collect();
    main_path(kind, &files).unwrap_or_default().to_string()
}

/// `main_of` over (label, extension, kind, path) – the rendition rule's default file (`rendition::Media::of_files`)
pub(crate) fn main_path<'a>(kind: Kind, files: &[(&'a str, &'a str, Kind, &'a str)]) -> Option<&'a str> {
    let of_kind: Vec<&(&str, &str, Kind, &str)> = files.iter().filter(|f| f.2 == kind).collect();
    let by = |label: &str| of_kind.iter().find(|f| f.0 == label).map(|f| f.3);
    let first = || of_kind.first().map(|f| f.3);
    let picked = match kind {
        Kind::Video => by("720p").or_else(|| by("source")).or_else(first),
        Kind::Audio => by("128k").or_else(|| by("source")).or_else(first),
        Kind::Image => by("1x").or_else(|| of_kind.iter().find(|f| f.0 == "source" && f.1 == "webp").map(|f| f.3)).or_else(|| by("source")).or_else(first),
        _ => first(),
    };
    picked.or_else(|| files.first().map(|f| f.3))
}

/// The deck's preview rule for a family (`library::asset`): a picture or a video its smallest image, a model a still
/// the stage renders once and keeps under `asset:<path>`, anything else its kind.
fn preview_of(kind: Kind, versions: &[Version], main: &str) -> Preview {
    // unknown widths: the poster, the 1x, the WebP source, the source, then the densities
    let rank = |v: &Version| match (v.label.as_str(), v.ext.as_str()) {
        ("poster", _) => 0,
        ("1x", _) => 1,
        ("source", "webp") => 2,
        ("source", _) => 3,
        _ => 4,
    };
    let smallest = versions.iter().filter(|v| v.kind == Kind::Image).min_by_key(|v| (v.width.unwrap_or(u32::MAX), rank(v)));
    match (kind, smallest) {
        (Kind::Image | Kind::Video, Some(v)) => Preview::Picture { url: v.path.clone() },
        (Kind::Model, _) => Preview::Still { key: format!("asset:{main}") },
        _ => Preview::Kind,
    }
}

fn str_of(v: &Value) -> Option<String> {
    v.as_str().map(str::to_string)
}

fn u32_of(v: &Value) -> Option<u32> {
    v.as_u64().and_then(|n| u32::try_from(n).ok())
}

/// One file of `GET /api/assets`' answer (an asset's rendition, or the asset itself when it lists none).
fn version_of(asset: &Value, file: &Value, id: &str) -> Option<(String, Version)> {
    let path = file["path"].as_str()?;
    let name = version_name(path);
    // the API's kind wins for a file its extension can't tell (a .glb in `animations/` is an animation, #817)
    let kind = match kind_of_asset(asset["kind"].as_str().unwrap_or("")) {
        Kind::Animation => Kind::Animation,
        k if name.kind == Kind::File => k,
        _ => name.kind,
    };
    let origin = file["origin"].as_str().and_then(Origin::of_id).unwrap_or(Origin::Imported);
    let version = Version {
        label: name.label,
        path: path.to_string(),
        ext: name.ext,
        kind,
        size: file["size"].as_u64().unwrap_or(0),
        width: u32_of(&file["width"]),
        height: u32_of(&file["height"]),
        duration: file["duration"].as_f64(),
        quality: str_of(&file["quality"]),
        origin,
        asset: id.to_string(),
    };
    Some((name.base, version))
}

/// The registry's assets (`GET /api/assets`' `items`) as families: every file goes to the family of its name (an
/// asset's renditions are usually one family; a still or a poster filed as its own asset joins its video). By key.
pub fn families(assets: &[Value]) -> Vec<Family> {
    let mut by_key: BTreeMap<String, (Vec<Version>, Vec<&Value>)> = BTreeMap::new();
    for asset in assets {
        let Some(id) = asset["id"].as_str().or_else(|| asset["path"].as_str()) else { continue };
        let renditions = asset["renditions"].as_array().filter(|r| !r.is_empty());
        let files: Vec<&Value> = renditions.map_or_else(|| vec![asset], |r| r.iter().collect());
        for file in files {
            let Some((base, version)) = version_of(asset, file, id) else { continue };
            let entry = by_key.entry(base).or_default();
            if !entry.1.iter().any(|a| std::ptr::eq(*a, asset)) {
                entry.1.push(asset);
            }
            if !entry.0.iter().any(|v| v.path == version.path) {
                entry.0.push(version);
            }
        }
    }
    by_key.into_iter().map(|(key, (versions, members))| family(key, versions, &members)).collect()
}

fn family(key: String, mut versions: Vec<Version>, members: &[&Value]) -> Family {
    // two files with the same label (a JPEG master and its WebP twin): the WebP is the 1x
    let twins: Vec<String> = versions.iter().filter(|v| v.label == "source").map(|v| v.label.clone()).collect();
    if twins.len() > 1 {
        for v in versions.iter_mut().filter(|v| v.label == "source" && v.ext == "webp") {
            v.label = "1x".into();
        }
    }
    versions.sort_by(|a, b| version_rank(&a.label).total_cmp(&version_rank(&b.label)).then_with(|| a.path.cmp(&b.path)));
    let kind = LEAD.into_iter().find(|k| versions.iter().any(|v| v.kind == *k)).unwrap_or(Kind::File);
    let (folder, name) = key.rsplit_once('/').map_or((String::new(), key.clone()), |(f, n)| (f.to_string(), n.to_string()));
    let lead: Vec<&Version> = versions.iter().filter(|v| v.kind == kind).collect();
    let sized = lead.iter().copied().find(|v| v.width.is_some() && v.height.is_some()).or_else(|| versions.iter().find(|v| v.width.is_some() && v.height.is_some()));
    let duration = lead.iter().filter_map(|v| v.duration).filter(|d| *d > 0.0).reduce(f64::max);
    let origin = versions.iter().find(|v| v.label == "source").or(versions.first()).map_or(Origin::Imported, |v| v.origin);
    let titled = members.iter().filter_map(|a| a["title"].as_str().filter(|t| !t.is_empty() && Some(*t) != a["slug"].as_str())).next();
    let mut tags: Vec<String> = Vec::new();
    for t in members.iter().flat_map(|a| a["tags"].as_array().into_iter().flatten()).filter_map(Value::as_str) {
        if !tags.iter().any(|x| x == t) {
            tags.push(t.to_string());
        }
    }
    let credits_known: Vec<bool> = members.iter().filter(|a| a.get("credits").is_some()).map(|a| a["credits"].as_str().is_some_and(|c| !c.trim().is_empty())).collect();
    let main = main_of(kind, &versions);
    let made_for = character_of(&key).map(|c| vec![c.to_string()]).unwrap_or_default();
    let missing = {
        let expected = expected_versions(kind, &folder, &versions);
        expected.into_iter().filter(|l| !versions.iter().any(|v| &v.label == l)).collect()
    };
    Family {
        title: titled.map_or_else(|| title_of(&name), str::to_string),
        preview: preview_of(kind, &versions, &main),
        usages: members.iter().filter_map(|a| u32_of(&a["usages"])).sum(),
        size: versions.iter().map(|v| v.size).sum(),
        width: sized.and_then(|v| v.width),
        height: sized.and_then(|v| v.height),
        updated: members.iter().filter_map(|a| a["updated_at"].as_str()).max().map(str::to_string),
        version: members.iter().filter_map(|a| u32_of(&a["version"])).max().unwrap_or(1),
        credits: (!credits_known.is_empty()).then(|| credits_known.iter().all(|c| *c)),
        assets: members.iter().filter_map(|a| str_of(&a["id"])).collect(),
        key,
        name,
        folder,
        kind,
        versions,
        missing,
        duration,
        origin,
        tags,
        main,
        made_for,
    }
}

/// the character a file is made for: `characters/<id>/…` or a persona's folder `characters/<id>-<persona>/…`
pub fn character_of(key: &str) -> Option<&str> {
    let rest = key.strip_prefix("characters/")?;
    let folder = rest.split('/').next().filter(|_| rest.contains('/'))?;
    Some(folder.split('-').next().unwrap_or(folder)).filter(|c| !c.is_empty())
}

/// The levels shown single (the browser's toggle): every file a family of its own, keyed by its path.
pub fn singles(families: &[Family]) -> Vec<Family> {
    let mut out: Vec<Family> = Vec::new();
    for f in families {
        for v in &f.versions {
            let one = vec![v.clone()];
            let name = v.path.rsplit('/').next().unwrap_or(&v.path).to_string();
            out.push(Family {
                key: v.path.clone(),
                title: if f.versions.len() > 1 { format!("{} · {}", f.title, v.label) } else { f.title.clone() },
                name,
                kind: v.kind,
                preview: preview_of(v.kind, &one, &v.path),
                missing: Vec::new(),
                size: v.size,
                width: v.width,
                height: v.height,
                duration: v.duration,
                origin: v.origin,
                main: v.path.clone(),
                versions: one,
                ..f.clone()
            });
        }
    }
    out
}

/// what stands together in a frame: a video and its subtitles, a model and the parts its document links
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Frame {
    /// the main family's key
    pub main: String,
    pub kind: Kind,
    /// the other families' keys, in their order
    pub parts: Vec<String>,
}

/// The frames: a subtitle file named after a video (`speak.en.vtt` → `speak`) by the content contract, and every link
/// `(main file, part file)` the caller read from a document. Nothing else is linked.
pub fn frames(families: &[Family], links: &[(String, String)]) -> Vec<Frame> {
    let key_of = |path: &str| families.iter().find(|f| f.key == path || f.versions.iter().any(|v| v.path == path)).map(|f| f.key.as_str());
    let mut out: Vec<Frame> = Vec::new();
    let mut add = |main: &str, part: &str, kind: Kind| {
        if main == part {
            return;
        }
        match out.iter_mut().find(|fr| fr.main == main) {
            Some(fr) if !fr.parts.iter().any(|p| p == part) => fr.parts.push(part.to_string()),
            Some(_) => {}
            None => out.push(Frame { main: main.to_string(), kind, parts: vec![part.to_string()] }),
        }
    };
    for f in families {
        let subtitles = f.versions.iter().any(|v| SUBTITLES.contains(&v.ext.as_str()));
        let video = f.key.rsplit_once('.').filter(|(_, lang)| (2..=5).contains(&lang.len())).map(|(base, _)| base);
        if let (true, Some(base)) = (subtitles, video) {
            if families.iter().any(|v| v.key == base && v.kind == Kind::Video) {
                add(base, &f.key, Kind::Video);
            }
        }
    }
    for (main, part) in links {
        if let (Some(m), Some(p)) = (key_of(main), key_of(part)) {
            let kind = families.iter().find(|f| f.key == m).map_or(Kind::File, |f| f.kind);
            add(m, p, kind);
        }
    }
    out
}

