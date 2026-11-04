//! An ABI v1 instance: the fixed `Io` written field by field and the parameters set by `dm_set_param` – what the three
//! repository modules speak until #343 migrates them and deletes this file (ABI-V2 §7, §9a: v1 beside v2, no dual host
//! after that ticket). A trap or a budget overrun silences the instance.
use crate::{fault_text, Arg, Backend, Export, Fault, Loaded};
use anyhow::{anyhow, bail, Result};
use deck_core::tempo::TempoMap;
use deck_module::abi::{get_str, put_str, EventRec, ABI_VERSION, MAX_AUDIO_PORTS, MAX_BLOCK, MAX_CONTROL_IN, MAX_EVENTS, MAX_VARS, STR_LEN};
use deck_module::host::Offsets;
use std::sync::Arc;

pub struct Instance<B: Backend> {
    pub(crate) loaded: Arc<Loaded<B>>,
    pub(crate) backend: B,
    io: usize,
    off: Offsets,
    rate: f64,
    pub(crate) silenced: Option<String>,
    /// µs of the last `process` / the slowest so far
    pub(crate) last_us: u32,
    pub(crate) max_us: u32,
}

impl<B: Backend> Instance<B> {
    /// `backend` is instantiated and answered `dm_abi() == 1`
    pub(crate) fn new(loaded: &Arc<Loaded<B>>, mut backend: B, rate: f64, seed: u64) -> Result<Instance<B>> {
        let name = &loaded.desc.name;
        debug_assert_eq!(ABI_VERSION, 1);
        let io = backend.call(Export::Io, &[]).map_err(|f| anyhow!("{name}: dm_io: {}", fault_text(&f)))? as usize;
        let off = Offsets::get();
        match io.checked_add(off.size) {
            Some(end) if end <= backend.memory_size() => {}
            _ => bail!("module '{name}': its Io does not lie in its memory"),
        }
        let mut me = Instance { loaded: loaded.clone(), backend, io, off, rate, silenced: None, last_us: 0, max_us: 0 };
        me.put_f64(me.off.rate, rate);
        me.put_u64(me.off.seed, seed);
        me.set_tempo(&TempoMap::new(120.0, 4, rate));
        // building the module runs its `init`: it gets the same budget as a block
        me.backend.set_budget(50);
        me.backend.call(Export::New, &[]).map_err(|f| anyhow!("{}: dm_new: {}", me.loaded.desc.name, fault_text(&f)))?;
        Ok(me)
    }

    fn put(&mut self, at: usize, bytes: &[u8]) {
        let io = self.io;
        self.backend.write(io + at, bytes);
    }

    fn put_f64(&mut self, at: usize, v: f64) {
        self.put(at, &v.to_le_bytes());
    }

    fn put_u64(&mut self, at: usize, v: u64) {
        self.put(at, &v.to_le_bytes());
    }

    fn put_u32(&mut self, at: usize, v: u32) {
        self.put(at, &v.to_le_bytes());
    }

    fn put_f32(&mut self, at: usize, v: f32) {
        self.put(at, &v.to_le_bytes());
    }

    fn get_u32(&self, at: usize) -> u32 {
        let mut b = [0u8; 4];
        self.backend.read(self.io + at, &mut b);
        u32::from_le_bytes(b)
    }

    pub fn set_tempo(&mut self, m: &TempoMap) {
        let o = self.off;
        self.put_f64(o.tempo_frame, m.frame);
        self.put_f64(o.tempo_beat, m.beat);
        self.put_f64(o.bpm, m.bpm);
        self.put_f64(o.rate, m.rate);
        self.put_u32(o.running, u32::from(m.running));
        self.put_u32(o.per_bar, m.per_bar);
    }

    pub fn set_character(&mut self, id: &str, persona: &str, mood: &str) {
        let o = self.off;
        for (at, s) in [(o.character_id, id), (o.character_persona, persona), (o.character_mood, mood)] {
            let mut buf = [0u8; STR_LEN];
            put_str(&mut buf, s);
            self.put(at, &buf);
        }
    }

    pub fn set_vars(&mut self, vars: &[(String, f32)]) {
        let o = self.off;
        let n = vars.len().min(MAX_VARS);
        for (i, (k, v)) in vars.iter().take(n).enumerate() {
            let mut key = [0u8; deck_module::abi::VAR_KEY_LEN];
            put_str(&mut key, k);
            self.put(o.var_keys + i * deck_module::abi::VAR_KEY_LEN, &key);
            self.put_f32(o.var_values + i * 4, *v);
        }
        self.put_u32(o.vars_len, n as u32);
    }

