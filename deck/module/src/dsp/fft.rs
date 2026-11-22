//! The FFT (radix 2, a power-of-two length): a real signal into its spectrum and back. The twiddles and the bit-reversal
//! table are built in `new` (libm, so the same bits everywhere) – call it in `init`; a transform works in the caller's
//! two buffers and allocates nothing.

#[derive(Debug, Clone)]
pub struct Fft {
    n: usize,
    /// cos and sin of 2π k / n, k < n / 2
    cos: Box<[f32]>,
    sin: Box<[f32]>,
    /// the bit-reversed index of every index
    rev: Box<[u32]>,
}

impl Fft {
    /// for transforms of `n` points; `n` is raised to the next power of two, at least 2 (allocates)
    pub fn new(n: usize) -> Fft {
        let n = n.max(2).next_power_of_two();
        let turn = |k: usize| 2.0 * core::f64::consts::PI * k as f64 / n as f64;
        let bits = n.trailing_zeros();
        Fft {
            n,
            cos: (0..n / 2).map(|k| libm::cos(turn(k)) as f32).collect(),
            sin: (0..n / 2).map(|k| libm::sin(turn(k)) as f32).collect(),
            rev: (0..n as u32).map(|i| i.reverse_bits() >> (32 - bits)).collect(),
        }
    }

