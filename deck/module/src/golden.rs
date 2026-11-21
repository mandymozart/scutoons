//! What a module's render tests share. `Hash`: FNV-1a 64 over the bits of every sample (and whatever else a module
//! writes) – a module's `tests/golden.rs` renders a script, hashes it and holds the number: a change of the hash is a
//! change of the sound. `Bench` (feature `host`): the module on its `Rig` with parameters by name, a seeded signal for
//! its inputs and the calls counted in frames, so the golden test and the no_alloc test drive one script.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hash(pub u64);

impl Default for Hash {
    fn default() -> Hash {
        Hash::new()
    }
}

impl Hash {
    pub const fn new() -> Hash {
        Hash(0xcbf2_9ce4_8422_2325)
    }

    pub fn bytes(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0 ^ u64::from(*b)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    pub fn f32s(&mut self, samples: &[f32]) {
        for v in samples {
            self.bytes(&v.to_bits().to_le_bytes());
        }
    }

    pub fn u32s(&mut self, values: &[u32]) {
        for v in values {
            self.bytes(&v.to_le_bytes());
        }
    }
}

/// A module on the test bench.
#[cfg(feature = "host")]
pub struct Bench<M: crate::Module> {
    pub rig: crate::testing::Rig<M>,
    /// the audio frame the next call starts at
    pub frame: u64,
    /// what `feed` plays into the inputs
    pub signal: crate::dsp::noise::White,
    /// name, min, max in descriptor order
    params: Vec<(String, f32, f32)>,
}

#[cfg(feature = "host")]
impl<M: crate::Module> Bench<M> {
    pub fn new(rate: f64, seed: u64) -> Bench<M> {
        let d = crate::host::parse(M::DESCRIPTOR).expect("the module's descriptor");
        let params = d.params.iter().map(|p| (p.name.to_string(), p.min, p.max)).collect();
        Bench { rig: crate::testing::Rig::new(rate, seed), frame: 0, signal: crate::dsp::noise::White::new(seed), params }
    }

    pub fn set(&mut self, name: &str, value: f32) -> &mut Bench<M> {
        let i = self.params.iter().position(|p| p.0 == name).unwrap_or_else(|| panic!("{}: no parameter '{name}'", M::NAME));
        self.rig.set_param(i, value);
        self
    }

    /// one parameter to a place in its range, both picked by `step`: over a run every parameter visits its whole range
    pub fn turn(&mut self, step: usize) {
        let i = step % self.params.len();
        let (_, lo, hi) = self.params[i];
        self.rig.set_param(i, lo + (hi - lo) * (step % 13) as f32 / 12.0);
    }

    /// audio input `p`, channel `c`: `frames` samples of the seeded signal times `level`
    pub fn feed(&mut self, p: usize, c: usize, frames: usize, level: f32) {
        let mut buf = [0.0f32; crate::QUANTUM];
        for s in buf[..frames].iter_mut() {
            *s = self.signal.sample() * level;
        }
        self.rig.set_audio_in(p, c, &buf[..frames]);
    }

    /// one call of `frames` frames (at most `QUANTUM`) where the last one ended
    pub fn run(&mut self, frames: usize) {
        self.rig.run(self.frame as f64, frames as u32);
        self.frame += frames as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::Hash;

    #[test]
    fn the_hash_is_fnv_1a_and_sees_every_bit() {
        let mut h = Hash::new();
        h.bytes(b"a");
        assert_eq!(h.0, 0xaf63_dc4c_8601_ec8c);
        let (mut a, mut b) = (Hash::new(), Hash::new());
        a.f32s(&[0.0]);
        b.f32s(&[-0.0]);
        assert_ne!(a, b);
    }
}
