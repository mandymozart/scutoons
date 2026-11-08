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

Build: `cargo build --release --target wasm32-unknown-unknown -p analog-module` → `analog_module.wasm` (under 250 KB; the SDK
alone is ~100 KB). Tests: `cargo test -p analog-module` (native ≡ wasm bit for bit: `tests/wasm.rs`).

## Ports

| port | dir | what |
|---|---|---|
| `notes` | event in | **notes** (`music.note`, module ABI v2 #343): note on (pitch Hz, velocity 0…1) and note off, paired by the note's **id**; a bend per note (semitones, a note expression); a reset (all notes off). |
| `sustain` | control in | ≥ 0.5 holds released notes (the pedal); letting go releases them. |
| `out` | audio out | stereo (2 channels), the sum of the voices times `volume`. |

A note on starts a voice – or **steals the oldest** (a 1 ms fade-out first, no click) – keyed by its id, never by its pitch, so
two notes of one pitch are two voices and an off ends only its own; a note's release tail keeps sounding while the next notes
start. A note on for an id that still sounds is a **legato** pitch change of that voice; a bend moves it from the note's own
pitch. Events take effect at their frame (the render cuts the block there). Eight voices. The engine's event host delivers
notes from the deck (MIDI, generators) and from other modules – `walk`'s `notes` play it directly (deck-native's
tests/modules.rs). Until #343 notes came in as four cv lanes; they are gone.

## Parameters

91, one table (`src/params.rs`): every row is a `#[param(…)]` attribute – range or `choices`, default, `curve`, `unit`, `doc`
– that the derive reads into the descriptor (the catalog's `Descriptor` carries units, choices and docs: the `deck.params`
section of ABI v1 is gone), expanded into the module struct, the `Patch` the voices read and `PARAM_NAMES`. A switch is
0/1, a mode 0..n with its `choices` in order (`saw|pulse|triangle|sine`; the value is the index). Groups: `volume` · `o1_*`,
`o2_*` (wave, semi, fine, pw, level, sub, key follow; `o2_sync`) · `noise_level` · `*_route` (per source: filter 1 | filter
2 | both), `f_serial`, `f1_to_f2` · `f1_*`, `f2_*` (mode LP/BP/HP/notch, cutoff, resonance, drive, key tracking, envelope
amount, velocity to cutoff and to resonance) · `fe1_*`, `fe2_*` (filter ADSR) · `a1_*`, `a2_*` (level, pan, velocity, key) ·
`ae1_*`, `ae2_*` (amp ADSR) · `l1_*`, `l2_*` (shape, rate, tempo sync, delay, attack, and the amounts to pitch / pulse width /
cutoff 1 / cutoff 2 / amp 1 / amp 2) · `vel_pitch`, `vel_pwm`, `fe2_p2`, `l1_p2`, `l2_p2` (to oscillator 2 alone – the
hard-sync sweep).

## Presets

**One factory preset in the descriptor** (research/ABI-V2.md §2, owner 2026-11-03): `pad` (`#[preset]` in `src/params.rs`) –
the host's `Preset::apply` sets every parameter to its default, then its values. The other three – `bass`, `lead`,
`sync lead` – are `preset:` documents: the module is the catalog's factory document `module:analog` (#350); until preset
files import (#346) they wait in **`analog.preset.json`** (format `scutoons.preset`, version 1, `module`, `module_version`, `presets: [{name, doc, values
by parameter name}]`), the file the creator imports as drafts. A preset lists only what differs from the defaults. The tests
apply all four (`tests/common`).

## The DSP (`src/*.rs` beside `lib.rs`, plain Rust, no SDK)

Flat files on purpose (#350): the build service compiles a module as flat `src/*.rs` files, and the catalog's factory
document `module:analog` stores exactly these (migration 2026-11-05-factory-modules; the server's FactoryModulesTest keeps
the two equal). A sub-folder would make it a repository crate the service cannot build.

- `osc` – saw, pulse (width), triangle, sine; PolyBLEP steps and PolyBLAMP corners, **hard sync as a band-limited step** (a one-sample
  output delay lets the pre-step half of the correction in), the sub as a flip-flop on the parent's wraps (an exact octave).
- `svf` – trapezoidal state-variable filter (LP / BP / HP / notch), resonance Q 0.7 … 24, a drive crossfade.
- `adsr` – linear attack, exponential decay and release that take exactly their time (0.1 % left at the end).
- `lfo` – sine, triangle, saw, square, seeded sample & hold; free (Hz) or locked to the tempo map's beat; delay, attack.
- `pan` – constant power (−3 dB at the centre); `noise` – seeded xorshift64*; `math` – `exp2`, `log2`, `sin`, `tan` from
  `+ − × ÷ floor` only, so no platform libm is on the audio path (ENGINE-REFACTOR-PLAN §3.4) and no dependency is added.

Modulation is read every sample; pitch, cutoffs and pan are recomputed every 16 samples of the voice's own clock, so a render does
not depend on how the host cuts the audio into blocks (tested). No allocation on the audio path (`tests/no_alloc.rs`: the SDK's
counting allocator under the strict `Rig`).