    pub fn set_param(&mut self, index: usize, value: f32) {
        if self.silenced.is_some() {
            return;
        }
        // a deadline is relative to when it is set: the last call's would be long past
        self.backend.set_budget(50);
        let r = self.backend.call(Export::SetParam, &[Arg::U32(index as u32), Arg::F32(value)]);
        self.check(r, "dm_set_param");
    }

    pub fn set_control_in(&mut self, port: usize, value: f32) {
        if port < MAX_CONTROL_IN {
            let at = self.off.control_in + port * 4;
            self.put_f32(at, value);
        }
    }

    pub fn set_audio_in(&mut self, port: usize, l: &[f32], r: &[f32]) {
        if port >= MAX_AUDIO_PORTS {
            return;
        }
        let n = l.len().min(r.len()).min(MAX_BLOCK);
        let base = self.off.audio_in + port * 2 * MAX_BLOCK * 4;
        for (ch, src) in [l, r].into_iter().enumerate() {
            self.put(base + ch * MAX_BLOCK * 4, bytemuck::cast_slice(&src[..n]));
        }
    }

    pub fn read_audio_out(&mut self, port: usize, l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len()).min(MAX_BLOCK);
        if port >= MAX_AUDIO_PORTS {
            l.fill(0.0);
            r.fill(0.0);
            return;
        }
        let base = self.io + self.off.audio_out + port * 2 * MAX_BLOCK * 4;
        for (ch, dst) in [l, r].into_iter().enumerate() {
            self.backend.read(base + ch * MAX_BLOCK * 4, bytemuck::cast_slice_mut(&mut dst[..n]));
        }
    }

    /// channel `c` (0 left, 1 right) of audio output `port` into `dst`; any other channel is silent
    pub fn read_audio_chan(&mut self, port: usize, c: usize, dst: &mut [f32]) {
        let n = dst.len().min(MAX_BLOCK);
        if port >= MAX_AUDIO_PORTS || c >= 2 {
            dst.fill(0.0);
            return;
        }
        let base = self.io + self.off.audio_out + port * 2 * MAX_BLOCK * 4;
        self.backend.read(base + c * MAX_BLOCK * 4, bytemuck::cast_slice_mut(&mut dst[..n]));
        dst[n..].fill(0.0);
    }

    fn budget(&self, frames: u32) -> u64 {
        ((0.75 * f64::from(frames) / self.rate * 1000.0).ceil() as u64).clamp(2, 50)
    }

    pub fn call(&mut self, frame: f64, frames: u32, control: bool) -> bool {
        if self.silenced.is_some() {
            return false;
        }
        let frames = frames.min(MAX_BLOCK as u32);
        self.put_f64(self.off.frame, frame);
        let ms = self.budget(frames);
        self.backend.set_budget(ms);
        let t = self.backend.clock_us();
        let export = if control { Export::Control } else { Export::Process };
        let r = self.backend.call(export, &[Arg::U32(frames)]);
        let us = self.backend.clock_us().saturating_sub(t).min(u64::from(u32::MAX)) as u32;
        (self.last_us, self.max_us) = (us, self.max_us.max(us));
        self.check(r, export.name())
    }

    /// a trap silences the instance: logged once, outputs stay silent
    fn check(&mut self, r: Result<u32, Fault>, what: &str) -> bool {
        match r {
            Ok(_) => true,
            Err(f) => {
                let msg = match f {
                    Fault::Budget => format!("{what} ran past its time budget"),
                    Fault::Trap(why) => format!("{what}: {why}"),
                };
                tracing::error!("module '{}' silenced – {msg}", self.loaded.desc.name);
                self.silenced = Some(msg);
                false
            }
        }
    }

    pub fn events(&mut self) -> Vec<EventRec> {
        let n = (self.get_u32(self.off.events_len) as usize).min(MAX_EVENTS);
        let size = self.off.event_size;
        let mut buf = vec![0u8; MAX_EVENTS * size];
        self.backend.read(self.io + self.off.events, &mut buf[..n * size]);
        (0..n).filter_map(|i| deck_module::host::event_at(&buf, i)).collect()
    }

    pub fn events_dropped(&mut self) -> u32 {
        self.get_u32(self.off.events_dropped)
    }

    pub fn character_id(&mut self) -> String {
        let mut buf = [0u8; STR_LEN];
        self.backend.read(self.io + self.off.character_id, &mut buf);
        get_str(&buf).to_owned()
    }
}
