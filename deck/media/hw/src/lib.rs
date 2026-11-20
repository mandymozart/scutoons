//! Hardware decode-only adapters for deck-media: each one implements `DecodeBackend` for one platform service and
//! decides nothing – what plays, when, which backend and the fallback stay in deck-media (`backend::Auto`).
//! - Windows: Media Foundation's decoder with the GPU's D3D11 video decoder (DXVA), NV12 pictures copied back (`mf`).
//! - Linux x86_64: VA-API through cros-codecs' stateless decoders, NV12 pictures read back (`vaapi`).
//! - The browser's WebCodecs adapter lives in space-web; other platforms play software.

#[cfg(windows)]
pub mod mf;
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub mod vaapi;

/// This platform's hardware decoders, offered to deck-media (`backend::install`) – once, by the host at start. A
/// platform without an adapter installs nothing.
pub fn install() {
    #[cfg(windows)]
    deck_media::backend::install(Box::new(Windows));
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    deck_media::backend::install(Box::new(Linux));
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

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
struct Linux;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl deck_media::Hardware for Linux {
    fn codecs(&self) -> Vec<deck_media::Codec> {
        let c = vaapi::codecs();
        if !c.is_empty() {
            tracing::info!("video: hardware decoding (VA-API): {c:?}");
        }
        c
    }

    fn backend(&self) -> Box<dyn deck_media::DecodeBackend> {
        Box::new(vaapi::Vaapi::new())
    }
}
