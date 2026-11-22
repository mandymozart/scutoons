//! Uniformly partitioned FFT convolution (overlap-save with a frequency-domain delay line): one channel of a signal
//! through an impulse response, a block of `part` samples at a time, the result one block late. The impulse response is
//! cut into partitions of `part` samples, each transformed once in `new` (allocates); `block` then transforms the last
//! two blocks of input (`2 · part` points), multiplies every partition's spectrum with the input's spectrum that many
//! blocks ago and sums them (`mac`), and takes the last `part` samples of the inverse – nothing allocates.
//! Only the bins 0 ..= part are kept: the input and the response are real, the rest is their mirror image. `Split` puts
//! a long response's tail into longer partitions (`Tail`) at the same latency.
use super::fft::Fft;

/// `acc += x · h`, bin by bin, for complex spectra in split form (the real parts, then the imaginary ones)
#[inline(always)]
pub fn mac(acc_re: &mut [f32], acc_im: &mut [f32], x_re: &[f32], x_im: &[f32], h_re: &[f32], h_im: &[f32]) {
    let n = acc_re.len();
    let (acc_im, x_re, x_im, h_re, h_im) = (&mut acc_im[..n], &x_re[..n], &x_im[..n], &h_re[..n], &h_im[..n]);
    for k in 0..n {
        acc_re[k] += x_re[k] * h_re[k] - x_im[k] * h_im[k];
        acc_im[k] += x_re[k] * h_im[k] + x_im[k] * h_re[k];
    }
}

#[derive(Debug, Clone)]
pub struct Partitioned {
    fft: Fft,
    part: usize,
    /// bins kept a spectrum: part + 1
    bins: usize,
    parts: usize,
    /// how many partitions sound (`set_active`)
    active: usize,
    /// the response's spectra, partition after partition
    h_re: Box<[f32]>,
    h_im: Box<[f32]>,
    /// the input's spectra, a ring of `parts`; `head` is the newest
    x_re: Box<[f32]>,
    x_im: Box<[f32]>,
    head: usize,
    /// the block before the newest
    prev: Box<[f32]>,
    re: Box<[f32]>,
    im: Box<[f32]>,
    acc_re: Box<[f32]>,
    acc_im: Box<[f32]>,
}

impl Partitioned {
    /// the response `ir` in partitions of `part` samples (a power of two; at least one partition, also for an empty
    /// response); allocates
    pub fn new(ir: &[f32], part: usize) -> Partitioned {
        let part = part.max(1).next_power_of_two();
        let fft = Fft::new(2 * part);
        let bins = part + 1;
        let parts = ir.len().div_ceil(part).max(1);
        let (mut h_re, mut h_im) = (vec![0.0f32; parts * bins], vec![0.0f32; parts * bins]);
        let (mut re, mut im) = (vec![0.0f32; 2 * part], vec![0.0f32; 2 * part]);
        for p in 0..parts {
            let piece = &ir[(p * part).min(ir.len())..((p + 1) * part).min(ir.len())];
            re.fill(0.0);
            re[..piece.len()].copy_from_slice(piece);
            im.fill(0.0);
            fft.forward(&mut re, &mut im);
            h_re[p * bins..(p + 1) * bins].copy_from_slice(&re[..bins]);
            h_im[p * bins..(p + 1) * bins].copy_from_slice(&im[..bins]);
        }
        Partitioned {
            fft,
            part,
            bins,
            parts,
            active: parts,
            h_re: h_re.into(),
            h_im: h_im.into(),
            x_re: vec![0.0; parts * bins].into(),
            x_im: vec![0.0; parts * bins].into(),
            head: 0,
            prev: vec![0.0; part].into(),
            re: re.into(),
            im: im.into(),
            acc_re: vec![0.0; bins].into(),
            acc_im: vec![0.0; bins].into(),
        }
    }

    /// the block length
    pub fn part(&self) -> usize {
        self.part
    }

    /// the response's partitions
    pub fn parts(&self) -> usize {
        self.parts
    }

    /// only the first `n` partitions sound (kept in 1 ..= parts): the response cut to `n · part` samples
    pub fn set_active(&mut self, n: usize) {
        self.active = n.clamp(1, self.parts);
    }

    pub fn active(&self) -> usize {
        self.active
    }

    /// silence: no input heard before
    pub fn clear(&mut self) {
        for b in [&mut self.x_re, &mut self.x_im, &mut self.prev] {
            b.fill(0.0);
        }
    }

