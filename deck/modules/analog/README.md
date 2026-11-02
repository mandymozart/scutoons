# analog – a two-oscillator analog synth as a module (#331)

The classic subtractive voice, written against the module SDK (`deck/module/README.md`) as ONE module: it runs the same in
the browser's engine and on a native deck (wasm), and a render is bit-identical native ≡ wasm. Basic on purpose: no UI (the
Sound design workspace builds one from the descriptor), no effects, no unison, no MIDI hardware.

```
Osc 1 ─┬─ sub 1 ─┐                                   LFO 1/2, filter env 1/2, amp env 1/2,
Osc 2 ─┼─ sub 2 ─┼─ route ─▶ Filter 1 ─▶ Amp 1 ─┐    key and velocity modulate pitch, pulse
Noise ─┘ (sync 1→2)      └▶ Filter 2 ─▶ Amp 2 ─┴─▶ stereo out    width, cutoff, resonance and level
             (serial: Filter 1 also feeds Filter 2)
```

Build: `cargo build --release --target wasm32-unknown-unknown -p analog-module` → `analog_module.wasm` (192 KB, 36 KB gzipped;
the SDK alone is ~100 KB). Tests: `cargo test -p analog-module`.

## Ports

| port | dir | what |
|---|---|---|
| `cv1` … `cv4` | audio in | four **note lanes**: pitch in Hz on the left channel, the gate on the right. The gate's value is the **velocity** (`seq`'s and `walk`'s 1.0 = full), 0 = off. |
| `sustain` | control in | ≥ 0.5 holds released notes (the pedal); letting go releases them. |
| `out` | audio out | stereo, the sum of the voices times `volume`. |

Why cv lanes: ABI v1 has no note-event input (`#[port(input, event)]` is refused: "events only go out of a module"), so notes
arrive the way the stage's `seq` and the `walk` module write them (`deck_module::cv`). A rising gate starts a note, a falling
one releases it; a **pitch change under a held gate glides the same voice** (legato, `walk`'s glide). Four lanes play four notes at
once; a note's release tail keeps sounding on its own voice while its lane plays the next, so eight voices are used up by
overlapping tails. A ninth note **steals the oldest** voice (a 1 ms fade-out first, no click). Link `walk.cv` into `cv1` to
hear it. An event input (note on/off with velocity, any number at a time) is ABI v2's job.

## Parameters

91, one table (`src/params.rs`): every row is `name: min..max = default, unit, choices, doc`, expanded into the module struct
(the derive reads range and default), into the `Patch` the voices read, and into `PARAMS_JSON` – the unit / choices / doc of
every parameter, in the wasm's custom section **`deck.params`** (the v1 descriptor carries only name, range and default; the
catalog descriptor of #315 takes units and docs, and this is the data for it). A switch is 0/1, a mode 0..n with its
`choices` in order (`saw|pulse|triangle|sine`). Groups: `volume` · `o1_*`, `o2_*` (wave, semi, fine, pw, level, sub, key
follow; `o2_sync`) · `noise_level` · `*_route` (per source: filter 1 | filter 2 | both), `f_serial`, `f1_to_f2` ·
`f1_*`, `f2_*` (mode LP/BP/HP/notch, cutoff, resonance, drive, key tracking, envelope amount, velocity to cutoff and to
resonance) · `fe1_*`, `fe2_*` (filter ADSR) · `a1_*`, `a2_*` (level, pan, velocity, key) · `ae1_*`, `ae2_*` (amp ADSR) ·
`l1_*`, `l2_*` (shape, rate, tempo sync, delay, attack, and the amounts to pitch / pulse width / cutoff 1 / cutoff 2 / amp 1 /
amp 2) · `vel_pitch`, `vel_pwm`, `fe2_p2`, `l1_p2`, `l2_p2` (to oscillator 2 alone – the hard-sync sweep).
The ABI's cap of 32 parameters was raised to 128 for this module (a limit in the checks, not in the `Io` layout).

## Presets

Four factory presets – `pad`, `bass`, `lead`, `sync lead` – are data in `data/presets.json`, in the wasm's custom section
**`deck.presets`**: `{"presets": [{"name", "doc", "values": {parameter: value, …}}]}`. A preset lists only what differs from the
defaults. **To apply one**: set every parameter to its default (the descriptor has them), then each of the preset's values with
`dm_set_param(index, value)` (the index is the parameter's place in the descriptor; hosts: `Instance::set_param`). The tests do
exactly that (`tests/common`).

## The DSP (`src/dsp`, plain Rust, no SDK)

- `osc` – saw, pulse (width), triangle, sine; PolyBLEP steps and PolyBLAMP corners, **hard sync as a band-limited step** (a one-sample
  output delay lets the pre-step half of the correction in), the sub as a flip-flop on the parent's wraps (an exact octave).
- `svf` – trapezoidal state-variable filter (LP / BP / HP / notch), resonance Q 0.7 … 24, a drive crossfade.
- `adsr` – linear attack, exponential decay and release that take exactly their time (0.1 % left at the end).
- `lfo` – sine, triangle, saw, square, seeded sample & hold; free (Hz) or locked to the tempo map's beat; delay, attack.
- `pan` – constant power (−3 dB at the centre); `noise` – seeded xorshift64*; `math` – `exp2`, `log2`, `sin`, `tan` from
  `+ − × ÷ floor` only, so no platform libm is on the audio path (ENGINE-REFACTOR-PLAN §3.4) and no dependency is added.

Modulation is read every sample; pitch, cutoffs and pan are recomputed every 16 samples of the voice's own clock, so a render does
not depend on how the host cuts the audio into blocks (tested). No allocation on the audio path (`tests/no_alloc.rs`).
