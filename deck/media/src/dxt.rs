//! DXT1 / DXT5 (BC1 / BC3) blocks to RGBA8 in integers – the software path for a GPU without BC sampling and the
//! goldens; a GPU with BC samples the blocks as they are.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Block {
    Bc1,
    Bc3,
}

fn rgb565(c: u16) -> [i32; 3] {
    let (r, g, b) = (i32::from(c >> 11), i32::from((c >> 5) & 0x3f), i32::from(c & 0x1f));
    [(r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2)]
}

/// a colour block's sixteen pixels; `four`: always the four-colour mode (BC3's colour half)
fn colours(b: &[u8], four: bool, out: &mut [[u8; 4]; 16]) {
    let (c0, c1) = (u16::from_le_bytes([b[0], b[1]]), u16::from_le_bytes([b[2], b[3]]));
    let (p, q) = (rgb565(c0), rgb565(c1));
    let mix = |a: i32, b: i32, n: i32, d: i32| ((a * (d - n) + b * n) / d) as u8;
    let mut table = [[0u8; 4]; 4];
    for k in 0..3 {
        table[0][k] = p[k] as u8;
        table[1][k] = q[k] as u8;
        if four || c0 > c1 {
            table[2][k] = mix(p[k], q[k], 1, 3);
            table[3][k] = mix(p[k], q[k], 2, 3);
        } else {
            table[2][k] = mix(p[k], q[k], 1, 2);
        }
    }
    table[0][3] = 255;
    table[1][3] = 255;
    table[2][3] = 255;
    table[3][3] = if four || c0 > c1 { 255 } else { 0 };
    let bits = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
    for (i, px) in out.iter_mut().enumerate() {
        *px = table[(bits >> (2 * i) & 3) as usize];
    }
}

/// a BC3 alpha block over the pixels' alpha
fn alphas(b: &[u8], out: &mut [[u8; 4]; 16]) {
    let (a0, a1) = (i32::from(b[0]), i32::from(b[1]));
    let mut table = [0u8; 8];
    table[0] = a0 as u8;
    table[1] = a1 as u8;
    if a0 > a1 {
        for i in 1..7 {
            table[i + 1] = ((a0 * (7 - i as i32) + a1 * i as i32) / 7) as u8;
        }
    } else {
        for i in 1..5 {
            table[i + 1] = ((a0 * (5 - i as i32) + a1 * i as i32) / 5) as u8;
        }
        table[7] = 255;
    }
    let bits = b[2..8].iter().rev().fold(0u64, |acc, &x| (acc << 8) | u64::from(x));
    for (i, px) in out.iter_mut().enumerate() {
        px[3] = table[(bits >> (3 * i) & 7) as usize];
    }
}

/// `width` × `height` RGBA8 from the blocks
pub fn decode(blocks: &[u8], width: u32, height: u32, kind: Block) -> Vec<u8> {
    let (w, h) = (width as usize, height as usize);
    let size = if kind == Block::Bc1 { 8 } else { 16 };
    let across = w.div_ceil(4);
    let mut out = vec![0u8; w * h * 4];
    let mut px = [[0u8; 4]; 16];
    for (n, block) in blocks.chunks_exact(size).enumerate() {
        let (bx, by) = (n % across * 4, n / across * 4);
        if by >= h {
            break;
        }
        match kind {
            Block::Bc1 => colours(block, false, &mut px),
            Block::Bc3 => {
                colours(&block[8..], true, &mut px);
                alphas(block, &mut px);
            }
        }
        for (i, p) in px.iter().enumerate() {
            let (x, y) = (bx + i % 4, by + i / 4);
            if x < w && y < h {
                out[(y * w + x) * 4..][..4].copy_from_slice(p);
            }
        }
    }
    out
}

/// HAP Q's scaled YCoCg (Co, Cg, scale, Y in R, G, B, A) to RGB in place, alpha opaque – HAP's own shader in integers
pub fn ycocg_to_rgb(px: &mut [u8]) {
    for p in px.chunks_exact_mut(4) {
        let (co, cg, s, y) = (i32::from(p[0]) - 128, i32::from(p[1]) - 128, i32::from(p[2]) + 8, i32::from(p[3]));
        let div = |v: i32| (v * 8 + v.signum() * s / 2) / s;
        let (co, cg) = (div(co), div(cg));
        p[0] = (y + co - cg).clamp(0, 255) as u8;
        p[1] = (y + cg).clamp(0, 255) as u8;
        p[2] = (y - co - cg).clamp(0, 255) as u8;
        p[3] = 255;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bc1_block_picks_its_end_colours_and_the_two_between() {
        // red and blue, indices 0,1,2,3 along the first row
        let red: u16 = 0xf800;
        let blue: u16 = 0x001f;
        let mut b = red.to_le_bytes().to_vec();
        b.extend_from_slice(&blue.to_le_bytes());
        b.extend_from_slice(&[0b1110_0100, 0, 0, 0]);
        let px = decode(&b, 4, 1, Block::Bc1);
        assert_eq!(&px[0..4], &[255, 0, 0, 255]);
        assert_eq!(&px[4..8], &[0, 0, 255, 255]);
        assert_eq!(&px[8..12], &[170, 0, 85, 255]);
        assert_eq!(&px[12..16], &[85, 0, 170, 255]);
    }

    #[test]
    fn bc3_alpha_and_a_cropped_edge() {
        let mut b = vec![255, 0, 0b0000_1000, 0, 0, 0, 0, 0];
        b.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0]);
        let px = decode(&b, 2, 2, Block::Bc3);
        assert_eq!(px.len(), 16, "a 2 × 2 picture from one block");
        assert_eq!((px[3], px[7]), (255, 0), "the first two pixels' alpha: index 0 = 255, index 1 = 0");
    }

    #[test]
    fn grey_ycocg_stays_grey() {
        let mut p = [128, 128, 0, 77];
        ycocg_to_rgb(&mut p);
        assert_eq!(p, [77, 77, 77, 255]);
    }
}