    /// `x` (`part` samples) in, the convolution's `part` samples for it out into `y`: `y[i]` is the output at the time
    /// of `x[i]`, so a host that plays `y` a block later delays the result by exactly `part`
    pub fn block(&mut self, x: &[f32], y: &mut [f32]) {
        let (part, bins) = (self.part, self.bins);
        self.re[..part].copy_from_slice(&self.prev);
        self.re[part..].copy_from_slice(&x[..part]);
        self.prev.copy_from_slice(&x[..part]);
        self.im.fill(0.0);
        self.fft.forward(&mut self.re, &mut self.im);
        self.head = (self.head + self.parts - 1) % self.parts;
        let at = self.head * bins;
        self.x_re[at..at + bins].copy_from_slice(&self.re[..bins]);
        self.x_im[at..at + bins].copy_from_slice(&self.im[..bins]);
        self.acc_re.fill(0.0);
        self.acc_im.fill(0.0);
        for p in 0..self.active {
            let (xi, hi) = (((self.head + p) % self.parts) * bins, p * bins);
            mac(
                &mut self.acc_re,
                &mut self.acc_im,
                &self.x_re[xi..xi + bins],
                &self.x_im[xi..xi + bins],
                &self.h_re[hi..hi + bins],
                &self.h_im[hi..hi + bins],
            );
        }
        // the whole spectrum from its half: bin 2 · part − k is bin k's conjugate
        let n = 2 * part;
        self.re[..bins].copy_from_slice(&self.acc_re);
        self.im[..bins].copy_from_slice(&self.acc_im);
        for k in bins..n {
            self.re[k] = self.acc_re[n - k];
            self.im[k] = -self.acc_im[n - k];
        }
        self.fft.inverse(&mut self.re, &mut self.im);
        y[..part].copy_from_slice(&self.re[part..]);
    }
}

/// The response's tail in long partitions (`part` samples, at least four of the head's blocks `sub`), fed `sub` samples a
/// call. A tail block's input is transformed, multiplied and transformed back during the next tail block, a piece a
/// call (the forward transform in two calls, the products in slices, the inverse in two), and its output plays during
/// the block after that: a tail that starts `2 · part` samples into the response is on time, and no call carries more
/// than one share of the work.
#[derive(Debug, Clone)]
pub struct Tail {
    fft: Fft,
    part: usize,
    sub: usize,
    bins: usize,
    parts: usize,
    active: usize,
    h_re: Box<[f32]>,
    h_im: Box<[f32]>,
    x_re: Box<[f32]>,
    x_im: Box<[f32]>,
    head: usize,
    /// the two input blocks before this one: the window being worked on
    prev: Box<[f32]>,
    pending: Box<[f32]>,
    input: Box<[f32]>,
    /// the output playing now, and the one being made
    out: Box<[f32]>,
    next: Box<[f32]>,
    fill: usize,
    re: Box<[f32]>,
    im: Box<[f32]>,
    acc_re: Box<[f32]>,
    acc_im: Box<[f32]>,
}

impl Tail {
    /// `ir` in partitions of `part`, fed `sub` samples a call (powers of two; `part` at least `4 · sub`); allocates
    pub fn new(ir: &[f32], part: usize, sub: usize) -> Tail {
        let sub = sub.max(1).next_power_of_two();
        let p = Partitioned::new(ir, part.max(4 * sub));
        let part = p.part;
        Tail {
            fft: p.fft,
            part,
            sub,
            bins: p.bins,
            parts: p.parts,
            active: p.parts,
            h_re: p.h_re,
            h_im: p.h_im,
            x_re: p.x_re,
            x_im: p.x_im,
            head: 0,
            prev: p.prev,
            pending: vec![0.0; part].into(),
            input: vec![0.0; part].into(),
            out: vec![0.0; part].into(),
            next: vec![0.0; part].into(),
            fill: 0,
            re: p.re,
            im: p.im,
            acc_re: p.acc_re,
            acc_im: p.acc_im,
        }
    }

    pub fn parts(&self) -> usize {
        self.parts
    }

    /// only the first `n` partitions sound (0 ..= parts)
    pub fn set_active(&mut self, n: usize) {
        self.active = n.min(self.parts);
    }

    pub fn clear(&mut self) {
        for b in [&mut self.x_re, &mut self.x_im, &mut self.prev, &mut self.pending, &mut self.input, &mut self.out, &mut self.next, &mut self.re, &mut self.im, &mut self.acc_re, &mut self.acc_im] {
            b.fill(0.0);
        }
    }

