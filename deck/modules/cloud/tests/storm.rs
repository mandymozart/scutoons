//! The storm's repeater, measured: with sync every repeat lands on its note value's grid of the tempo clock to the
//! frame, however the calls are cut; free, the repeats follow the rate a second; a new rate glides through the values
//! between; nothing fires while nothing is held; time, flash, gate and colour leave on the control outputs; the gate rises at
//! every repeat at every rate.
mod common;
use cloud_module::DIVISIONS;
use common::*;
use deck_module::{Ev, Kind};

/// 120 BPM (the rig's default tempo): frames a beat
const BEAT: f64 = RATE * 0.5;

/// the absolute frames of the notes a port sent over `frames`, in calls of the sizes `cut` cycles through
fn repeats(b: &mut Bench<Cloud>, port: usize, frames: usize, cut: &[usize]) -> Vec<u64> {
    let mut at = vec![];
    let mut k = 0;
    let end = b.frame + frames as u64;
    while b.frame < end {
        let n = cut[k % cut.len()].min((end - b.frame) as usize);
        let start = b.frame;
        b.run(n);
        at.extend(b.rig.events_out(port).iter().filter(|e| e.kind() == Some(Kind::NoteOn)).map(|e| start + u64::from(e.frame)));
        k += 1;
    }
    at
}

fn held(seed: u64, sync: f32, rate: f32) -> Bench<Cloud> {
    let mut b = bench(seed);
    b.set("sync", sync).set("rate", rate).set("hold", 1.0);
    b
}

#[test]
fn synced_repeats_land_on_the_note_value_s_grid() {
    for (i, beats) in DIVISIONS.iter().enumerate() {
        let rate = i as f32 / (DIVISIONS.len() - 1) as f32;
        assert_eq!(Cloud::division(rate), *beats);
        let span = (BEAT * 8.0) as usize;
        let mut b = held(1, 1.0, rate);
        let at = repeats(&mut b, Cloud::KICK, span, &[128]);
        let want: Vec<u64> = (0..).map(|k| (k as f64 * beats * BEAT).round() as u64).take_while(|&f| f < span as u64).collect();
        assert_eq!(at, want, "1/{} on the grid", (4.0 / beats) as u32);
        for cut in [&[37usize, 128, 5][..], &[64], &[1, 127]] {
            let mut c = held(1, 1.0, rate);
            assert_eq!(repeats(&mut c, Cloud::KICK, span, cut), want, "cut {cut:?}: the same frames");
        }
    }
}

#[test]
fn a_synced_press_waits_for_the_grid_and_a_stopped_clock_plays_nothing() {
    let mut b = bench(1);
    b.set("sync", 1.0).set("rate", 0.4); // 1/4: a beat
    b.run(128);
    b.run(128);
    b.set("hold", 1.0);
    let at = repeats(&mut b, Cloud::KICK, (BEAT * 2.0) as usize, &[128]);
    assert_eq!(at, [24_000, 48_000], "held from frame 256: beats 1 and 2, not at the press");
    let mut s = held(1, 1.0, 1.0);
    s.rig.set_tempo(0.0, 0.0, 120.0, 4, false);
    assert!(repeats(&mut s, Cloud::KICK, 48_000, &[128]).is_empty(), "no clock, no grid");
}

#[test]
fn free_repeats_follow_the_rate() {
    for rate in [0.0, 0.3, 0.5, 0.8, 1.0] {
        let hz = Cloud::free_hz(rate);
        let mut b = bench(1);
        b.set("sync", 0.0).set("rate", rate);
        b.run(100);
        b.set("hold", 1.0);
        let at = repeats(&mut b, Cloud::THUNDER, (RATE * 4.5) as usize, &[128, 77]);
        assert_eq!(at[0], 100, "{rate}: the first repeat at the press");
        let want = RATE / hz;
        assert_eq!(at.len(), 1 + ((RATE * 4.5 - 1.0) / want) as usize, "{rate}: {hz} a second");
        assert!(at.windows(2).all(|w| ((w[1] - w[0]) as f64 - want).abs() <= 1.0), "{rate}: every {want} frames: {at:?}");
    }
    assert_eq!((Cloud::free_hz(0.0), Cloud::free_hz(1.0)), (0.5, 32.0));
}