    /// the transform's length
    pub fn len(&self) -> usize {
        self.n
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    /// The spectrum of the real signal `x` (`len()` samples) into `re` / `im`: bin k is k · rate / len Hz, bins
    /// 0 ..= len / 2 are the spectrum, the rest its mirror image. A sine of amplitude a on a bin reads a · len / 2.
    pub fn real(&self, x: &[f32], re: &mut [f32], im: &mut [f32]) {
        re[..self.n].copy_from_slice(&x[..self.n]);
        im[..self.n].fill(0.0);
        self.forward(re, im);
    }

    /// the complex transform, in place
    pub fn forward(&self, re: &mut [f32], im: &mut [f32]) {
        self.permute(re, im);
        self.passes(re, im, 0, self.depth());
    }

    /// how many butterfly passes a transform takes: log2 of the length
    pub fn depth(&self) -> u32 {
        self.n.trailing_zeros()
    }

    /// `forward` in pieces, the same bits (to spread one transform over several calls): `permute`, then `passes` from
    /// 0 up to `depth()` in any steps
    pub fn permute(&self, re: &mut [f32], im: &mut [f32]) {
        let n = self.n;
        let (re, im) = (&mut re[..n], &mut im[..n]);
        for i in 0..n {
            let j = self.rev[i] as usize;
            if i < j {
                re.swap(i, j);
                im.swap(i, j);
            }
        }
    }

    /// the butterfly passes `from .. to` (pass s joins halves of 2^s points)
    pub fn passes(&self, re: &mut [f32], im: &mut [f32], from: u32, to: u32) {
        let n = self.n;
        let (re, im) = (&mut re[..n], &mut im[..n]);
        let mut len = 2usize << from.min(31);
        let end = 2usize << to.min(self.depth()).max(from);
        while len < end {
            let (half, stride) = (len / 2, n / len);
            let mut start = 0;
            while start < n {
                for k in 0..half {
                    let (wr, wi) = (self.cos[k * stride], -self.sin[k * stride]);
                    let (a, b) = (start + k, start + k + half);
                    let (tr, ti) = (re[b] * wr - im[b] * wi, re[b] * wi + im[b] * wr);
                    (re[b], im[b]) = (re[a] - tr, im[a] - ti);
                    (re[a], im[a]) = (re[a] + tr, im[a] + ti);
                }
                start += len;
            }
            len <<= 1;
        }
    }

    /// the inverse of `forward`, in place, scaled so the two undo each other: a spectrum made by `real` becomes its
    /// signal in `re`
    pub fn inverse(&self, re: &mut [f32], im: &mut [f32]) {
        // the forward transform with the two parts swapped is the inverse times n
        self.forward(im, re);
        let scale = 1.0 / self.n as f32;
        for v in re[..self.n].iter_mut().chain(im[..self.n].iter_mut()) {
            *v *= scale;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::noise::White;
    use super::super::probe;
    use super::*;

    #[test]
    fn a_sine_on_a_bin_is_that_bin_alone() {
        for (n, bin, amp) in [(256usize, 5usize, 0.8f32), (1024, 100, 0.25), (16_384, 211, 1.0)] {
            let fft = Fft::new(n);
            assert_eq!(fft.len(), n);
            let x: Vec<f32> = (0..n).map(|i| amp * libm::sin(2.0 * core::f64::consts::PI * (bin * i) as f64 / n as f64) as f32).collect();
            let (mut re, mut im) = (vec![0.0f32; n], vec![0.0f32; n]);
            fft.real(&x, &mut re, &mut im);
            let mag = |k: usize| f64::from(re[k]).hypot(f64::from(im[k])) * 2.0 / n as f64;
            assert!((mag(bin) - f64::from(amp)).abs() < 1e-5, "n {n}: bin {bin} reads {}", mag(bin));
            // and the mirror image at n − bin; everything else is under −110 dB of it
            assert!((mag(n - bin) - f64::from(amp)).abs() < 1e-5);
            let rest = (0..n).filter(|&k| k != bin && k != n - bin).map(mag).fold(0.0, f64::max);
            assert!(probe::db(rest / f64::from(amp)) < -110.0, "n {n}: leakage {:.1} dB", probe::db(rest / f64::from(amp)));
        }
    }

    #[test]
    fn it_agrees_with_the_f64_transform_and_keeps_the_energy() {
        let n = 4096;
        let fft = Fft::new(n);
        let mut w = White::new(11);
        let x: Vec<f32> = (0..n).map(|_| w.sample()).collect();
        let (mut re, mut im) = (vec![0.0f32; n], vec![0.0f32; n]);
        fft.real(&x, &mut re, &mut im);
        let mut want_re: Vec<f64> = x.iter().map(|&v| f64::from(v)).collect();
        let mut want_im = vec![0.0f64; n];
        probe::fft(&mut want_re, &mut want_im);
        let worst = (0..n).map(|k| (f64::from(re[k]) - want_re[k]).abs().max((f64::from(im[k]) - want_im[k]).abs())).fold(0.0, f64::max);
        assert!(worst < 2e-3, "against the f64 transform: {worst:e} (bins are of size ~{:.0})", (n as f64 / 3.0).sqrt());
        // Parseval: the energy in time is the energy of the bins / n
        let time: f64 = x.iter().map(|&v| f64::from(v).powi(2)).sum();
        let freq: f64 = (0..n).map(|k| f64::from(re[k]).powi(2) + f64::from(im[k]).powi(2)).sum::<f64>() / n as f64;
        assert!((time / freq - 1.0).abs() < 1e-5, "{time} against {freq}");
        // a real signal's spectrum mirrors: bin n − k is bin k's conjugate
        for k in 1..n / 2 {
            assert!((re[k] - re[n - k]).abs() < 1e-3 && (im[k] + im[n - k]).abs() < 1e-3, "bin {k}");
        }
    }

    #[test]
    fn there_and_back_is_the_signal() {
        for n in [2usize, 8, 256, 2048] {
            let fft = Fft::new(n);
            let mut w = White::new(n as u64);
            let x: Vec<f32> = (0..n).map(|_| w.sample()).collect();
            let (mut re, mut im) = (vec![0.0f32; n], vec![0.0f32; n]);
            fft.real(&x, &mut re, &mut im);
            fft.inverse(&mut re, &mut im);
            let worst = (0..n).map(|i| (re[i] - x[i]).abs().max(im[i].abs())).fold(0.0, f32::max);
            assert!(worst < 2e-6, "n {n}: {worst:e}");
        }
    }

    #[test]
    fn an_impulse_is_flat_and_a_delay_turns_the_phase() {
        let n = 256;
        let fft = Fft::new(n);
        let mut x = vec![0.0f32; n];
        x[1] = 1.0;
        let (mut re, mut im) = (vec![0.0f32; n], vec![0.0f32; n]);
        fft.real(&x, &mut re, &mut im);
        for k in 0..n {
            let turn = 2.0 * core::f64::consts::PI * k as f64 / n as f64;
            assert!((f64::from(re[k]) - turn.cos()).abs() < 1e-6 && (f64::from(im[k]) + turn.sin()).abs() < 1e-6, "bin {k}");
        }
    }

    #[test]
    fn the_length_is_a_power_of_two_and_the_tables_are_the_same_every_time() {
        assert_eq!((Fft::new(0).len(), Fft::new(3).len(), Fft::new(1000).len(), Fft::new(1024).len()), (2, 4, 1024, 1024));
        let (a, b) = (Fft::new(512), Fft::new(512));
        assert!(a.cos == b.cos && a.sin == b.sin && a.rev == b.rev);
    }

    #[test]
    fn the_transform_in_pieces_is_the_same_bits() {
        let fft = Fft::new(2048);
        assert_eq!(fft.depth(), 11);
        let mut w = White::new(3);
        let x: Vec<f32> = (0..2048).map(|_| w.sample()).collect();
        let (mut re, mut im) = (x.clone(), vec![0.0f32; 2048]);
        fft.forward(&mut re, &mut im);
        let (mut re2, mut im2) = (x, vec![0.0f32; 2048]);
        fft.permute(&mut re2, &mut im2);
        for (a, b) in [(0, 4), (4, 4), (4, 9), (9, 11)] {
            fft.passes(&mut re2, &mut im2, a, b);
        }
        assert!(re == re2 && im == im2);
    }
}
