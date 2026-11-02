//! A fixture module for the hosts' tests (#280): behaves by its `mode` parameter – 0 plays a constant 0.25 on both
//! channels, 1 panics, 2 never returns, 3 logs 50 lines a block, 4 reads the character / a variable / the tempo and
//! plays them as numbers (left = bpm / 100 + variable `fx.level`, right = the beat). Not an instrument.
use deck_module::{log, Block, Ctx, Event, Module};

#[derive(Module)]
#[module(name = "fixture")]
#[port(input, audio, name = "in")]
#[port(output, audio, name = "out")]
#[port(output, event, name = "ticks")]
pub struct Fixture {
    #[param(range = 0.0..4.0, default = 0.0)]
    mode: f32,
    #[param(range = -1.0..1.0, default = 0.0)]
    bias: f32,
}

impl Module for Fixture {
    fn process(&mut self, block: &mut Block, ctx: &Ctx) {
        match self.mode as u32 {
            1 => panic!("boom {}", ctx.seed()),
            2 => {
                let mut n = 0u64;
                loop {
                    n = std::hint::black_box(n.wrapping_add(1));
                }
            }
            3 => {
                for i in 0..50 {
                    log!("line {i}");
                }
            }
            _ => {}
        }
        let n = block.frames();
        let (il, ir) = block.audio_in(0);
        let (mut sl, mut sr) = (0.0f32, 0.0f32);
        for i in 0..n {
            sl += il[i];
            sr += ir[i];
        }
        let (bpm, beat) = (ctx.tempo().bpm as f32, ctx.beats().0 as f32);
        let level = ctx.var("fx.level").unwrap_or(0.0);
        let mode = self.mode as u32;
        let bias = self.bias;
        let (l, r) = block.audio_out(0);
        for i in 0..n {
            (l[i], r[i]) = match mode {
                4 => (bpm / 100.0 + level, beat),
                _ => (0.25 + bias, 0.25 + bias),
            };
        }
        // the input is added so a test can see the audio cross the boundary
        if n > 0 && mode == 0 {
            l[0] += sl;
            r[0] += sr;
        }
        block.emit(Event::at_beat("tick", beat as f64));
    }
}
