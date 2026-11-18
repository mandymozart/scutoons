//! 8-bit YUV 4:2:0 planes to RGBA8 in integers (BT.601 / BT.709, limited or full range): the goldens and a host without
//! a GPU; a deck's GPU turns the planes into RGB in a shader (deck-space video.wgsl, the same coefficients).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Matrix {
    Bt601,
    Bt709,
}

/// The planes of a 4:2:0 picture: luma `w` × `h`, chroma half size rounded up, each with its stride.
pub struct Planes<'a> {
    pub y: (&'a [u8], usize),
    pub u: (&'a [u8], usize),
    pub v: (&'a [u8], usize),
}

impl Matrix {
    /// (a, rv, gu, gv, bu) ×256 and the black level: R = a·(Y − black) + rv·V, G = a·(Y − black) − gu·U − gv·V,
    /// B = a·(Y − black) + bu·U (U, V centred on 128)
    pub fn coefficients(self, full: bool) -> ([i32; 5], i32) {
        let c = match (self, full) {
            (Matrix::Bt709, false) => [298, 459, 55, 136, 541],
            (Matrix::Bt709, true) => [256, 403, 48, 120, 475],
            (Matrix::Bt601, false) => [298, 409, 100, 208, 516],
            (Matrix::Bt601, true) => [256, 359, 88, 183, 454],
        };
        (c, if full { 0 } else { 16 })
    }
}

/// RGBA8, `w` × `h`
pub fn to_rgba(p: &Planes, w: usize, h: usize, matrix: Matrix, full: bool) -> Vec<u8> {
    let ([a, rv, gu, gv, bu], floor) = matrix.coefficients(full);
    let mut out = vec![0u8; w * h * 4];
    for row in 0..h {
        let (ys, us, vs) = (&p.y.0[row * p.y.1..], &p.u.0[row / 2 * p.u.1..], &p.v.0[row / 2 * p.v.1..]);
        for col in 0..w {
            let c = a * (i32::from(ys[col]) - floor);
            let (d, e) = (i32::from(us[col / 2]) - 128, i32::from(vs[col / 2]) - 128);
            let px = &mut out[(row * w + col) * 4..][..4];
            px[0] = ((c + rv * e + 128) >> 8).clamp(0, 255) as u8;
            px[1] = ((c - gu * d - gv * e + 128) >> 8).clamp(0, 255) as u8;
            px[2] = ((c + bu * d + 128) >> 8).clamp(0, 255) as u8;
            px[3] = 255;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_white_and_grey() {
        let planes = |y: u8| (vec![y; 4], vec![128u8; 1], vec![128u8; 1]);
        for (y, full, want) in [(16, false, 0), (235, false, 255), (0, true, 0), (255, true, 255), (128, true, 128)] {
            let (ys, us, vs) = planes(y);
            let rgba = to_rgba(&Planes { y: (&ys, 2), u: (&us, 1), v: (&vs, 1) }, 2, 2, Matrix::Bt709, full);
            assert_eq!(&rgba[..4], &[want, want, want, 255], "Y {y} full {full}");
        }
    }
}