    /// `x` (`sub` samples) in; `y` gets the tail's output at their times added: the convolution `2 · part` samples ago
    pub fn push(&mut self, x: &[f32], y: &mut [f32]) {
        let (sub, bins, o, part) = (self.sub, self.bins, self.fill, self.part);
        for (y, v) in y[..sub].iter_mut().zip(&self.out[o..o + sub]) {
            *y += v;
        }
        self.input[o..o + sub].copy_from_slice(&x[..sub]);
        let (slot, slots, depth) = (o / sub, part / sub, self.fft.depth());
        let half = depth / 2;
        match slot {
            0 => {
                // the window: the block before the pending one, then the pending one
                self.re[..part].copy_from_slice(&self.prev);
                self.re[part..].copy_from_slice(&self.pending);
                self.prev.copy_from_slice(&self.pending);
                self.im.fill(0.0);
                self.fft.permute(&mut self.re, &mut self.im);
                self.fft.passes(&mut self.re, &mut self.im, 0, half);
            }
            1 => {
                self.fft.passes(&mut self.re, &mut self.im, half, depth);
                self.head = (self.head + self.parts - 1) % self.parts;
                let at = self.head * bins;
                self.x_re[at..at + bins].copy_from_slice(&self.re[..bins]);
                self.x_im[at..at + bins].copy_from_slice(&self.im[..bins]);
                self.acc_re.fill(0.0);
                self.acc_im.fill(0.0);
            }
            _ => {}
        }
        // the products in the slots 1 .. slots − 2, the newest input block's included
        let macs = slots - 2;
        if (1..=macs).contains(&slot) {
            let chunk = self.active.div_ceil(macs);
            let from = (slot - 1) * chunk;
            for p in from..(from + chunk).min(self.active) {
                let (xi, hi) = (((self.head + p) % self.parts) * bins, p * bins);
                mac(&mut self.acc_re, &mut self.acc_im, &self.x_re[xi..xi + bins], &self.x_im[xi..xi + bins], &self.h_re[hi..hi + bins], &self.h_im[hi..hi + bins]);
            }
        }
        if slot == slots - 2 {
            // the inverse is the forward transform with the parts swapped, divided by its length
            let n = 2 * part;
            self.re[..bins].copy_from_slice(&self.acc_re);
            self.im[..bins].copy_from_slice(&self.acc_im);
            for k in bins..n {
                self.re[k] = self.acc_re[n - k];
                self.im[k] = -self.acc_im[n - k];
            }
            self.fft.permute(&mut self.im, &mut self.re);
            self.fft.passes(&mut self.im, &mut self.re, 0, half);
        } else if slot == slots - 1 {
            self.fft.passes(&mut self.im, &mut self.re, half, depth);
            let scale = 1.0 / (2 * part) as f32;
            for (o, v) in self.next.iter_mut().zip(&self.re[part..]) {
                *o = v * scale;
            }
        }
        self.fill += sub;
        if self.fill == part {
            self.fill = 0;
            core::mem::swap(&mut self.out, &mut self.next);
            core::mem::swap(&mut self.pending, &mut self.input);
        }
    }
}

/// Two-stage (non-uniform) partitioned convolution: the response's first `2 · tail_part` samples in partitions of `part`
/// (`Partitioned`, the latency), the rest in partitions of `tail_part` (`Tail`) – a 4 s response at 48 kHz is 16
/// partitions of 128 and 186 of 1024 instead of 1500 of 128. The same output as `Partitioned` with `part`, one block
/// late, within the transforms' rounding.
#[derive(Debug, Clone)]
pub struct Split {
    head: Partitioned,
    tail: Option<Tail>,
    tail_part: usize,
    len: usize,
}

impl Split {
    /// allocates
    pub fn new(ir: &[f32], part: usize, tail_part: usize) -> Split {
        let part = part.max(1).next_power_of_two();
        let tail_part = tail_part.max(4 * part).next_power_of_two();
        let cut = ir.len().min(2 * tail_part);
        let tail = (ir.len() > cut).then(|| Tail::new(&ir[cut..], tail_part, part));
        Split { head: Partitioned::new(&ir[..cut], part), tail, tail_part, len: ir.len() }
    }

    pub fn part(&self) -> usize {
        self.head.part()
    }

    /// the response's length, samples
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// the response cut to (about) `samples`: whole partitions of each stage, at least one of the head's
    pub fn set_length(&mut self, samples: usize) {
        self.head.set_active(samples.div_ceil(self.head.part()));
        if let Some(t) = self.tail.as_mut() {
            t.set_active(samples.saturating_sub(2 * self.tail_part).div_ceil(self.tail_part));
        }
    }

    pub fn clear(&mut self) {
        self.head.clear();
        if let Some(t) = self.tail.as_mut() {
            t.clear();
        }
    }

