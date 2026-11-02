//! The synth's parameters: ONE table, expanded into the module struct (the derive reads each `range` / `default`), the
//! `Patch` snapshot the voices read, the name list, and the unit / choices / doc metadata that goes into the wasm next to
//! the descriptor (custom section `deck.params`, until the catalog descriptor carries units and docs: #315).
//!
//! Row: `name: min .. max = default, "unit", "choices", "doc";`. A switch or a mode is a number: 0 / 1, or 0..n with the
//! `choices` in order (`|`-separated). Names are at most 24 lowercase characters (the ABI).
use crate::engine::Engine;
use deck_module::Module;

macro_rules! analog {
    (
        $fname:ident: $fmin:literal .. $fmax:literal = $fdef:literal, $funit:literal, $fchoices:literal, $fdoc:literal;
        $( $name:ident: $min:literal .. $max:literal = $def:literal, $unit:literal, $choices:literal, $doc:literal; )*
    ) => {
        /// the synth: parameters (the table above), the voices behind them
        #[derive(Module)]
        #[module(name = "analog")]
        #[port(input, audio, name = "cv1")]
        #[port(input, audio, name = "cv2")]
        #[port(input, audio, name = "cv3")]
        #[port(input, audio, name = "cv4")]
        #[port(input, control, name = "sustain")]
        #[port(output, audio, name = "out")]
        pub struct Analog {
            #[param(range = $fmin..$fmax, default = $fdef)]
            pub $fname: f32,
            $( #[param(range = $min..$max, default = $def)] pub $name: f32, )*
            pub(crate) engine: Engine,
        }

        /// every parameter's value at one moment: what the voices read
        #[derive(Debug, Clone, Copy)]
        pub struct Patch {
            pub $fname: f32,
            $( pub $name: f32, )*
        }

        impl Analog {
            pub fn patch(&self) -> Patch {
                Patch { $fname: self.$fname, $( $name: self.$name, )* }
            }
        }

        /// the parameter names in descriptor order
        pub const PARAM_NAMES: &[&str] = &[stringify!($fname), $( stringify!($name), )*];

        /// `[{"name", "unit", "choices", "doc"}, …]` in descriptor order
        pub const PARAMS_JSON: &str = concat!(
            "[",
            concat!("{\"name\":\"", stringify!($fname), "\",\"unit\":\"", $funit, "\",\"choices\":\"", $fchoices, "\",\"doc\":\"", $fdoc, "\"}"),
            $( ",", concat!("{\"name\":\"", stringify!($name), "\",\"unit\":\"", $unit, "\",\"choices\":\"", $choices, "\",\"doc\":\"", $doc, "\"}"), )*
            "]"
        );
    };
}

analog! {
    volume: 0.0..1.0 = 0.7, "", "", "master level: the sum of the voices times this";

    o1_wave: 0.0..3.0 = 0.0, "", "saw|pulse|triangle|sine", "oscillator 1 waveform";
    o1_semi: -24.0..24.0 = 0.0, "st", "", "oscillator 1 pitch offset in semitones";
    o1_fine: -100.0..100.0 = 0.0, "cent", "", "oscillator 1 fine detune";
    o1_pw: 0.05..0.95 = 0.5, "", "", "oscillator 1 pulse width (the pulse wave only)";
    o1_level: 0.0..1.0 = 0.8, "", "", "oscillator 1 level";
    o1_sub: 0.0..1.0 = 0.0, "", "", "oscillator 1 sub-oscillator level (a square, one octave down)";
    o1_key: 0.0..1.0 = 1.0, "", "", "oscillator 1 keyboard follow: 1 plays the pitch it is sent, 0 holds middle C";

    o2_wave: 0.0..3.0 = 0.0, "", "saw|pulse|triangle|sine", "oscillator 2 waveform";
    o2_semi: -24.0..24.0 = 0.0, "st", "", "oscillator 2 pitch offset in semitones";
    o2_fine: -100.0..100.0 = 0.0, "cent", "", "oscillator 2 fine detune";
    o2_pw: 0.05..0.95 = 0.5, "", "", "oscillator 2 pulse width (the pulse wave only)";
    o2_level: 0.0..1.0 = 0.0, "", "", "oscillator 2 level";
    o2_sub: 0.0..1.0 = 0.0, "", "", "oscillator 2 sub-oscillator level (a square, one octave down)";
    o2_key: 0.0..1.0 = 1.0, "", "", "oscillator 2 keyboard follow: 1 plays the pitch it is sent, 0 holds middle C";
    o2_sync: 0.0..1.0 = 0.0, "", "off|on", "hard sync: oscillator 1 restarts oscillator 2 at every cycle";

    noise_level: 0.0..1.0 = 0.0, "", "", "white noise level (seeded, the same on every deck)";

    o1_route: 0.0..2.0 = 0.0, "", "filter 1|filter 2|both", "where oscillator 1 goes";
    sub1_route: 0.0..2.0 = 0.0, "", "filter 1|filter 2|both", "where oscillator 1's sub goes";
    o2_route: 0.0..2.0 = 0.0, "", "filter 1|filter 2|both", "where oscillator 2 goes";
    sub2_route: 0.0..2.0 = 0.0, "", "filter 1|filter 2|both", "where oscillator 2's sub goes";
    noise_route: 0.0..2.0 = 0.0, "", "filter 1|filter 2|both", "where the noise goes";
    f_serial: 0.0..1.0 = 0.0, "", "parallel|serial", "serial: filter 1's output also feeds filter 2";
    f1_to_f2: 0.0..1.0 = 1.0, "", "", "how much of filter 1 feeds filter 2 (serial only)";

    vel_pitch: -12.0..12.0 = 0.0, "st", "", "velocity to pitch: semitones at full velocity";
    vel_pwm: -0.45..0.45 = 0.0, "", "", "velocity to pulse width of both oscillators at full velocity";

    f1_mode: 0.0..3.0 = 0.0, "", "lowpass|bandpass|highpass|notch", "filter 1 type";
    f1_cutoff: 20.0..20000.0 = 8000.0, "Hz", "", "filter 1 cutoff";
    f1_res: 0.0..1.0 = 0.0, "", "", "filter 1 resonance";
    f1_drive: 0.0..1.0 = 0.0, "", "", "filter 1 drive: saturation in front of the filter";
    f1_key: -1.0..1.0 = 0.0, "", "", "filter 1 key tracking: 1 moves the cutoff an octave per octave of pitch";
    f1_env: -1.0..1.0 = 0.0, "", "", "filter 1 envelope amount: six octaves at full";
    f1_vel: 0.0..1.0 = 0.0, "", "", "velocity to filter 1 cutoff: four octaves at full velocity";
    f1_velres: 0.0..1.0 = 0.0, "", "", "velocity to filter 1 resonance";

    f2_mode: 0.0..3.0 = 0.0, "", "lowpass|bandpass|highpass|notch", "filter 2 type";
    f2_cutoff: 20.0..20000.0 = 8000.0, "Hz", "", "filter 2 cutoff";
    f2_res: 0.0..1.0 = 0.0, "", "", "filter 2 resonance";
    f2_drive: 0.0..1.0 = 0.0, "", "", "filter 2 drive: saturation in front of the filter";
    f2_key: -1.0..1.0 = 0.0, "", "", "filter 2 key tracking: 1 moves the cutoff an octave per octave of pitch";
    f2_env: -1.0..1.0 = 0.0, "", "", "filter 2 envelope amount: six octaves at full";
    f2_vel: 0.0..1.0 = 0.0, "", "", "velocity to filter 2 cutoff: four octaves at full velocity";
    f2_velres: 0.0..1.0 = 0.0, "", "", "velocity to filter 2 resonance";

    fe1_a: 0.001..10.0 = 0.005, "s", "", "filter envelope 1 attack";
    fe1_d: 0.001..10.0 = 0.3, "s", "", "filter envelope 1 decay";
    fe1_s: 0.0..1.0 = 0.5, "", "", "filter envelope 1 sustain level";
    fe1_r: 0.001..10.0 = 0.3, "s", "", "filter envelope 1 release";
    fe2_a: 0.001..10.0 = 0.005, "s", "", "filter envelope 2 attack";
    fe2_d: 0.001..10.0 = 0.3, "s", "", "filter envelope 2 decay";
    fe2_s: 0.0..1.0 = 0.5, "", "", "filter envelope 2 sustain level";
    fe2_r: 0.001..10.0 = 0.3, "s", "", "filter envelope 2 release";
    fe2_p2: -24.0..24.0 = 0.0, "st", "", "filter envelope 2 to oscillator 2 pitch only: semitones at full (the classic sync sweep)";

    a1_level: 0.0..1.0 = 0.8, "", "", "amp 1 level (filter 1's output)";
    a1_pan: -1.0..1.0 = 0.0, "", "", "amp 1 pan, constant power";
    a1_vel: 0.0..1.0 = 0.5, "", "", "velocity to amp 1 level";
    a1_key: -1.0..1.0 = 0.0, "", "", "amp 1 key tracking: level change with pitch, 3 dB an octave at full";
    a2_level: 0.0..1.0 = 0.8, "", "", "amp 2 level (filter 2's output)";
    a2_pan: -1.0..1.0 = 0.0, "", "", "amp 2 pan, constant power";
    a2_vel: 0.0..1.0 = 0.5, "", "", "velocity to amp 2 level";
    a2_key: -1.0..1.0 = 0.0, "", "", "amp 2 key tracking: level change with pitch, 3 dB an octave at full";

    ae1_a: 0.001..10.0 = 0.003, "s", "", "amp envelope 1 attack";
    ae1_d: 0.001..10.0 = 0.3, "s", "", "amp envelope 1 decay";
    ae1_s: 0.0..1.0 = 0.8, "", "", "amp envelope 1 sustain level";
    ae1_r: 0.001..10.0 = 0.25, "s", "", "amp envelope 1 release";
    ae2_a: 0.001..10.0 = 0.003, "s", "", "amp envelope 2 attack";
    ae2_d: 0.001..10.0 = 0.3, "s", "", "amp envelope 2 decay";
    ae2_s: 0.0..1.0 = 0.8, "", "", "amp envelope 2 sustain level";
    ae2_r: 0.001..10.0 = 0.25, "s", "", "amp envelope 2 release";

    l1_shape: 0.0..4.0 = 0.0, "", "sine|triangle|saw|square|sample & hold", "LFO 1 shape";
    l1_rate: 0.01..40.0 = 4.0, "Hz", "", "LFO 1 rate when free-running";
    l1_sync: 0.0..7.0 = 0.0, "", "free|2 bars|1 bar|1/2|1/4|1/8|1/16|1/32", "LFO 1 tempo sync: a cycle per note value, locked to the beat";
    l1_delay: 0.0..5.0 = 0.0, "s", "", "LFO 1 delay: silent for this long after the note starts";
    l1_attack: 0.0..5.0 = 0.0, "s", "", "LFO 1 attack: fades in over this long after the delay";
    l1_pitch: 0.0..12.0 = 0.0, "st", "", "LFO 1 to pitch of both oscillators: semitones either way (vibrato)";
    l1_pwm: 0.0..0.45 = 0.0, "", "", "LFO 1 to the pulse width of both oscillators";
    l1_f1: 0.0..4.0 = 0.0, "oct", "", "LFO 1 to filter 1 cutoff: octaves either way";
    l1_f2: 0.0..4.0 = 0.0, "oct", "", "LFO 1 to filter 2 cutoff: octaves either way";
    l1_a1: 0.0..1.0 = 0.0, "", "", "LFO 1 to amp 1 level (tremolo depth)";
    l1_a2: 0.0..1.0 = 0.0, "", "", "LFO 1 to amp 2 level (tremolo depth)";
    l1_p2: 0.0..12.0 = 0.0, "st", "", "LFO 1 to oscillator 2 pitch only: semitones either way (sweeps a synced oscillator)";

    l2_shape: 0.0..4.0 = 0.0, "", "sine|triangle|saw|square|sample & hold", "LFO 2 shape";
    l2_rate: 0.01..40.0 = 4.0, "Hz", "", "LFO 2 rate when free-running";
    l2_sync: 0.0..7.0 = 0.0, "", "free|2 bars|1 bar|1/2|1/4|1/8|1/16|1/32", "LFO 2 tempo sync: a cycle per note value, locked to the beat";
    l2_delay: 0.0..5.0 = 0.0, "s", "", "LFO 2 delay: silent for this long after the note starts";
    l2_attack: 0.0..5.0 = 0.0, "s", "", "LFO 2 attack: fades in over this long after the delay";
    l2_pitch: 0.0..12.0 = 0.0, "st", "", "LFO 2 to pitch of both oscillators: semitones either way (vibrato)";
    l2_pwm: 0.0..0.45 = 0.0, "", "", "LFO 2 to the pulse width of both oscillators";
    l2_f1: 0.0..4.0 = 0.0, "oct", "", "LFO 2 to filter 1 cutoff: octaves either way";
    l2_f2: 0.0..4.0 = 0.0, "oct", "", "LFO 2 to filter 2 cutoff: octaves either way";
    l2_a1: 0.0..1.0 = 0.0, "", "", "LFO 2 to amp 1 level (tremolo depth)";
    l2_a2: 0.0..1.0 = 0.0, "", "", "LFO 2 to amp 2 level (tremolo depth)";
    l2_p2: 0.0..12.0 = 0.0, "st", "", "LFO 2 to oscillator 2 pitch only: semitones either way (sweeps a synced oscillator)";
}