#[test]
fn a_new_rate_glides() {
    // free: from 2 a second to 32 a second, the gaps shrink step by step, never at once
    let mut b = held(1, 0.0, 0.4);
    repeats(&mut b, Cloud::KICK, 48_000, &[128]);
    b.set("rate", 1.0);
    let at = repeats(&mut b, Cloud::KICK, 48_000 * 3, &[128]);
    let gaps: Vec<u64> = at.windows(2).map(|w| w[1] - w[0]).collect();
    assert!(gaps.windows(2).all(|g| g[1] <= g[0] + 1), "shrinking: {gaps:?}");
    let (first, last) = (gaps[0] as f64, *gaps.last().unwrap() as f64);
    assert!((last / (RATE / 32.0) - 1.0).abs() < 0.02, "near 32 a second after six glide times: {last}");
    let between = gaps.iter().filter(|&&g| (g as f64) < first * 0.8 && (g as f64) > last * 1.5).count();
    assert!(between >= 5, "through the speeds between: {gaps:?}");
    // synced: from 1/4 to 1/32 through 1/8 and 1/16, every repeat still on its grid
    let mut s = held(1, 1.0, 0.4);
    repeats(&mut s, Cloud::KICK, BEAT as usize, &[128]);
    s.set("rate", 1.0);
    let at = repeats(&mut s, Cloud::KICK, (BEAT * 4.0) as usize, &[128]);
    let mut seen = vec![];
    for w in at.windows(2) {
        let gap = (w[1] - w[0]) as f64 / BEAT;
        let beats = DIVISIONS.iter().copied().find(|d| (gap - d).abs() < 1e-3).unwrap_or_else(|| panic!("a gap of {gap} beats"));
        assert!((w[1] as f64 / (beats * BEAT)).fract().abs() < 1e-9, "{} on the 1/{} grid", w[1], (4.0 / beats) as u32);
        if seen.last() != Some(&beats) {
            seen.push(beats);
        }
    }
    assert_eq!(seen, [0.5, 0.25, 0.125], "from 1/4 through 1/8 and 1/16 to 1/32");
}

#[test]
fn nothing_held_nothing_fires() {
    for sync in [0.0, 1.0] {
        let mut b = bench(2);
        b.set("sync", sync).set("rate", 1.0);
        assert!(repeats(&mut b, Cloud::KICK, 48_000, &[128]).is_empty());
        b.set("hold", 1.0);
        assert!(!repeats(&mut b, Cloud::KICK, 12_000, &[128]).is_empty());
        b.set("hold", 0.0);
        b.run(128);
        assert!(repeats(&mut b, Cloud::THUNDER, 48_000, &[128]).is_empty(), "released: silent");
        assert!(repeats(&mut b, Cloud::KICK, 48_000, &[128]).is_empty());
    }
}

#[test]
fn each_repeat_is_a_thunder_and_a_kick_at_their_velocities() {
    let mut b = held(1, 1.0, 0.6);
    b.set("rumble", 0.6).set("thump", 0.9);
    let mut both: Vec<(Ev, Ev)> = vec![];
    for _ in 0..400 {
        b.run(128);
        both.extend(b.rig.events_out(Cloud::THUNDER).into_iter().zip(b.rig.events_out(Cloud::KICK)));
    }
    assert_eq!(both.len(), 5, "1/8 over 1.07 s at 120 BPM");
    for (k, (t, h)) in both.iter().enumerate() {
        assert_eq!((t.frame, t.id), (h.frame, h.id));
        assert_eq!(t.id, k as u32);
        assert_eq!((t.key(), h.key()), (40.0, 36.0));
        assert!((t.velocity() - 0.6).abs() < 1e-6 && (h.velocity() - 0.9).abs() < 1e-6);
        assert!((t.len - 0.25).abs() < 1e-6, "half the period: {}", t.len);
    }
    b.set("thump", 0.0);
    assert!(repeats(&mut b, Cloud::KICK, 48_000, &[128]).is_empty(), "thump 0: no kick");
}