    /// as `Partitioned::block`
    pub fn block(&mut self, x: &[f32], y: &mut [f32]) {
        self.head.block(x, y);
        if let Some(t) = self.tail.as_mut() {
            t.push(x, y);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::noise::White;
    use super::*;

    fn direct(x: &[f32], h: &[f32]) -> Vec<f64> {
        (0..x.len()).map(|n| (0..h.len().min(n + 1)).map(|k| f64::from(x[n - k]) * f64::from(h[k])).sum()).collect()
    }

    fn run(c: &mut Partitioned, x: &[f32]) -> Vec<f32> {
        let p = c.part();
        let mut y = vec![0.0f32; x.len()];
        for (xs, ys) in x.chunks(p).zip(y.chunks_mut(p)) {
            c.block(xs, ys);
        }
        y
    }

    #[test]
    fn it_is_the_direct_convolution() {
        let mut w = White::new(5);
        for (len, part) in [(1usize, 128usize), (100, 128), (128, 128), (1000, 128), (777, 64)] {
            let h: Vec<f32> = (0..len).map(|i| w.sample() * libm::expf(-(i as f32) / 200.0)).collect();
            let x: Vec<f32> = (0..part * 24).map(|_| w.sample()).collect();
            let mut c = Partitioned::new(&h, part);
            assert_eq!(c.parts(), len.div_ceil(part));
            let (y, want) = (run(&mut c, &x), direct(&x, &h));
            let err: f64 = y.iter().zip(&want).map(|(a, b)| (f64::from(*a) - b).powi(2)).sum();
            let sig: f64 = want.iter().map(|b| b * b).sum();
            assert!(10.0 * (err / sig).log10() < -100.0, "{len} / {part}: {:.1} dB", 10.0 * (err / sig).log10());
        }
    }

    #[test]
    fn fewer_active_partitions_cut_the_response_and_clear_forgets() {
        let h: Vec<f32> = (0..512).map(|i| 1.0 / (1 + i) as f32).collect();
        let mut c = Partitioned::new(&h, 128);
        c.set_active(2);
        assert_eq!(c.active(), 2);
        let mut x = vec![0.0f32; 1024];
        x[0] = 1.0;
        let y = run(&mut c, &x);
        for (i, v) in y.iter().enumerate() {
            let want = if i < 256 { h[i] } else { 0.0 };
            assert!((v - want).abs() < 1e-6, "{i}: {v}");
        }
        c.set_active(99);
        assert_eq!(c.active(), 4);
        run(&mut c, &[1.0; 256]);
        c.clear();
        assert!(run(&mut c, &[0.0; 1024]).iter().all(|v| v.abs() < 1e-9));
    }

    #[test]
    fn the_split_is_the_direct_convolution_and_cuts_by_stage() {
        let mut w = White::new(8);
        for len in [300usize, 2048, 2049, 5000, 9000, 20_000] {
            let h: Vec<f32> = (0..len).map(|i| w.sample() * libm::expf(-(i as f32) / 3000.0)).collect();
            let x: Vec<f32> = (0..128 * 240).map(|_| w.sample()).collect();
            let mut c = Split::new(&h, 128, 1024);
            assert_eq!(c.len(), len);
            let mut y = vec![0.0f32; x.len()];
            for (xs, ys) in x.chunks(128).zip(y.chunks_mut(128)) {
                c.block(xs, ys);
            }
            let want = direct(&x, &h);
            let err: f64 = y.iter().zip(&want).map(|(a, b)| (f64::from(*a) - b).powi(2)).sum();
            let sig: f64 = want.iter().map(|b| b * b).sum();
            assert!(10.0 * (err / sig).log10() < -100.0, "{len}: {:.1} dB", 10.0 * (err / sig).log10());
        }
        // cut to 3000 samples: the head's 16 partitions and one of the tail's
        let h = vec![1.0f32; 9000];
        let mut c = Split::new(&h, 128, 1024);
        c.set_length(3000);
        let mut x = vec![0.0f32; 128 * 80];
        x[0] = 1.0;
        let mut y = vec![0.0f32; x.len()];
        for (xs, ys) in x.chunks(128).zip(y.chunks_mut(128)) {
            c.block(xs, ys);
        }
        assert_eq!(y.iter().rposition(|v| v.abs() > 1e-4), Some(3071));
        c.clear();
        let mut y = vec![0.0f32; 128];
        for _ in 0..80 {
            c.block(&[0.0; 128], &mut y);
            assert!(y.iter().all(|v| v.abs() < 1e-9));
        }
    }
}
