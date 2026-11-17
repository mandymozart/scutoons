//! `md_inbox(len)` → where to write a clip of `len` bytes; `md_golden(len)` → the golden line's length (negative: an
//! error's), its text at `md_outbox()`.
use std::sync::Mutex;

static INBOX: Mutex<Vec<u8>> = Mutex::new(Vec::new());
static OUTBOX: Mutex<Vec<u8>> = Mutex::new(Vec::new());

#[no_mangle]
pub extern "C" fn md_inbox(len: u32) -> u32 {
    let mut b = INBOX.lock().unwrap();
    b.clear();
    b.resize(len as usize, 0);
    b.as_mut_ptr() as usize as u32
}

#[no_mangle]
pub extern "C" fn md_golden(len: u32) -> i32 {
    let input = INBOX.lock().unwrap();
    let (text, ok) = match deck_media::golden(&input[..len as usize]) {
        Ok(g) => (g.line(), true),
        Err(e) => (e.0, false),
    };
    let mut out = OUTBOX.lock().unwrap();
    *out = text.into_bytes();
    if ok { out.len() as i32 } else { -(out.len() as i32) }
}

/// the clip's sound as f32 LE bytes at `md_outbox()`: their length (negative: an error's)
#[no_mangle]
pub extern "C" fn md_pcm(len: u32) -> i32 {
    let input = INBOX.lock().unwrap();
    let (bytes, ok) = match deck_media::audio::decode(input[..len as usize].to_vec()) {
        Ok(p) => (p.map(|p| p.samples).unwrap_or_default().iter().flat_map(|s| s.to_le_bytes()).collect(), true),
        Err(e) => (e.0.into_bytes(), false),
    };
    let mut out = OUTBOX.lock().unwrap();
    *out = bytes;
    if ok { out.len() as i32 } else { -(out.len() as i32) }
}

#[no_mangle]
pub extern "C" fn md_outbox() -> u32 {
    OUTBOX.lock().unwrap().as_ptr() as usize as u32
}
