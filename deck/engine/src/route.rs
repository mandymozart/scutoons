//! The routing kinds' blocks (plan 2.6, #318): `bus`, `pan`, `downmix` – and the rules for their channel counts. `out` is
//! blocks.rs's `Out` (the sink `audio_out` is too) and `feedback` is the engine's own step (lib.rs: it is a copy one block
//! late, not a computation). Discrete by default: none of these mixes channels unless it is asked to.
use super::blocks::Glide;
use super::config::DeckConfig;
use super::layout::{self, Layout, Speaker};
use super::pan::{Law, Panner};
use super::{Block, Buf, Param};
use anyhow::{anyhow, bail, Result};
use deck_core::smooth::GLIDE;
use deck_space::graph::{Kind, Node, MAX_CHANNELS};
use std::sync::Arc;

/// The layout a node names: its own speakers (`az`, and `el`, `dist` per speaker – a venue's array as data on the node),
/// else the layout called `name` (the deck configuration's, then a built-in).
pub fn layout_of(node: &Node, cfg: &DeckConfig, key: &str, default: &str) -> Result<Layout> {
    let az = node.f32s("az");
    if !az.is_empty() {
        let (el, dist) = (node.f32s("el"), node.f32s("dist"));
        let speakers = az.iter().enumerate().map(|(i, &az)| Speaker { az, el: el.get(i).copied().unwrap_or(0.0), dist: dist.get(i).copied().unwrap_or(1.0), lfe: false }).collect();
        return Ok(Layout { name: format!("{} speakers", node.id), speakers });
    }
    let name = node.str(key).unwrap_or(default);
    cfg.layout(name).ok_or_else(|| anyhow!("node '{}': unknown layout '{name}' (mono, stereo, quad, 5.1, 7.1, or one of the deck configuration's)", node.id))
}

/// a knob that holds a channel count, checked
fn count(node: &Node, key: &str, default: f32) -> Result<usize> {
    let v = node.f32(key, default);
    if !(0.0..=MAX_CHANNELS as f32).contains(&v) {
        bail!("node '{}': {key} = {v} (0..={MAX_CHANNELS} channels)", node.id);
    }
    Ok(v as usize)
}

/// The gain matrix of a `downmix` node (rows = output channels, columns = input channels): its `matrix` row by row, or the
/// spec's table between `from` and `to`.
pub fn downmix_matrix(node: &Node, cfg: &DeckConfig) -> Result<Vec<Vec<f32>>> {
    let (from, to) = (node.str("from").unwrap_or("5.1"), node.str("to").unwrap_or("stereo"));
    let flat = node.f32s("matrix");
    if !flat.is_empty() {
        let cols = match count(node, "inputs", 0.0)? {
            0 => cfg.layout(from).ok_or_else(|| anyhow!("node '{}': unknown layout '{from}'", node.id))?.channels(),
            n => n,
        };
        if flat.len() % cols != 0 || flat.len() / cols > MAX_CHANNELS {
            bail!("node '{}': the matrix has {} gains – rows of {cols} (the input channels) are needed", node.id, flat.len());
        }
        return Ok(flat.chunks(cols).map(<[f32]>::to_vec).collect());
    }
    layout::matrix(from, to).ok_or_else(|| anyhow!("node '{}': no standard down-mix from {from} to {to} – give a `matrix`", node.id))
}

/// how many channels input `port` of `node` must have: a fixed port its count, an `any` port that sizes itself on what is
/// linked (None) unless the node says how wide it works
pub fn want_in(node: &Node, port: usize, cfg: &DeckConfig) -> Result<Option<usize>> {
    use deck_catalog::Channels;
    let declared = node.kind.inputs()[port].channels;
    Ok(match (node.kind, declared) {
        (_, Some(Channels::Count(n))) => Some(usize::from(n)),
        (Kind::Bus, _) => Some(count(node, "channels", 0.0)?).filter(|&n| n > 0),
        (Kind::Feedback, _) => Some(count(node, "channels", 2.0)?.max(1)),
        (Kind::Downmix, _) => Some(downmix_matrix(node, cfg)?.first().map_or(1, Vec::len)),
        (_, Some(Channels::Any(_))) => None,
        // a module: ABI v1's stereo pair
        (_, None) => Some(2),
    })
}

