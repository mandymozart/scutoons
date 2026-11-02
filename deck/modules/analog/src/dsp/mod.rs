//! The synth's DSP units, plain Rust with no SDK in sight: band-limited oscillators, the state-variable filter, the
//! ADSR, the LFO, the pan law and seeded noise. Nothing here allocates, reads a clock or calls the platform's maths:
//! every transcendental is `math`'s own arithmetic, so a render is bit-identical native ≡ wasm (ENGINE-REFACTOR-PLAN
//! §3.4). They are candidates to become deck-engine kinds later (#317 / #318).
pub mod adsr;
pub mod lfo;
pub mod math;
pub mod noise;
pub mod osc;
pub mod pan;
pub mod svf;
