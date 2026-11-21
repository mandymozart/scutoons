//! The DSP units allocate in their constructors only: with every unit built, thousands of samples through all of them
//! – every wave, filter mode and envelope stage, whole and fractional delay reads, the feedback units, the FFT there and
//! back – make no call to the allocator (the SDK's counting allocator is this binary's global allocator).
use deck_module::alloc::{self, Counting};
use deck_module::dsp::adsr::{Adsr, Times};
use deck_module::dsp::delay::{Allpass, Comb, Delay};
use deck_module::dsp::fft::Fft;
use deck_module::dsp::follower::Follower;
use deck_module::dsp::noise::{Pink, White};
use deck_module::dsp::onepole::OnePole;
use deck_module::dsp::svf::{self, Coef, Mode, Svf};
use deck_module::dsp::{osc, wavetable};

#[global_allocator]
static A: Counting = Counting;

const RATE: f32 = 48_000.0;
const N: usize = 256;

#[test]
fn the_units_never_allocate_after_they_are_built() {
    // init: everything that owns a buffer or a table is built here
    let tables = [wavetable::Wave::Saw, wavetable::Wave::Square, wavetable::Wave::Triangle].map(|w| wavetable::WaveTable::new(w, RATE));
    let mut table_osc = wavetable::Osc::new();
    let (mut blep, mut sync, mut sub) = (osc::Osc::default(), osc::Osc::default(), osc::Sub::default());
    let (mut white, mut pink) = (White::new(3), Pink::new(4));
    let (mut filter, mut lowpass, mut pole) = (Svf::new(), Svf::new(), OnePole::new());
    let mut env = Adsr::default();
    let times = Times { attack: 0.002, decay: 0.01, sustain: 0.5, release: 0.02 };
    let mut follower = Follower::new(0.001, 0.05, RATE);
    let (mut line, mut allpass, mut comb) = (Delay::new(2048), Allpass::new(600), Comb::new(2048));
    let fft = Fft::new(N);
    let (mut frame, mut re, mut im) = ([0.0f32; N], [0.0f32; N], [0.0f32; N]);

    let mut sum = 0.0f64;
    alloc::arm(false);
    for i in 0..48_000usize {
        if i % 2400 == 0 {
            env.gate_on(&times, f64::from(RATE));
        }
        if i % 2400 == 1500 {
            env.gate_off(&times, f64::from(RATE));
        }
        let hz = 110.0 + (i % 5000) as f32;
        let wave = osc::Wave::from_param((i / 6000 % 4) as f32);
        let main = blep.tick(wave, f64::from(hz / RATE), 0.3, None);
        let mut x = main.value + sync.tick(osc::Wave::Saw, f64::from(2.37 * hz / RATE), 0.5, main.wrap).value + sub.tick(main.wrap);
        x += table_osc.tick(&tables[i / 4000 % 3], hz, 1.0 / RATE) + 0.3 * white.sample() + 0.3 * white.next() + 0.3 * pink.sample();
        if i % 16 == 0 {
            // a control-rate coefficient, as a module computes it
            let coef = Coef::new(f64::from(200.0 + hz * 4.0), 0.6, f64::from(RATE));
            x = filter.tick(&coef, Mode::from_param((i / 3000 % 4) as f32), svf::drive(x, 0.4));
        }
        x = lowpass.lowpass(x, 400.0 + hz, 1.2, core::f32::consts::PI / RATE);
        x = pole.highpass(x, OnePole::coef(30.0, RATE)) * env.tick(&times, f64::from(RATE));
        let echo = line.read(1 + i % 2000) + line.read_frac(1.0 + (i % 1999) as f32 * 0.731);
        line.write(x);
        x += 0.2 * echo + comb.tick(x, 1 + i % 1500, 0.8, 0.4) + allpass.tick(x, 1 + i % 500, 0.6);
        sum += f64::from(follower.tick(x));
        frame[i % N] = x;
        if i % N == N - 1 {
            fft.real(&frame, &mut re, &mut im);
            fft.inverse(&mut re, &mut im);
            sum += f64::from(re[0]);
        }
    }
    let allocations = alloc::disarm();
    assert_eq!(allocations, 0, "a unit allocated after it was built");
    assert!(sum.is_finite() && sum > 1.0, "the run was not silent: {sum}");
}

#[test]
fn the_counter_sees_an_allocation() {
    alloc::arm(false);
    let line = Delay::new(64);
    let seen = alloc::disarm();
    assert!(seen > 0 && line.max() == 64, "building a unit allocates, and that is counted: {seen}");
}