#[test]
fn time_flash_and_colour_go_out() {
    let mut b = held(1, 1.0, 0.4);
    b.set("hue", 0.0).set("tint", 100.0).set("light", 50.0).set("flash", 1.0);
    let mut pulses = vec![];
    let mut table_at_hit = None;
    let mut quiet = None;
    for k in 0..375 {
        let start = b.frame;
        b.run(128);
        let hit = !b.rig.events_out(Cloud::KICK).is_empty();
        let (period, _) = b.rig.control_out(Cloud::PERIOD);
        assert!((period - 0.5).abs() < 1e-6, "a beat at 120 BPM: {period}");
        let (phase, _) = b.rig.control_out(Cloud::PHASE);
        let want = ((start + 128) as f64 / BEAT).fract() as f32;
        assert!((phase - want).abs() < 1e-4, "{k}: phase {phase}, not {want}");
        let pulse = b.rig.control_out(Cloud::PULSE).0;
        pulses.push((hit, pulse));
        let lit = b.rig.instances()[0].colour[0];
        if hit && table_at_hit.is_none() {
            table_at_hit = Some(b.rig.instances());
        }
        if k == 180 {
            quiet = Some(lit);
        }
    }
    assert!(pulses.iter().filter(|p| p.0).all(|p| p.1 == 1.0), "1 at every repeat");
    assert!(pulses[180].1 < 0.05, "faded between the beats: {}", pulses[180].1);
    assert_eq!([0, 1, 2].map(|i| b.rig.control_out(Cloud::RED + i).0), [0.5, 0.0, 0.0], "red at half light");
    let hit = table_at_hit.expect("a repeat");
    assert!(hit[0].colour[1] > 0.2 && quiet.unwrap() < hit[0].colour[0], "the flash lights the puffs");
}

/// per call over `frames`: (a repeat in it, the gate, the phase), in calls of the sizes `cut` cycles through
fn gates(b: &mut Bench<Cloud>, frames: usize, cut: &[usize]) -> Vec<(bool, f32, f32)> {
    let mut calls = vec![];
    let end = b.frame + frames as u64;
    let mut k = 0;
    while b.frame < end {
        b.run(cut[k % cut.len()].min((end - b.frame) as usize));
        let hit = b.rig.events_out(Cloud::KICK).iter().any(|e| e.kind() == Some(Kind::NoteOn));
        calls.push((hit, b.rig.control_out(Cloud::GATE).0, b.rig.control_out(Cloud::PHASE).0));
        k += 1;
    }
    calls
}

#[test]
fn the_gate_rises_at_every_repeat_and_is_low_in_the_second_half() {
    // 1/32 synced (3000 frames at 120 BPM) and 32 a second free (1500 frames): pulse never falls below 0.25 there
    for (sync, what) in [(1.0, "1/32"), (0.0, "32 a second")] {
        for cut in [&[128usize][..], &[37, 128, 5]] {
            let mut b = held(1, sync, 1.0);
            let calls = gates(&mut b, 48_000, cut);
            let hits = calls.iter().filter(|c| c.0).count();
            assert!(hits >= 16, "{what}: {hits} repeats");
            let mut was = 0.0;
            let mut rises = 0;
            for (k, &(hit, gate, phase)) in calls.iter().enumerate() {
                assert!(gate == 0.0 || gate == 1.0, "{what}: {gate}");
                if gate > was {
                    rises += 1;
                    assert!(hit, "{what} {cut:?}: call {k} rises without a repeat");
                }
                if hit {
                    assert_eq!(gate, 1.0, "{what} {cut:?}: call {k} has a repeat, the gate is up");
                }
                if phase >= 0.5 || (phase < 1e-5 && !hit) {
                    assert_eq!(gate, 0.0, "{what} {cut:?}: call {k}: the second half of the period (phase {phase})");
                } else if phase < 0.499 {
                    assert_eq!(gate, 1.0, "{what} {cut:?}: call {k}: the first half of the period (phase {phase})");
                }
                was = gate;
            }
            assert_eq!(rises, hits, "{what} {cut:?}: one rise a repeat");
        }
    }
}

#[test]
fn the_gate_is_low_while_not_held() {
    for sync in [0.0, 1.0] {
        let mut b = bench(2);
        b.set("sync", sync).set("rate", 1.0);
        assert!(gates(&mut b, 24_000, &[128]).iter().all(|c| c.1 == 0.0), "{sync}: not held");
        b.set("hold", 1.0);
        assert!(gates(&mut b, 12_000, &[128]).iter().any(|c| c.1 == 1.0));
        b.set("hold", 0.0);
        assert!(gates(&mut b, 24_000, &[128]).iter().all(|c| c.1 == 0.0), "{sync}: released");
    }
    let mut s = held(1, 1.0, 1.0);
    s.rig.set_tempo(0.0, 0.0, 120.0, 4, false);
    assert!(gates(&mut s, 24_000, &[128]).iter().all(|c| c.1 == 0.0), "a stopped clock: no repeats, no gate");
}
