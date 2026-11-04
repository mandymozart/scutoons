//! The synth's parameters: ONE table – each row a `#[param]` attribute (range or choices, default, curve, unit, doc) the
//! derive reads into the descriptor (#343: the catalog descriptor carries units, choices and docs; no extra section) –
//! expanded into the module struct, the `Patch` snapshot the voices read and the name list. The factory preset `pad`
//! is the descriptor's one preset (`#[preset]`); every other preset is a document (research/ABI-V2.md §2).
//!
//! A switch or a mode is a number: 0 / 1, or 0..n with its `choices` in order (the value is the index). Names are at
//! most 24 lowercase characters.
use crate::engine::Engine;
use deck_module::Module;

macro_rules! analog {
    ( $( #[param($($meta:tt)*)] $name:ident; )* ) => {
        /// the synth: parameters (the table below), the voices behind them
        #[derive(Module)]
        #[module(name = "analog", doc = "a classic two-oscillator, two-filter, two-amp subtractive synth, eight voices")]
        #[port(input, event, name = "notes", names = "music.note", props = "pitch|vel|glide|key", capacity = 64, doc = "notes on and off (by id), a bend per note, a reset")]
        #[port(input, control, name = "sustain", doc = "the pedal: at 0.5 and above released notes hold")]
        #[port(output, audio, name = "out", channels = 2, doc = "stereo: the sum of the voices times volume")]
        #[preset(name = "pad", doc = "A wide, slow pad: a detuned saw through filter 1 on the left and a pulse with a slow pulse-width sweep through filter 2 on the right, a long filter bloom and a slow filter wobble.", values(volume = 0.6, o1_wave = 0.0, o1_fine = -7.0, o1_level = 0.7, o1_route = 0.0, o2_wave = 1.0, o2_fine = 7.0, o2_pw = 0.5, o2_level = 0.6, o2_route = 1.0, f1_cutoff = 1400.0, f1_res = 0.2, f1_key = 0.5, f1_env = 0.35, f2_cutoff = 2200.0, f2_res = 0.1, f2_key = 0.5, f2_env = 0.2, fe1_a = 1.2, fe1_d = 1.5, fe1_s = 0.5, fe1_r = 1.5, fe2_a = 1.5, fe2_d = 2.0, fe2_s = 0.6, fe2_r = 1.5, a1_level = 0.8, a1_pan = -0.55, a1_vel = 0.3, a2_level = 0.8, a2_pan = 0.55, a2_vel = 0.3, ae1_a = 0.9, ae1_d = 0.8, ae1_s = 0.85, ae1_r = 2.0, ae2_a = 1.1, ae2_d = 0.8, ae2_s = 0.85, ae2_r = 2.2, l1_shape = 0.0, l1_rate = 0.25, l1_f1 = 0.5, l2_shape = 1.0, l2_rate = 0.17, l2_pwm = 0.3, l2_f2 = 0.4))]
        pub struct Analog {
            $( #[param($($meta)*)] pub $name: f32, )*
            pub(crate) engine: Engine,
        }

        /// every parameter's value at one moment: what the voices read
        #[derive(Debug, Clone, Copy)]
        pub struct Patch {
            $( pub $name: f32, )*
        }

        impl Analog {
            pub fn patch(&self) -> Patch {
                Patch { $( $name: self.$name, )* }
            }
        }

        /// the parameter names in descriptor order
        pub const PARAM_NAMES: &[&str] = &[$( stringify!($name), )*];
    };
}

analog! {
    #[param(range = 0.0..1.0, default = 0.7, doc = "master level: the sum of the voices times this")] volume;

    #[param(choices = "saw|pulse|triangle|sine", default = 0, doc = "oscillator 1 waveform")] o1_wave;
    #[param(range = -24.0..24.0, default = 0.0, unit = "st", doc = "oscillator 1 pitch offset in semitones")] o1_semi;
    #[param(range = -100.0..100.0, default = 0.0, unit = "cent", doc = "oscillator 1 fine detune")] o1_fine;
    #[param(range = 0.05..0.95, default = 0.5, doc = "oscillator 1 pulse width (the pulse wave only)")] o1_pw;
    #[param(range = 0.0..1.0, default = 0.8, doc = "oscillator 1 level")] o1_level;
    #[param(range = 0.0..1.0, default = 0.0, doc = "oscillator 1 sub-oscillator level (a square, one octave down)")] o1_sub;
    #[param(range = 0.0..1.0, default = 1.0, doc = "oscillator 1 keyboard follow: 1 plays the pitch it is sent, 0 holds middle C")] o1_key;

    #[param(choices = "saw|pulse|triangle|sine", default = 0, doc = "oscillator 2 waveform")] o2_wave;
    #[param(range = -24.0..24.0, default = 0.0, unit = "st", doc = "oscillator 2 pitch offset in semitones")] o2_semi;
    #[param(range = -100.0..100.0, default = 0.0, unit = "cent", doc = "oscillator 2 fine detune")] o2_fine;
    #[param(range = 0.05..0.95, default = 0.5, doc = "oscillator 2 pulse width (the pulse wave only)")] o2_pw;
    #[param(range = 0.0..1.0, default = 0.0, doc = "oscillator 2 level")] o2_level;
    #[param(range = 0.0..1.0, default = 0.0, doc = "oscillator 2 sub-oscillator level (a square, one octave down)")] o2_sub;
    #[param(range = 0.0..1.0, default = 1.0, doc = "oscillator 2 keyboard follow: 1 plays the pitch it is sent, 0 holds middle C")] o2_key;
    #[param(choices = "off|on", default = 0, doc = "hard sync: oscillator 1 restarts oscillator 2 at every cycle")] o2_sync;

    #[param(range = 0.0..1.0, default = 0.0, doc = "white noise level (seeded, the same on every deck)")] noise_level;

    #[param(choices = "filter 1|filter 2|both", default = 0, doc = "where oscillator 1 goes")] o1_route;
    #[param(choices = "filter 1|filter 2|both", default = 0, doc = "where oscillator 1's sub goes")] sub1_route;
    #[param(choices = "filter 1|filter 2|both", default = 0, doc = "where oscillator 2 goes")] o2_route;
    #[param(choices = "filter 1|filter 2|both", default = 0, doc = "where oscillator 2's sub goes")] sub2_route;
    #[param(choices = "filter 1|filter 2|both", default = 0, doc = "where the noise goes")] noise_route;
    #[param(choices = "parallel|serial", default = 0, doc = "serial: filter 1's output also feeds filter 2")] f_serial;
    #[param(range = 0.0..1.0, default = 1.0, doc = "how much of filter 1 feeds filter 2 (serial only)")] f1_to_f2;

    #[param(range = -12.0..12.0, default = 0.0, unit = "st", doc = "velocity to pitch: semitones at full velocity")] vel_pitch;
    #[param(range = -0.45..0.45, default = 0.0, doc = "velocity to pulse width of both oscillators at full velocity")] vel_pwm;

    #[param(choices = "lowpass|bandpass|highpass|notch", default = 0, doc = "filter 1 type")] f1_mode;
    #[param(range = 20.0..20000.0, default = 8000.0, curve = log, unit = "Hz", doc = "filter 1 cutoff")] f1_cutoff;
    #[param(range = 0.0..1.0, default = 0.0, doc = "filter 1 resonance")] f1_res;
    #[param(range = 0.0..1.0, default = 0.0, doc = "filter 1 drive: saturation in front of the filter")] f1_drive;
    #[param(range = -1.0..1.0, default = 0.0, doc = "filter 1 key tracking: 1 moves the cutoff an octave per octave of pitch")] f1_key;
    #[param(range = -1.0..1.0, default = 0.0, doc = "filter 1 envelope amount: six octaves at full")] f1_env;
    #[param(range = 0.0..1.0, default = 0.0, doc = "velocity to filter 1 cutoff: four octaves at full velocity")] f1_vel;
    #[param(range = 0.0..1.0, default = 0.0, doc = "velocity to filter 1 resonance")] f1_velres;

    #[param(choices = "lowpass|bandpass|highpass|notch", default = 0, doc = "filter 2 type")] f2_mode;
    #[param(range = 20.0..20000.0, default = 8000.0, curve = log, unit = "Hz", doc = "filter 2 cutoff")] f2_cutoff;
    #[param(range = 0.0..1.0, default = 0.0, doc = "filter 2 resonance")] f2_res;
    #[param(range = 0.0..1.0, default = 0.0, doc = "filter 2 drive: saturation in front of the filter")] f2_drive;
    #[param(range = -1.0..1.0, default = 0.0, doc = "filter 2 key tracking: 1 moves the cutoff an octave per octave of pitch")] f2_key;
    #[param(range = -1.0..1.0, default = 0.0, doc = "filter 2 envelope amount: six octaves at full")] f2_env;
    #[param(range = 0.0..1.0, default = 0.0, doc = "velocity to filter 2 cutoff: four octaves at full velocity")] f2_vel;
    #[param(range = 0.0..1.0, default = 0.0, doc = "velocity to filter 2 resonance")] f2_velres;

    #[param(range = 0.001..10.0, default = 0.005, unit = "s", doc = "filter envelope 1 attack")] fe1_a;
    #[param(range = 0.001..10.0, default = 0.3, unit = "s", doc = "filter envelope 1 decay")] fe1_d;
    #[param(range = 0.0..1.0, default = 0.5, doc = "filter envelope 1 sustain level")] fe1_s;
    #[param(range = 0.001..10.0, default = 0.3, unit = "s", doc = "filter envelope 1 release")] fe1_r;
    #[param(range = 0.001..10.0, default = 0.005, unit = "s", doc = "filter envelope 2 attack")] fe2_a;
    #[param(range = 0.001..10.0, default = 0.3, unit = "s", doc = "filter envelope 2 decay")] fe2_d;
    #[param(range = 0.0..1.0, default = 0.5, doc = "filter envelope 2 sustain level")] fe2_s;
    #[param(range = 0.001..10.0, default = 0.3, unit = "s", doc = "filter envelope 2 release")] fe2_r;
    #[param(range = -24.0..24.0, default = 0.0, unit = "st", doc = "filter envelope 2 to oscillator 2 pitch only: semitones at full (the classic sync sweep)")] fe2_p2;

    #[param(range = 0.0..1.0, default = 0.8, doc = "amp 1 level (filter 1's output)")] a1_level;
    #[param(range = -1.0..1.0, default = 0.0, doc = "amp 1 pan, constant power")] a1_pan;
    #[param(range = 0.0..1.0, default = 0.5, doc = "velocity to amp 1 level")] a1_vel;
    #[param(range = -1.0..1.0, default = 0.0, doc = "amp 1 key tracking: level change with pitch, 3 dB an octave at full")] a1_key;
    #[param(range = 0.0..1.0, default = 0.8, doc = "amp 2 level (filter 2's output)")] a2_level;
    #[param(range = -1.0..1.0, default = 0.0, doc = "amp 2 pan, constant power")] a2_pan;
    #[param(range = 0.0..1.0, default = 0.5, doc = "velocity to amp 2 level")] a2_vel;
    #[param(range = -1.0..1.0, default = 0.0, doc = "amp 2 key tracking: level change with pitch, 3 dB an octave at full")] a2_key;

    #[param(range = 0.001..10.0, default = 0.003, unit = "s", doc = "amp envelope 1 attack")] ae1_a;
    #[param(range = 0.001..10.0, default = 0.3, unit = "s", doc = "amp envelope 1 decay")] ae1_d;
    #[param(range = 0.0..1.0, default = 0.8, doc = "amp envelope 1 sustain level")] ae1_s;
    #[param(range = 0.001..10.0, default = 0.25, unit = "s", doc = "amp envelope 1 release")] ae1_r;
    #[param(range = 0.001..10.0, default = 0.003, unit = "s", doc = "amp envelope 2 attack")] ae2_a;
    #[param(range = 0.001..10.0, default = 0.3, unit = "s", doc = "amp envelope 2 decay")] ae2_d;
    #[param(range = 0.0..1.0, default = 0.8, doc = "amp envelope 2 sustain level")] ae2_s;
    #[param(range = 0.001..10.0, default = 0.25, unit = "s", doc = "amp envelope 2 release")] ae2_r;

    #[param(choices = "sine|triangle|saw|square|sample & hold", default = 0, doc = "LFO 1 shape")] l1_shape;
    #[param(range = 0.01..40.0, default = 4.0, curve = log, unit = "Hz", doc = "LFO 1 rate when free-running")] l1_rate;
    #[param(choices = "free|2 bars|1 bar|1/2|1/4|1/8|1/16|1/32", default = 0, doc = "LFO 1 tempo sync: a cycle per note value, locked to the beat")] l1_sync;
    #[param(range = 0.0..5.0, default = 0.0, unit = "s", doc = "LFO 1 delay: silent for this long after the note starts")] l1_delay;
    #[param(range = 0.0..5.0, default = 0.0, unit = "s", doc = "LFO 1 attack: fades in over this long after the delay")] l1_attack;
    #[param(range = 0.0..12.0, default = 0.0, unit = "st", doc = "LFO 1 to pitch of both oscillators: semitones either way (vibrato)")] l1_pitch;
    #[param(range = 0.0..0.45, default = 0.0, doc = "LFO 1 to the pulse width of both oscillators")] l1_pwm;
    #[param(range = 0.0..4.0, default = 0.0, unit = "oct", doc = "LFO 1 to filter 1 cutoff: octaves either way")] l1_f1;
    #[param(range = 0.0..4.0, default = 0.0, unit = "oct", doc = "LFO 1 to filter 2 cutoff: octaves either way")] l1_f2;
    #[param(range = 0.0..1.0, default = 0.0, doc = "LFO 1 to amp 1 level (tremolo depth)")] l1_a1;
    #[param(range = 0.0..1.0, default = 0.0, doc = "LFO 1 to amp 2 level (tremolo depth)")] l1_a2;
    #[param(range = 0.0..12.0, default = 0.0, unit = "st", doc = "LFO 1 to oscillator 2 pitch only: semitones either way (sweeps a synced oscillator)")] l1_p2;

    #[param(choices = "sine|triangle|saw|square|sample & hold", default = 0, doc = "LFO 2 shape")] l2_shape;
    #[param(range = 0.01..40.0, default = 4.0, curve = log, unit = "Hz", doc = "LFO 2 rate when free-running")] l2_rate;
    #[param(choices = "free|2 bars|1 bar|1/2|1/4|1/8|1/16|1/32", default = 0, doc = "LFO 2 tempo sync: a cycle per note value, locked to the beat")] l2_sync;
    #[param(range = 0.0..5.0, default = 0.0, unit = "s", doc = "LFO 2 delay: silent for this long after the note starts")] l2_delay;
    #[param(range = 0.0..5.0, default = 0.0, unit = "s", doc = "LFO 2 attack: fades in over this long after the delay")] l2_attack;
    #[param(range = 0.0..12.0, default = 0.0, unit = "st", doc = "LFO 2 to pitch of both oscillators: semitones either way (vibrato)")] l2_pitch;
    #[param(range = 0.0..0.45, default = 0.0, doc = "LFO 2 to the pulse width of both oscillators")] l2_pwm;
    #[param(range = 0.0..4.0, default = 0.0, unit = "oct", doc = "LFO 2 to filter 1 cutoff: octaves either way")] l2_f1;
    #[param(range = 0.0..4.0, default = 0.0, unit = "oct", doc = "LFO 2 to filter 2 cutoff: octaves either way")] l2_f2;
    #[param(range = 0.0..1.0, default = 0.0, doc = "LFO 2 to amp 1 level (tremolo depth)")] l2_a1;
    #[param(range = 0.0..1.0, default = 0.0, doc = "LFO 2 to amp 2 level (tremolo depth)")] l2_a2;
    #[param(range = 0.0..12.0, default = 0.0, unit = "st", doc = "LFO 2 to oscillator 2 pitch only: semitones either way (sweeps a synced oscillator)")] l2_p2;
}
