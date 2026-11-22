//! A module's slot assets through the general-purpose loader (`Library::load`: deck-media's `asset::load` over the
//! library's disk cache and the CDN), decoded for the slot – the pick, the fallback and the decode are the core's. The
//! quality is the deck's (`library::quality`). Decoded assets stay in memory by file, so a scene built again does not
//! decode again.
use crate::library::{self, Library};
use deck_catalog::rendition::{Media, Quality};
use deck_catalog::AssetKind;
use deck_engine::AssetLoader;
use deck_media::asset;
use deck_module_host::AssetData;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// The loader the engine asks for module slot assets.
pub struct Assets {
    lib: Library,
    /// a fixed quality (tests); None: the deck's
    fixed: Option<Quality>,
    decoded: Mutex<HashMap<(String, u8), AssetData>>,
    /// where its log lines go (tests); None: the deck's log
    sink: Option<Sink>,
}

type Sink = Arc<dyn Fn(&str) + Send + Sync>;

impl Assets {
    pub fn new(lib: Library) -> Assets {
        Assets { lib, fixed: None, decoded: Mutex::new(HashMap::new()), sink: None }
    }

    /// … its log lines handed to `sink`
    pub fn logging_to(mut self, sink: impl Fn(&str) + Send + Sync + 'static) -> Assets {
        self.sink = Some(Arc::new(sink));
        self
    }

    /// … loading at `q` whatever the deck's quality
    pub fn at(lib: Library, q: Quality) -> Assets {
        Assets { fixed: Some(q), ..Assets::new(lib) }
    }
}

impl AssetLoader for Assets {
    fn load(&self, slot: usize, media: &Media, kind: AssetKind) -> Result<AssetData, String> {
        let q = self.fixed.unwrap_or_else(library::quality);
        let got = self.lib.load(None, media, q).map_err(|e| format!("{e:#}"))?;
        if got.fallback {
            self.log(&format!("asset {}: {} could not be had at {} – {} plays", media.src, media.pick(q), q.name(), got.path));
        }
        let key = (got.path.clone(), kind as u8);
        if let Some(a) = self.decoded.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
            return Ok(AssetData { slot, ..a.clone() });
        }
        let data = match kind {
            AssetKind::Pcm => {
                let p = asset::planar(got.bytes).map_err(|e| format!("{}: {}", got.path, e.0))?;
                let chans: Vec<&[f32]> = p.channels.iter().map(Vec::as_slice).collect();
                AssetData::pcm(slot, &chans, p.rate)
            }
            AssetKind::Bytes => AssetData::bytes(slot, &got.bytes),
            AssetKind::Image => return Err("a picture slot is not filled on the deck yet".into()),
        };
        self.decoded.lock().unwrap_or_else(|e| e.into_inner()).insert(key, data.clone());
        Ok(data)
    }

    fn log(&self, line: &str) {
        match &self.sink {
            Some(s) => s(line),
            None => tracing::warn!("{line}"),
        }
    }
}

static LOADER: Mutex<Option<Arc<Assets>>> = Mutex::new(None);

/// the process's loader over `lib` (the scene's library: its CDN and cache)
pub fn install(lib: Library) {
    let mut l = LOADER.lock().unwrap_or_else(|e| e.into_inner());
    if l.as_ref().is_none_or(|a| a.lib.cdn != lib.cdn || a.lib.cache != lib.cache) {
        *l = Some(Arc::new(Assets::new(lib)));
    }
}

/// the installed loader for the engine's seams (None: module slots play their fallback)
pub fn loader() -> Option<Arc<dyn AssetLoader>> {
    LOADER.lock().unwrap_or_else(|e| e.into_inner()).clone().map(|a| a as Arc<dyn AssetLoader>)
}
