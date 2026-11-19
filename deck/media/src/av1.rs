//! AV1 through rav1d (the Rust port of dav1d) – its C-shaped API wrapped once here: one decoding thread (the caller's,
//! no worker threads, so wasm without threads decodes too), no frame delay, each packet's id echoed by the picture.
use crate::yuv::Matrix;
use crate::{err, Error, Format, Picture};
use rav1d::include::dav1d::data::Dav1dData;
use rav1d::include::dav1d::dav1d::{Dav1dContext, Dav1dSettings};
use rav1d::include::dav1d::headers::{DAV1D_MC_BT470BG, DAV1D_MC_BT601, DAV1D_PIXEL_LAYOUT_I420};
use rav1d::include::dav1d::picture::Dav1dPicture;
use rav1d::src::lib as d;
use std::ptr::NonNull;

const EAGAIN: i32 = -libc::EAGAIN;

pub struct Av1 {
    ctx: Option<Dav1dContext>,
}

// SAFETY: a rav1d context is used from one thread at a time (`&mut self`); with one thread rav1d starts no workers
unsafe impl Send for Av1 {}

impl Av1 {
    pub fn new() -> Result<Av1, Error> {
        let mut s = std::mem::MaybeUninit::<Dav1dSettings>::uninit();
        let mut ctx: Option<Dav1dContext> = None;
        // SAFETY: the settings are written by `dav1d_default_settings` before they are read; `ctx` is a valid out slot
        let r = unsafe {
            d::dav1d_default_settings(NonNull::new(s.as_mut_ptr()).expect("a stack slot"));
            let mut s = s.assume_init();
            s.n_threads = 1;
            s.max_frame_delay = 1;
            d::dav1d_open(Some(NonNull::from(&mut ctx)), Some(NonNull::from(&mut s)))
        };
        if r.0 != 0 || ctx.is_none() {
            return err(format!("AV1: rav1d did not open ({})", r.0));
        }
        Ok(Av1 { ctx })
    }

    /// one temporal unit; `id` comes back with its picture. Pictures that are ready go to `out`.
    pub fn send(&mut self, packet: &[u8], id: i64, out: &mut Vec<(i64, Picture)>) -> Result<(), Error> {
        let mut data = Dav1dData::default();
        // SAFETY: `dav1d_data_create` hands out a buffer of `packet.len()` bytes owned by `data`
        unsafe {
            let buf = d::dav1d_data_create(Some(NonNull::from(&mut data)), packet.len());
            if buf.is_null() {
                return err("AV1: no buffer for a packet");
            }
            std::ptr::copy_nonoverlapping(packet.as_ptr(), buf, packet.len());
        }
        data.m.timestamp = id;
        let mut stuck = 0;
        while data.sz > 0 {
            // SAFETY: `ctx` is open, `data` is a valid Dav1dData
            let r = unsafe { d::dav1d_send_data(self.ctx, Some(NonNull::from(&mut data))) };
            let had = out.len();
            if r.0 == 0 || r.0 == EAGAIN {
                self.drain(out)?;
            }
            stuck = if r.0 == EAGAIN && out.len() == had { stuck + 1 } else { 0 };
            if (r.0 != 0 && r.0 != EAGAIN) || stuck > 2 {
                // SAFETY: `data` still owns its buffer
                unsafe { d::dav1d_data_unref(Some(NonNull::from(&mut data))) };
                return err(format!("AV1: a broken packet ({})", r.0));
            }
        }
        Ok(())
    }

    /// every picture that is ready
    pub fn drain(&mut self, out: &mut Vec<(i64, Picture)>) -> Result<(), Error> {
        loop {
            let mut pic = Dav1dPicture::default();
            // SAFETY: `ctx` is open, `pic` a valid out slot
            let r = unsafe { d::dav1d_get_picture(self.ctx, Some(NonNull::from(&mut pic))) };
            if r.0 == EAGAIN {
                return Ok(());
            }
            if r.0 != 0 {
                return err(format!("AV1: decoding failed ({})", r.0));
            }
            let (id, got) = (pic.m.timestamp, planes(&pic));
            // SAFETY: `pic` came from `dav1d_get_picture`
            unsafe { d::dav1d_picture_unref(Some(NonNull::from(&mut pic))) };
            out.push((id, got?));
        }
    }

    /// forget every packet (a seek)
    pub fn flush(&mut self) {
        if let Some(c) = self.ctx {
            // SAFETY: `c` is open
            unsafe { d::dav1d_flush(c) };
        }
    }
}

impl Drop for Av1 {
    fn drop(&mut self) {
        // SAFETY: `ctx` came from `dav1d_open` and is closed once
        unsafe { d::dav1d_close(Some(NonNull::from(&mut self.ctx))) };
    }
}

/// a decoded 8-bit 4:2:0 picture's planes, copied out of rav1d's pool with tight rows (the GPU converts them)
fn planes(pic: &Dav1dPicture) -> Result<Picture, Error> {
    let (w, h) = (pic.p.w as usize, pic.p.h as usize);
    if pic.p.bpc != 8 || pic.p.layout != DAV1D_PIXEL_LAYOUT_I420 {
        return err(format!("AV1: {}-bit layout {} – only 8-bit 4:2:0 plays yet", pic.p.bpc, pic.p.layout));
    }
    let (ys, cs) = (pic.stride[0] as usize, pic.stride[1] as usize);
    let (cw, ch) = (w.div_ceil(2), h.div_ceil(2));
    let mut data = Vec::with_capacity(w * h + 2 * cw * ch);
    for (i, stride, rows, width) in [(0, ys, h, w), (1, cs, ch, cw), (2, cs, ch, cw)] {
        let p = pic.data[i].ok_or_else(|| Error("AV1: a picture without its planes".into()))?;
        // SAFETY: rav1d's plane `i` holds `rows` rows of `stride` bytes (the last one at least `width`)
        let plane = unsafe { std::slice::from_raw_parts(p.as_ptr().cast::<u8>(), stride * (rows - 1) + width) };
        for r in 0..rows {
            data.extend_from_slice(&plane[r * stride..][..width]);
        }
    }
    // SAFETY: a decoded picture carries its sequence header
    let seq = pic.seq_hdr.map(|s| unsafe { s.as_ref() });
    let matrix = match seq.map(|s| s.mtrx) {
        Some(m) if m == DAV1D_MC_BT601 || m == DAV1D_MC_BT470BG => Matrix::Bt601,
        _ => Matrix::Bt709,
    };
    let full = seq.is_some_and(|s| s.color_range != 0);
    Ok(Picture::new(w as u32, h as u32, Format::Yuv420 { matrix, full }, data))
}
