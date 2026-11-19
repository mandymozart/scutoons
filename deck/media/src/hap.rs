//! HAP frames (Vidvox's open spec): a section header, then the texture's DXT blocks – stored as they are, Snappy
//! compressed whole, or cut into chunks each stored or Snappy compressed (the "decode instructions"). Out come the
//! blocks, ready for a GPU that samples BC; `Picture::to_rgba` is the software path.
use crate::{err, Error, Format, Picture};

/// a section: its type byte and its body
fn section(b: &[u8]) -> Result<(u8, &[u8], usize), Error> {
    if b.len() < 4 {
        return err("HAP: a section header is cut off");
    }
    let small = u32::from_le_bytes([b[0], b[1], b[2], 0]) as usize;
    let (size, head) = if small == 0 {
        if b.len() < 8 {
            return err("HAP: a section header is cut off");
        }
        (u32::from_le_bytes([b[4], b[5], b[6], b[7]]) as usize, 8)
    } else {
        (small, 4)
    };
    let body = b.get(head..head + size).ok_or_else(|| Error("HAP: a section runs past the frame".into()))?;
    Ok((b[3], body, head + size))
}

fn format(t: u8) -> Result<Format, Error> {
    match t & 0x0f {
        0x0b => Ok(Format::Bc1),
        0x0e => Ok(Format::Bc3),
        0x0f => Ok(Format::Bc3YCoCg),
        f => err(format!("HAP: texture format {f:#x} is not decoded yet (HAP Alpha-only, HAP R, HAP Q Alpha)")),
    }
}

/// the bytes of a `width` × `height` texture in `f`'s blocks
pub fn texture_len(f: Format, width: u32, height: u32) -> usize {
    let blocks = width.div_ceil(4) as usize * height.div_ceil(4) as usize;
    match f {
        Format::Bc1 => blocks * 8,
        Format::Bc3 | Format::Bc3YCoCg => blocks * 16,
        Format::Rgba8 => width as usize * height as usize * 4,
        Format::Yuv420 { .. } | Format::Nv12 { .. } => (width * height + 2 * width.div_ceil(2) * height.div_ceil(2)) as usize,
    }
}

fn snappy(src: &[u8], out: &mut Vec<u8>) -> Result<(), Error> {
    let n = snap::raw::decompress_len(src).map_err(|e| Error(format!("HAP: Snappy: {e}")))?;
    let at = out.len();
    out.resize(at + n, 0);
    snap::raw::Decoder::new().decompress(src, &mut out[at..]).map_err(|e| Error(format!("HAP: Snappy: {e}")))?;
    Ok(())
}

/// One HAP frame of a `width` × `height` clip → its blocks.
pub fn decode(frame: &[u8], width: u32, height: u32) -> Result<Picture, Error> {
    let (t, body, _) = section(frame)?;
    let format = format(t)?;
    let want = texture_len(format, width, height);
    let mut data = Vec::with_capacity(want);
    match t >> 4 {
        0x0a => data.extend_from_slice(body),
        0x0b => snappy(body, &mut data)?,
        0x0c => chunks(body, &mut data)?,
        c => return err(format!("HAP: compressor {c:#x} unknown")),
    }
    if data.len() != want {
        return err(format!("HAP: {} bytes of blocks for a {width} × {height} frame, {want} expected", data.len()));
    }
    Ok(Picture { width, height, format, data })
}

/// the decode instructions, then the chunks they describe
fn chunks(body: &[u8], out: &mut Vec<u8>) -> Result<(), Error> {
    let (t, instructions, used) = section(body)?;
    if t != 0x01 {
        return err("HAP: a chunked frame without its decode instructions");
    }
    let (mut compressors, mut sizes, mut offsets) = (&[][..], Vec::new(), None);
    let mut rest = instructions;
    while !rest.is_empty() {
        let (t, b, n) = section(rest)?;
        let words = || b.chunks_exact(4).map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]) as usize).collect::<Vec<_>>();
        match t {
            0x02 => compressors = b,
            0x03 => sizes = words(),
            0x04 => offsets = Some(words()),
            _ => {}
        }
        rest = &rest[n..];
    }
    if compressors.len() != sizes.len() || offsets.as_ref().is_some_and(|o| o.len() != sizes.len()) {
        return err("HAP: the decode instructions' tables differ in length");
    }
    let data = &body[used..];
    let mut at = 0;
    for (i, (&c, &len)) in compressors.iter().zip(&sizes).enumerate() {
        let start = offsets.as_ref().map_or(at, |o| o[i]);
        let chunk = data.get(start..start + len).ok_or_else(|| Error("HAP: a chunk runs past the frame".into()))?;
        match c {
            0x0a => out.extend_from_slice(chunk),
            0x0b => snappy(chunk, out)?,
            c => return err(format!("HAP: chunk compressor {c:#x} unknown")),
        }
        at = start + len;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(t: u8, body: &[u8]) -> Vec<u8> {
        let n = body.len() as u32;
        let mut out = vec![n as u8, (n >> 8) as u8, (n >> 16) as u8, t];
        out.extend_from_slice(body);
        out
    }

    #[test]
    fn stored_snappy_and_chunked_frames_give_the_same_blocks() {
        let blocks: Vec<u8> = (0..64u8).collect();
        let stored = decode(&header(0xab, &blocks), 16, 8).unwrap();
        assert_eq!((stored.format, &stored.data), (Format::Bc1, &blocks));
        let packed = snap::raw::Encoder::new().compress_vec(&blocks).unwrap();
        assert_eq!(decode(&header(0xbb, &packed), 16, 8).unwrap(), stored);
        let (a, b) = (snap::raw::Encoder::new().compress_vec(&blocks[..32]).unwrap(), &blocks[32..]);
        let mut sizes = (a.len() as u32).to_le_bytes().to_vec();
        sizes.extend_from_slice(&(b.len() as u32).to_le_bytes());
        let mut instr = header(0x02, &[0x0b, 0x0a]);
        instr.extend(header(0x03, &sizes));
        let mut body = header(0x01, &instr);
        body.extend_from_slice(&a);
        body.extend_from_slice(b);
        assert_eq!(decode(&header(0xcb, &body), 16, 8).unwrap(), stored);
    }

    #[test]
    fn a_wrong_size_or_format_is_named() {
        assert!(decode(&header(0xab, &[0; 8]), 16, 8).unwrap_err().0.contains("expected"));
        assert!(decode(&header(0xa1, &[0; 8]), 4, 4).unwrap_err().0.contains("not decoded yet"));
        assert!(decode(&[1, 0], 4, 4).is_err());
    }
}
