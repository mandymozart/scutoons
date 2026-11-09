//! The built-in catalog (#315): the one table of every node kind – ports with types and channels, parameters with range,
//! curve, unit, default and doc. deck-space's `Kind` (ports, domain, parameter specs), deck-core's `KIND_TABLES` and
//! `STAGE_MODULES`, and the batch evaluator's kind list all read it. The order of the `graph` kinds is the order of
//! deck-space's `Kind::ALL` (the generated server file keeps it). A new kind = a variant in `Kind` + an entry here + its
//! block; a test in deck-space checks both lists agree.
use crate::{AnyChannels, Channels, Curve, Descriptor, Dir, Domain, Mode, ParamDesc, PortDesc, PortType, Scope, EVENT_CAPACITY};
use std::borrow::Cow;
use std::sync::OnceLock;

const A: PortType = PortType::Audio;
const C: PortType = PortType::Control;
const V: PortType = PortType::Video;
const E: PortType = PortType::Event;

/// doc strings are data for `kinds.json`, the editor and the agents: only the `docs` feature (the generator, native
/// builds) carries them – a wasm build keeps the table's shape without the text
#[inline(always)]
fn d(doc: &'static str) -> &'static str {
    if cfg!(feature = "docs") {
        doc
    } else {
        ""
    }
}

fn port(name: &'static str, dir: Dir, ty: PortType, channels: Option<Channels>, engine: bool, doc: &'static str) -> PortDesc {
    PortDesc {
        name: Cow::Borrowed(name),
        dir,
        ty,
        channels,
        engine,
        doc: Cow::Borrowed(d(doc)),
        sidechain: false,
        names: Cow::Borrowed(&[]),
        props: Cow::Borrowed(&[]),
        capacity: EVENT_CAPACITY,
        path: Cow::Borrowed(""),
        mode: Mode::Set,
        choices: Cow::Borrowed(&[]),
    }
}
/// a control input (its parameter has the same name: `lin` / `log` / `driven`)
fn ci(name: &'static str, doc: &'static str) -> PortDesc {
    port(name, Dir::In, C, None, true, doc)
}
fn co(name: &'static str, doc: &'static str) -> PortDesc {
    port(name, Dir::Out, C, None, true, doc)
}
/// an event output (#430: an event cable's source)
fn eo(name: &'static str, doc: &'static str) -> PortDesc {
    port(name, Dir::Out, E, None, true, doc)
}
fn vi(name: &'static str, doc: &'static str) -> PortDesc {
    port(name, Dir::In, V, None, true, doc)
}
fn vo(name: &'static str, doc: &'static str) -> PortDesc {
    port(name, Dir::Out, V, None, true, doc)
}
/// audio ports carry the engine's stereo block unless they say otherwise (`mono`, `any`)
const STEREO: Option<Channels> = Some(Channels::Count(2));
fn ai(name: &'static str, doc: &'static str) -> PortDesc {
    port(name, Dir::In, A, STEREO, true, doc)
}
fn ao(name: &'static str, doc: &'static str) -> PortDesc {
    port(name, Dir::Out, A, STEREO, true, doc)
}
/// a mono audio input (a panner's source)
fn a1i(name: &'static str, doc: &'static str) -> PortDesc {
    port(name, Dir::In, A, Some(Channels::Count(1)), true, doc)
}
/// audio ports of any width: they carry what the links carry (plan 2.6, the widest linked input)
const ANY: Option<Channels> = Some(Channels::Any(AnyChannels::Any));
fn aanyi(name: &'static str, doc: &'static str) -> PortDesc {
    port(name, Dir::In, A, ANY, true, doc)
}
fn aanyo(name: &'static str, doc: &'static str) -> PortDesc {
    port(name, Dir::Out, A, ANY, true, doc)
}
/// an audio input deck-core's control graph does not take (the oscillators' `cv`)
fn ai_graph(name: &'static str, doc: &'static str) -> PortDesc {
    port(name, Dir::In, A, STEREO, false, doc)
}

fn param(name: &'static str, curve: Curve, min: f32, max: f32, default: Option<f32>, unit: &'static str, doc: &'static str) -> ParamDesc {
    ParamDesc { name: Cow::Borrowed(name), min, max, default, curve, unit: Cow::Borrowed(unit), choices: Cow::Borrowed(&[]), port: true, doc: Cow::Borrowed(d(doc)) }
}
fn lin(name: &'static str, min: f32, max: f32, default: f32, unit: &'static str, doc: &'static str) -> ParamDesc {
    param(name, Curve::Lin, min, max, Some(default), unit, doc)
}
fn log(name: &'static str, min: f32, max: f32, default: f32, unit: &'static str, doc: &'static str) -> ParamDesc {
    param(name, Curve::Log, min, max, Some(default), unit, doc)
}
/// a knob that is not driven until a param or a link sets it
fn driven(name: &'static str, min: f32, max: f32, doc: &'static str) -> ParamDesc {
    param(name, Curve::Lin, min, max, None, "", doc)
}
/// a string parameter (no port)
fn choice(name: &'static str, choices: &[Cow<'static, str>], doc: &'static str) -> ParamDesc {
    ParamDesc {
        name: Cow::Borrowed(name),
        min: 0.0,
        max: 0.0,
        default: None,
        curve: Curve::Lin,
        unit: Cow::Borrowed(""),
        choices: Cow::Owned(choices.to_vec()),
        port: false,
        doc: Cow::Borrowed(d(doc)),
    }
}
/// a number parameter without a port
fn knob(name: &'static str, min: f32, max: f32, default: f32, unit: &'static str, doc: &'static str) -> ParamDesc {
    ParamDesc { port: false, ..lin(name, min, max, default, unit, doc) }
}

