//! Hardware decode-only adapters for deck-media: each one implements `DecodeBackend` for one platform service and
//! decides nothing – what plays, when, and the fallback stay in deck-media.
//! - Windows: Media Foundation's decoder with the GPU's D3D11 video decoder (DXVA), NV12 pictures copied back (`mf`).

#[cfg(windows)]
pub mod mf;