/// the channel counts of `node`'s output ports, given the widths of its resolved inputs (None = nothing linked)
pub fn out_widths(node: &Node, in_widths: &[Option<usize>], cfg: &DeckConfig) -> Result<Vec<usize>> {
    use deck_catalog::Channels;
    let first_in = in_widths.iter().copied().flatten().next();
    node.kind
        .outputs()
        .iter()
        .map(|p| -> Result<usize> {
            Ok(match (node.kind, p.channels) {
                (_, Some(Channels::Count(n))) => usize::from(n),
                (Kind::Bus, _) => match count(node, "channels", 0.0)? {
                    0 => first_in.unwrap_or(2),
                    n => n,
                },
                (Kind::Feedback, _) => count(node, "channels", 2.0)?.max(1),
                (Kind::Downmix, _) => downmix_matrix(node, cfg)?.len(),
                (Kind::Pan, _) => layout_of(node, cfg, "layout", "stereo")?.channels(),
                (_, Some(Channels::Any(_))) => first_in.unwrap_or(2),
                (_, None) => 2,
            })
        })
        .collect::<Result<Vec<_>>>()
}

/// `bus`, `pan`, `downmix` as blocks; None for every other kind. `solos` = the solo params of every bus of the build.
pub fn make(node: &Node, rate: f32, params: &[Param], widths: &[usize], cfg: &DeckConfig, solos: &Solos) -> Result<Option<Box<dyn Block>>> {
    let p = |name: &str| params[node.kind.input(name).expect("a port of this kind")].clone();
    Ok(Some(match node.kind {
        Kind::Bus => Box::new(Bus { gain: Glide::new(p("gain"), GLIDE, rate), mute: p("mute"), solo: p("solo"), solos: solos.clone(), gate: f32::NAN }),
        Kind::Pan => {
            let layout = layout_of(node, cfg, "layout", "stereo")?;
            let law = match node.str("mode").unwrap_or("auto") {
                "auto" if layout.speakers.iter().filter(|s| !s.lfe).count() == 2 => Law::EqualPower,
                "auto" => Law::Vbap,
                m => Law::parse(m).ok_or_else(|| anyhow!("node '{}': unknown mode '{m}' (auto, equal_power, vbap, dbap)", node.id))?,
            };
            let panner = Panner::new(&layout, law, node.f32("rolloff", 6.0), node.f32("blur", 0.1)).map_err(|e| anyhow!("node '{}': {e}", node.id))?;
            let w = panner.channels();
            Box::new(PanBlock { panner, az: p("azimuth"), el: p("elevation"), dist: p("distance"), cur: vec![0.0; w], target: vec![0.0; w], last: None })
        }
        Kind::Downmix => {
            let matrix = downmix_matrix(node, cfg)?;
            debug_assert_eq!(matrix.len(), widths[0]);
            Box::new(Downmix { matrix })
        }
        _ => return Ok(None),
    }))
}

/// What a bus needs to know to follow solo: every bus's solo param, the ones of the buses that feed it, and the ones of the
/// buses its `out` feeds. Soloing a track mutes the others but not the folder it plays into or the bus its send feeds
/// (Reaper's rule): those lie downstream. Soloing a folder keeps its children playing into it: they lie upstream.
#[derive(Clone)]
pub struct Solos {
    pub all: Arc<[Param]>,
    pub upstream: Arc<[Param]>,
    pub downstream: Arc<[Param]>,
}

/// Reaper's track: the sum of its input, a fader, mute and solo. `out` is post-fader and gated, `pre` the sum itself.
struct Bus {
    gain: Glide,
    mute: Param,
    solo: Param,
    /// every bus's solo param: one soloed silences the `out` of the others – except those downstream of it
    solos: Solos,
    /// the gate (1 = open) the last block ended on; it ramps over a block (no click on a mute); NaN before the first block
    gate: f32,
}