fn s(v: &'static str) -> Cow<'static, str> {
    Cow::Borrowed(v)
}

const F_SCENE: u8 = 1;
const F_BATCH: u8 = 2;
const F_INTERNAL: u8 = 4;
const F_PLANNED: u8 = 8;
const F_TRANSPORT: u8 = 16;

/// a kind; `flags`: `F_SCENE` (a scene owns it once, never inside an instrument), `F_BATCH` (the batch evaluator runs
/// it), `F_INTERNAL` / `F_PLANNED` (its scope; none = a `graph` kind), `F_TRANSPORT` (it follows the transport, #359)
#[allow(clippy::too_many_arguments)]
fn kind(
    flags: u8,
    name: &'static str,
    domain: Domain,
    feature: &'static str,
    module: &'static str,
    doc: &'static str,
    ports: &[PortDesc],
    params: &[ParamDesc],
) -> Descriptor {
    Descriptor {
        name: Cow::Borrowed(name),
        version: Cow::Borrowed("1"),
        domain,
        doc: Cow::Borrowed(d(doc)),
        feature: Cow::Borrowed(feature),
        module: Cow::Borrowed(module),
        scope: if flags & F_INTERNAL != 0 {
            Scope::Internal
        } else if flags & F_PLANNED != 0 {
            Scope::Planned
        } else {
            Scope::Graph
        },
        scene_only: flags & F_SCENE != 0,
        batch: flags & F_BATCH != 0,
        follow_transport: flags & F_TRANSPORT != 0,
        needs: Cow::Borrowed(&[]),
        wants: Cow::Borrowed(&[]),
        reads: Cow::Borrowed(&[]),
        moods: Cow::Borrowed(&[]),
        abi: None,
        latency: 0,
        ports: Cow::Owned(ports.to_vec()),
        params: Cow::Owned(params.to_vec()),
        assets: Cow::Borrowed(&[]),
        preset: None,
    }
}

use Domain::{Audio, Control, Video};

/// every built-in kind, in `Kind::ALL` order, then the batch helpers and the planned kinds
pub fn builtin_kinds() -> &'static [Descriptor] {
    static TABLE: OnceLock<Vec<Descriptor>> = OnceLock::new();
    TABLE.get_or_init(build)
}

