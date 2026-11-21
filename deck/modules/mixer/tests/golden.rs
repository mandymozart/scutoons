//! The golden render: the script through the mixer – out and send – hashed. A change of the hash is a change of the sound.
mod common;
use common::*;

const GOLDEN: u64 = 0x5851_7685_0e88_f645;

type Stereo = (Vec<f32>, Vec<f32>);

/// one call of 128 frames with a full-scale constant on the inputs named: (out, send)
fn call(b: &mut Bench<Mixer>, inputs: &[usize]) -> (Stereo, Stereo) {
    for &p in inputs {
        b.rig.set_in_silent(p, false);
        b.rig.set_audio_in(p, 0, &[1.0; 128]);
        b.rig.set_audio_in(p, 1, &[-1.0; 128]);
    }
    b.run(128);
    let (out, send) = (b.rig.audio_out(Mixer::OUT), b.rig.audio_out(Mixer::SEND));
    ((out.chan(0).to_vec(), out.chan(1).to_vec()), (send.chan(0).to_vec(), send.chan(1).to_vec()))
}

/// the levels after everything has arrived: (out left, out right, send left, send right)
fn settled(b: &mut Bench<Mixer>, inputs: &[usize]) -> [f32; 4] {
    for _ in 0..40 {
        call(b, inputs);
    }
    let ((l, r), (sl, sr)) = call(b, inputs);
    [l[127], r[127], sl[127], sr[127]]
}

fn close(got: [f32; 4], want: [f32; 4]) -> bool {
    got.iter().zip(want).all(|(g, w)| (g - w).abs() < 1e-5)
}

#[test]
fn the_render_is_the_golden_one() {
    let mut b = bench(1);
    let mut h = Hash::new();
    let peak = (0..600).map(|k| step(&mut b, k, &mut h)).fold(0.0f32, f32::max);
    assert!(peak > 0.1 && peak < 8.0, "{peak}");
    assert_eq!(h.0, GOLDEN, "the render changed: {:#018x}", h.0);
}

#[test]
fn at_the_defaults_an_input_passes_as_it_is_and_inputs_add_up() {
    let mut b = bench(1);
    assert_eq!(call(&mut b, &[]).0 .0, vec![0.0; 128], "nothing linked, nothing out");
    let ((l, r), (sl, sr)) = call(&mut b, &[2]);
    assert!(l == [1.0; 128] && r == [-1.0; 128], "0 dB, centre, master 0 dB: bit for bit");
    assert!(sl == [0.0; 128] && sr == [0.0; 128], "no send until its level is raised");
    assert!(close(settled(&mut b, &[0, 2, 7]), [3.0, -3.0, 0.0, 0.0]));
}

#[test]
fn level_balance_mute_send_and_master_each_do_their_part() {
    let mut b = bench(1);
    b.set("gain1", -20.0);
    assert!(close(settled(&mut b, &[0]), [0.1, -0.1, 0.0, 0.0]));
    b.set("pan1", -1.0);
    assert!(close(settled(&mut b, &[0]), [0.1, 0.0, 0.0, 0.0]), "hard left: the right side is gone, the left as it was");
    b.set("pan1", 0.5);
    assert!(close(settled(&mut b, &[0]), [0.05, -0.1, 0.0, 0.0]));
    // the send takes the input after its level, before its balance; the master does not touch it
    b.set("send1", 0.0).set("master", -20.0);
    assert!(close(settled(&mut b, &[0]), [0.005, -0.01, 0.1, -0.1]));
    b.set("mute1", 1.0);
    assert!(close(settled(&mut b, &[0]), [0.0; 4]), "muted in out and send");
    b.set("mute1", 0.0).set("master", -60.0);
    assert!(close(settled(&mut b, &[0]), [0.0, 0.0, 0.1, -0.1]));
}

#[test]
fn the_preset_spreads_the_first_pair_and_a_level_glides() {
    let mut b = bench(1);
    b.rig.apply_preset();
    let send = 2.0 * 10f32.powf(-12.0 / 20.0);
    assert!(close(settled(&mut b, &[0, 1]), [1.4, -1.4, send, -send]));
    b.set("gain1", -60.0).set("gain2", -60.0);
    let ((l, _), _) = call(&mut b, &[0, 1]);
    assert!(l[0] > 1.35 && l.windows(2).all(|w| w[1] < w[0]) && l[127] < 0.5, "from {} down to {}", l[0], l[127]);
}