impl Block for Bus {
    fn process(&mut self, ins: &[Option<&Buf>], outs: &mut [Buf], n: usize) {
        let (post, pre) = outs.split_at_mut(1);
        let (post, pre) = (&mut post[0], &mut pre[0]);
        let any_solo = self.solos.all.iter().any(|s| s.get() > 0.5);
        let soloed = |list: &[Param]| list.iter().any(|s| s.get() > 0.5);
        let heard = !any_solo || self.solo.get() > 0.5 || soloed(&self.solos.upstream) || soloed(&self.solos.downstream);
        let open = self.mute.get() <= 0.5 && heard;
        let target = if open { 1.0 } else { 0.0 };
        let from = if self.gate.is_nan() { target } else { self.gate };
        self.gate = target;
        let (g, dg) = self.gain.block(n);
        let Some(src) = ins[0] else {
            post.clear(n);
            pre.clear(n);
            return;
        };
        for c in 0..post.channels() {
            if c >= src.channels() {
                post.ch_mut(c)[..n].fill(0.0);
                pre.ch_mut(c)[..n].fill(0.0);
                continue;
            }
            pre.ch_mut(c)[..n].copy_from_slice(&src.ch(c)[..n]);
            for (i, (o, x)) in post.ch_mut(c)[..n].iter_mut().zip(&src.ch(c)[..n]).enumerate() {
                *o = x * (g + dg * i as f32) * (from + (target - from) * (i + 1) as f32 / n as f32);
            }
        }
    }
}

/// A mono source on a speaker layout: its first input channel times a gain per speaker, the gains gliding across a block to
/// where the direction now puts them.
struct PanBlock {
    panner: Panner,
    az: Param,
    el: Param,
    dist: Param,
    cur: Vec<f32>,
    target: Vec<f32>,
    /// the direction the target gains are for (bits); None before the first block
    last: Option<[u32; 3]>,
}

impl Block for PanBlock {
    fn process(&mut self, ins: &[Option<&Buf>], outs: &mut [Buf], n: usize) {
        let (az, el, dist) = (self.az.get(), self.el.get(), self.dist.get());
        let key = [az.to_bits(), el.to_bits(), dist.to_bits()];
        let first = self.last.is_none();
        if self.last != Some(key) {
            self.panner.gains(az, el, dist, &mut self.target);
            self.last = Some(key);
        }
        if first {
            self.cur.copy_from_slice(&self.target);
        }
        let out = &mut outs[0];
        let Some(src) = ins[0] else {
            out.clear(n);
            self.cur.copy_from_slice(&self.target);
            return;
        };
        let x = &src.ch(0)[..n];
        for c in 0..out.channels() {
            let (g0, g1) = (self.cur[c], self.target[c]);
            let d = (g1 - g0) / n as f32;
            for (i, (o, x)) in out.ch_mut(c)[..n].iter_mut().zip(x).enumerate() {
                *o = x * (g0 + d * i as f32);
            }
        }
        self.cur.copy_from_slice(&self.target);
    }
}

/// A gain matrix: output row r = Σ over the columns of `matrix[r][c] · input c`, in column order, zeros skipped.
struct Downmix {
    matrix: Vec<Vec<f32>>,
}

impl Block for Downmix {
    fn process(&mut self, ins: &[Option<&Buf>], outs: &mut [Buf], n: usize) {
        let out = &mut outs[0];
        out.clear(n);
        let Some(src) = ins[0] else { return };
        for (r, row) in self.matrix.iter().enumerate().take(out.channels()) {
            let o = out.ch_mut(r);
            for (c, &m) in row.iter().enumerate().take(src.channels()) {
                if m != 0.0 {
                    for (o, x) in o[..n].iter_mut().zip(&src.ch(c)[..n]) {
                        *o += m * x;
                    }
                }
            }
        }
    }
}