fn build() -> Vec<Descriptor> {
    vec![
    kind(
        0,
        "test_pattern",
        Video,
        "core",
        "stage",
        "Procedural colour bars, rings and a grid – the first picture a deck draws.",
        &[ci("speed", "animation speed"), ci("zoom", "zoom of the pattern"), ci("hue", "colour rotation"), ci("pulse", "a kick that brightens the pattern"), vo("out", "the picture")],
        &[lin("speed", 0.0, 4.0, 1.0, "", "animation speed"), log("zoom", 0.25, 4.0, 1.0, "ratio", "zoom of the pattern"), lin("hue", 0.0, 1.0, 0.0, "", "colour rotation, 0..1 turns the wheel once"), lin("pulse", 0.0, 1.0, 0.0, "", "a kick that brightens the pattern")],
    ),
    kind(
        0,
        "shader",
        Video,
        "core",
        "stage",
        "A fragment shader over its input: a built-in effect or a WGSL file.",
        &[vi("in", "the picture to process"), ci("a", "shader knob a"), ci("b", "shader knob b"), ci("c", "shader knob c"), ci("d", "shader knob d, a trigger by convention"), vo("out", "the processed picture")],
        &[lin("a", 0.0, 1.0, 0.5, "", "shader knob a"), lin("b", 0.0, 1.0, 0.5, "", "shader knob b"), lin("c", 0.0, 1.0, 0.5, "", "shader knob c"), lin("d", 0.0, 1.0, 0.0, "", "shader knob d, a trigger by convention")],
    ),
    kind(
        F_SCENE,
        "screen",
        Video,
        "core",
        "stage",
        "The window: the video sink. A scene has one.",
        &[vi("in", "the picture shown")],
        &[],
    ),
    kind(
        0,
        "chapter",
        Video,
        "chapter",
        "stage",
        "A chapter's slices from the library API, one after the other with its dialog set in, over `in` (or a dark, stretched copy of the slice).",
        &[vi("in", "backdrop"), ci("next", "step to the next slice"), ci("hold", "seconds a slice stays"), ci("zoom", "slow zoom"), ci("pulse", "a kick"), vo("out", "slices with dialog"), vo("text", "the current slice's caption on transparent, for an overlay")],
        &[lin("next", 0.0, 1.0, 0.0, "", "a rising edge steps to the next slice"), log("hold", 0.5, 30.0, 6.0, "s", "seconds a slice stays"), log("zoom", 0.25, 4.0, 1.0, "ratio", "slow zoom"), lin("pulse", 0.0, 1.0, 0.0, "", "a kick")],
    ),
    kind(
        0,
        "overlay",
        Video,
        "core",
        "stage",
        "`top` over `in` (alpha), the top popping in when it changes, shaken by `shake`.",
        &[vi("in", "the bottom picture"), vi("top", "the picture laid over it"), ci("opacity", "opacity of the top"), ci("pulse", "a kick"), ci("shake", "shakes the top"), vo("out", "the composite")],
        &[lin("opacity", 0.0, 1.0, 1.0, "", "opacity of the top"), lin("pulse", 0.0, 1.0, 0.0, "", "a kick"), lin("shake", 0.0, 1.0, 0.0, "", "shakes the top")],
    ),
    kind(
        F_SCENE,
        "space",
        Video,
        "space",
        "stage",
        "A 3D space: a set's glTF and the deck's being (the running graph as a floating body) seen by an orbit camera; its inputs colour the being, light the set, kick its breath and drive the post stack.",
        &[
            ci("hue", "being colour"),
            ci("glow", "being glow"),
            ci("light", "set light"),
            ci("pulse", "kicks the core's light"),
            ci("bounce", "moves the body: wire the beat or the level to it"),
            ci("bloom", "bloom amount"),
            ci("threshold", "bloom threshold"),
            ci("lift", "curves: lift"),
            ci("gamma", "curves: gamma"),
            ci("gain", "curves: gain"),
            ci("pixel", "pixelation block size in render pixels"),
            ci("levels", "quantise levels per channel (< 2 = off)"),
            vo("out", "the rendered space"),
        ],
        &[
            lin("hue", 0.0, 1.0, 0.52, "", "being colour"),
            lin("glow", 0.0, 2.0, 0.6, "", "being glow"),
            lin("light", 0.0, 1.5, 0.85, "", "set light"),
            lin("pulse", 0.0, 1.0, 0.0, "", "kicks the core's light"),
            lin("bounce", 0.0, 1.0, 0.0, "", "moves the body: wire the beat or the level to it"),
            lin("bloom", 0.0, 2.0, 0.6, "", "bloom amount"),
            lin("threshold", 0.0, 1.0, 0.6, "", "bloom threshold"),
            lin("lift", -0.2, 0.3, 0.0, "", "curves: lift"),
            log("gamma", 0.4, 2.5, 1.0, "", "curves: gamma"),
            lin("gain", 0.5, 2.0, 1.0, "", "curves: gain"),
            log("pixel", 1.0, 64.0, 1.0, "px", "pixelation block size in render pixels"),
            lin("levels", 0.0, 16.0, 0.0, "", "quantise levels per channel (< 2 = off)"),
        ],
    ),
    kind(
        F_SCENE,
        "outlet",
        Video,
        "live",
        "stage",
        "A Live outlet: this deck's cells from the relay, drawn natively as one plane over `in` (the backdrop), cross-faded on each cell change.",
        &[vi("in", "the backdrop"), ci("opacity", "dims the outlet"), vo("out", "backdrop with the outlet's cells")],
        &[lin("opacity", 0.0, 1.0, 1.0, "", "dims the outlet")],
    ),
    kind(
        F_BATCH,
        "lfo",
        Control,
        "core",
        "control",
        "A low-frequency oscillator (control rate: stepped once per 128-frame engine block).",
        &[ci("rate", "cycles per second"), ci("depth", "amplitude"), co("out", "the wave, 0..depth")],
        &[
            log("rate", 0.01, 20.0, 0.25, "Hz", "cycles per second"),
            lin("depth", 0.0, 2.0, 1.0, "", "amplitude"),
            choice("shape", &[s("sine"), s("saw"), s("square")], "the wave (default sine)"),
            knob("phase", 0.0, 1.0, 0.0, "", "where the wave starts (0..1 of a cycle)"),
        ],
    ),
    kind(
        F_SCENE,
        "tempo",
        Control,
        "core",
        "stage",
        "The tempo clock, one per scene: its params set the clock (bpm, beats per bar, the source, the click, MIDI / OSC ports).",
        &[
            co("bpm", "beats per minute"),
            co("beat", "1 on the beat"),
            co("bar", "1 on the bar's first beat"),
            co("phase", "position in the beat, 0..1"),
            co("bar_phase", "position in the bar, 0..1"),
            co("pulse", "a decaying kick on each beat"),
        ],
        &[],
    ),
    kind(
        F_TRANSPORT,
        "click",
        Audio,
        "core",
        "stage",
        "A metronome on the tempo clock: a short tick per beat, higher on the bar's first (while the click is on).",
        &[ci("gain", "level"), ao("out", "the ticks")],
        &[lin("gain", 0.0, 1.0, 1.0, "", "level")],
    ),
    kind(
        F_BATCH,
        "osc",
        Audio,
        "core",
        "synth",
        "A band-limited oscillator, pitched by its `cv` input or `freq`.",
        &[ai_graph("cv", "pitch in Hz on the left channel (a seq / walk / events output); overrides `freq`"), ci("freq", "pitch"), ci("gain", "level"), ao("out", "the wave")],
        &[
            log("freq", 20.0, 4000.0, 220.0, "Hz", "pitch"),
            lin("gain", 0.0, 1.0, 1.0, "", "level"),
            choice("wave", &[s("sine"), s("saw"), s("square"), s("triangle")], "the wave (default sine)"),
        ],
    ),
    kind(
        0,
        "noise",
        Audio,
        "core",
        "stage",
        "White or pink noise through a lowpass.",
        &[ci("cutoff", "lowpass cutoff"), ci("gain", "level"), ao("out", "the noise")],
        &[
            log("cutoff", 50.0, 16000.0, 4000.0, "Hz", "lowpass cutoff"),
            lin("gain", 0.0, 1.0, 1.0, "", "level"),
            choice("color", &[s("white"), s("pink")], "the noise colour (default white)"),
        ],
    ),
    kind(
        F_BATCH,
        "fm",
        Audio,
        "core",
        "synth",
        "Two-operator FM.",
        &[
            ai_graph("cv", "pitch in Hz on the left channel (a seq / walk / events output); overrides `freq`"),
            ci("freq", "carrier pitch"),
            ci("ratio", "modulator frequency / carrier frequency"),
            ci("index", "modulation depth"),
            ci("gain", "level"),
            ao("out", "the wave"),
        ],
        &[
            log("freq", 20.0, 4000.0, 220.0, "Hz", "carrier pitch"),
            log("ratio", 0.25, 8.0, 2.0, "ratio", "modulator frequency / carrier frequency"),
            lin("index", 0.0, 10.0, 1.0, "", "modulation depth"),
            lin("gain", 0.0, 1.0, 1.0, "", "level"),
        ],
    ),
    kind(
        F_TRANSPORT,
        "seq",
        Audio,
        "core",
        "stage",
        "A step sequencer: `cv` = pitch in Hz on the left channel, gate 0 / 1 on the right (modular CV in an audio port).",
        &[ci("rate", "speed multiplier"), ao("cv", "pitch (left) and gate (right)")],
        &[log("rate", 0.25, 4.0, 1.0, "ratio", "speed multiplier")],
    ),
    kind(
        F_TRANSPORT,
        "walk",
        Audio,
        "events",
        "stage",
        "A generated melody (deck-events): a seeded random walk on a continuous pitch scale on the tempo clock, written as seq's cv. The same seed plays the same melody on every deck.",
        &[ci("spread", "0 = the base held, 1 = ± `octaves`"), ci("rate", "steps a beat"), ci("glide", "glide into each step, rides along for the voice"), ao("cv", "pitch (left) and gate (right)")],
        &[lin("spread", 0.0, 1.0, 0.0, "", "0 = the base held, 1 = ± `octaves`"), log("rate", 0.25, 8.0, 2.0, "ratio", "steps a beat"), lin("glide", 0.0, 1.0, 0.5, "", "glide into each step, rides along for the voice")],
    ),
    kind(
        F_TRANSPORT,
        "events",
        Audio,
        "events",
        "stage",
        "Generators rigged together (deck-events' patch): its params hold the patch (`generator`, `edge`, `out`, `knobs`) – an arp on a walk, a pulse … –, inputs a–d are the patch's knobs (unset = the patch's own values); its notes are written as seq's cv.",
        &[ci("a", "knob a"), ci("b", "knob b"), ci("c", "knob c"), ci("d", "knob d"), ao("cv", "pitch (left) and gate (right)")],
        &[
            driven("a", 0.0, 1.0, "raw value into the patch's knob a; the instrument's `expose.range` gives it a real range"),
            driven("b", 0.0, 1.0, "raw value into the patch's knob b"),
            driven("c", 0.0, 1.0, "raw value into the patch's knob c"),
            driven("d", 0.0, 1.0, "raw value into the patch's knob d"),
        ],
    ),
    kind(
        0,
        "chapter_music",
        Audio,
        "chapter",
        "stage",
        "A chapter's music track from the library API, cached and decoded.",
        &[ci("gain", "level"), ao("out", "the track")],
        &[lin("gain", 0.0, 1.0, 1.0, "", "level")],
    ),
    kind(
        0,
        "mixer",
        Audio,
        "core",
        "stage",
        "Four stereo inputs, a gain each.",
        &[ai("in1", "input 1"), ai("in2", "input 2"), ai("in3", "input 3"), ai("in4", "input 4"), ci("gain1", "gain of input 1"), ci("gain2", "gain of input 2"), ci("gain3", "gain of input 3"), ci("gain4", "gain of input 4"), ao("out", "the sum")],
        &[lin("gain1", 0.0, 1.5, 1.0, "", "gain of input 1"), lin("gain2", 0.0, 1.5, 1.0, "", "gain of input 2"), lin("gain3", 0.0, 1.5, 1.0, "", "gain of input 3"), lin("gain4", 0.0, 1.5, 1.0, "", "gain of input 4")],
    ),
    kind(
        0,
        "fft",
        Audio,
        "core",
        "analysis",
        "Sound → control values: level, bass, mid, high (0–1).",
        &[ai("in", "the sound analysed (a tap, it passes nothing on)"), co("level", "overall level"), co("bass", "low band"), co("mid", "mid band"), co("high", "high band")],
        &[knob("release", 0.0, 5.0, 0.25, "s", "how fast the values fall")],
    ),
    kind(
        F_SCENE,
        "audio_out",
        Audio,
        "core",
        "stage",
        "The device output: the audio sink. A scene has one.",
        &[ai("in", "the sound played"), ci("volume", "main volume")],
        &[lin("volume", 0.0, 1.0, 0.8, "", "main volume")],
    ),
    // routing (plan 2.6, #318): buses, outlets, panners, down-mixes, the explicit loop
    kind(
        0,
        "bus",
        Audio,
        "core",
        "stage",
        "A bus (Reaper's track): everything linked into `in` is summed, a fader (`gain`), mute and solo gate the post-fader `out`, `pre` is the sum before both. A send is a link out of `pre` or `out` into another bus, `scale` the send level, `map` the channel map (\"1/2 → 3/4\" is `map = [[1, 3], [2, 4]]`); a folder bus is a bus that sums its children. The insert chain is the nodes linked between a source and `in`. Mute and solo gate `out` only: a muted kick still ducks through its pre-fader send.",
        &[aanyi("in", "everything linked here is summed, in the scene's link order"), ci("gain", "fader"), ci("mute", "1 = out is silent (pre keeps playing)"), ci("solo", "1 = every bus not soloed is silent at its out"), aanyo("out", "post-fader: the sum × gain, gated by mute and solo"), aanyo("pre", "pre-fader: the sum itself – a pre-fader send or sidechain feed")],
        &[
            lin("gain", 0.0, 2.0, 1.0, "", "fader"),
            lin("mute", 0.0, 1.0, 0.0, "", "1 = out is silent"),
            lin("solo", 0.0, 1.0, 0.0, "", "1 = every bus not soloed is silent at its out"),
            knob("channels", 0.0, 32.0, 0.0, "", "the bus's width; 0 = the widest input"),
        ],
    ),
    kind(
        0,
        "out",
        Audio,
        "core",
        "stage",
        "An outlet (Max's dac~): writes its input to device channels. `group` names the outlet group (default main; cue …) whose device channels the deck configuration gives; `first` (counted from 1) or a `channels` list pins device channels instead; nothing set = main on channels 1.. and any other group silent. An outlet with more channels than the device has is folded by the deck configuration (the layout's down-mix, else the first channels). Several outs add up.",
        &[aanyi("in", "the sound written to the outlet"), ci("gain", "level of this out")],
        &[
            lin("gain", 0.0, 1.5, 1.0, "", "level of this out"),
            choice("group", &[s("main"), s("cue")], "the outlet group (default main); any other name is a group of its own"),
            knob("first", 0.0, 32.0, 0.0, "", "the first device channel, counted from 1; 0 = the deck configuration decides"),
            choice("layout", &[s("mono"), s("stereo"), s("quad"), s("5.1"), s("7.1")], "what the channels are, for the fold onto fewer device channels (default by count)"),
        ],
    ),
    kind(
        0,
        "pan",
        Audio,
        "core",
        "stage",
        "Places a mono source (its first channel) on a speaker layout: equal-power between two speakers, VBAP (Pulkki: the speaker pair or triple around the direction, power-normalised) or DBAP (distance-based) over any array. The output has one channel per speaker; LFE channels get nothing. Azimuth counts degrees clockwise from the front (right is positive), elevation up.",
        &[a1i("in", "the source (channel 1)"), ci("azimuth", "direction in degrees, 0 = front, positive = right"), ci("elevation", "degrees above the horizon"), ci("distance", "distance of the source for DBAP, in units of the layout's radius"), aanyo("out", "one channel per speaker")],
        &[
            lin("azimuth", -180.0, 180.0, 0.0, "deg", "direction in degrees, 0 = front, positive = right"),
            lin("elevation", -90.0, 90.0, 0.0, "deg", "degrees above the horizon"),
            lin("distance", 0.1, 10.0, 1.0, "", "distance of the source for DBAP"),
            choice("mode", &[s("auto"), s("equal_power"), s("vbap"), s("dbap")], "the panning law; auto = equal_power on two speakers, else vbap"),
            choice("layout", &[s("mono"), s("stereo"), s("quad"), s("5.1"), s("7.1")], "the speakers (default stereo); a venue's own layouts come from the deck configuration, or give `az` / `el` / `dist` lists on the node"),
            knob("rolloff", 0.0, 12.0, 6.0, "dB", "DBAP: level lost per doubling of the distance"),
            knob("blur", 0.0, 2.0, 0.1, "", "DBAP: spatial blur, keeps a source on a speaker from being a single-speaker point"),
        ],
    ),
    kind(
        0,
        "downmix",
        Audio,
        "core",
        "stage",
        "A gain matrix from `from` channels to `to` channels: the standard tables (ITU-R BS.775 / Web Audio: 5.1 → stereo is L + C/√2 + Ls/√2, the LFE dropped; quad → stereo; stereo → mono …) between the layouts, or your own `matrix` (rows = output channels, columns = input channels, row by row). Channels go n → n everywhere else; this is the one place a show asks for a mix-down.",
        &[aanyi("in", "the sound to mix (padded or cut to `from` channels)"), aanyo("out", "`to` channels")],
        &[
            choice("from", &[s("mono"), s("stereo"), s("quad"), s("5.1"), s("7.1")], "the layout of the input (default 5.1)"),
            choice("to", &[s("mono"), s("stereo"), s("quad"), s("5.1")], "the layout of the output (default stereo)"),
            knob("inputs", 0.0, 32.0, 0.0, "", "with a custom `matrix`: how many input channels a row has (0 = the `from` layout's)"),
        ],
    ),
    kind(
        0,
        "feedback",
        Audio,
        "core",
        "stage",
        "The explicit loop (Max's tapin~ / tapout~): `out` plays what came into `in` one block (128 frames, 2.7 ms) ago. Every other cycle in a graph is an error.",
        &[aanyi("in", "the sound fed back (padded or cut to `channels`)"), aanyo("out", "`in`, one block late")],
        &[knob("channels", 1.0, 32.0, 2.0, "", "how many channels the loop carries")],
    ),
    kind(
        F_TRANSPORT,
        "world",
        Audio,
        "events",
        "stage",
        "A source from outside as a node (#430): `source` says which, `listen` is its name filter – the one place in a document that names a world event; its `events` output is cabled like a module's. `midi`: the events of the MIDI adapter. `generator`: the deck-events patch in its `patch` table runs here and its notes go down its cables only. An event input with no cable hears nothing.",
        &[eo("events", "what the source sends, the names in `listen`")],
        &[choice("source", &[Cow::Borrowed("midi"), Cow::Borrowed("generator")], "where the events come from")],
    ),
    // helpers only the batch evaluator runs (control strips of the browser's lowering)
    kind(F_BATCH | F_INTERNAL, "smooth", Control, "core", "control", "Smooths its input over `time` (a control helper).", &[ci("in", "the value"), ci("time", "glide time"), co("out", "the smoothed value")], &[lin("in", -1000.0, 1000.0, 0.0, "", "the value"), log("time", 0.001, 10.0, 0.1, "s", "glide time")]),
    kind(F_BATCH | F_INTERNAL, "mul", Control, "core", "control", "Multiplies a and b (a control helper).", &[ci("a", "factor a"), ci("b", "factor b"), co("out", "a × b")], &[lin("a", -1000.0, 1000.0, 0.0, "", "factor a"), lin("b", -1000.0, 1000.0, 0.0, "", "factor b")]),
    kind(F_BATCH | F_INTERNAL, "add", Control, "core", "control", "Adds a and b (a control helper).", &[ci("a", "summand a"), ci("b", "summand b"), co("out", "a + b")], &[lin("a", -1000.0, 1000.0, 0.0, "", "summand a"), lin("b", -1000.0, 1000.0, 0.0, "", "summand b")]),
    // planned modules (CYBERDECKS.md §8, the owner's list #182)
    kind(F_PLANNED, "model3d", Video, "", "3d", "Planned: a 3D model.", &[ci("rot", "rotation"), vo("out", "the picture")], &[lin("rot", 0.0, 1.0, 0.0, "", "rotation")]),
    kind(F_PLANNED, "text", Video, "", "text", "Planned: text over a picture.", &[vi("in", "the picture"), vo("out", "the picture with text")], &[]),
    kind(F_PLANNED, "video", Video, "", "video", "Planned: a video clip.", &[ci("speed", "playback speed"), vo("out", "the picture")], &[lin("speed", 0.0, 1.0, 0.0, "", "playback speed")]),
]
}
