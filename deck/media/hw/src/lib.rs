//! Hardware decode-only adapters for deck-media: each one implements `DecodeBackend` for one platform service and
//! decides nothing – what plays, when, which backend and the fallback stay in deck-media (`backend::Auto`).
//! - Windows: Media Foundation's decoder with the GPU's D3D11 video decoder (DXVA), NV12 pictures copied back (`mf`).
//! - Elsewhere none yet: Linux (VA-API) and the browser (WebCodecs) follow (TODO(#327)); their decks play software.

#[cfg(windows)]
pub mod mf;

/// This platform's hardware decoders, offered to deck-media (`backend::install`) – once, by the host at start. A
/// platform without an adapter installs nothing.
pub fn install() {
    #[cfg(windows)]
    deck_media::backend::install(Box::new(Windows));
}

#[cfg(windows)]
struct Windows;

#[cfg(windows)]
impl deck_media::Hardware for Windows {
    fn codecs(&self) -> Vec<deck_media::Codec> {
        let c = mf::codecs();
        if !c.is_empty() {
            tracing::info!("video: hardware decoding (Media Foundation, DXVA): {c:?}");
        }
        c
    }

    fn backend(&self) -> Box<dyn deck_media::DecodeBackend> {
        Box::new(mf::MediaFoundation::new())
    }
}
