//! Each derivative of one source shows the source's picture at an in point given in source time: its time map measured
//! by pictures, then the frame at the in point – and again after a loop's wrap – is the source's frame there.
//! The fixtures (tests/fixtures/derivatives): a source of 64 × 48, 12 fps, 2 s whose every frame differs (a white bar
//! running right, a red one stepping down), and what was made of it:
//!   C="color=s=64x48:r=12:d=2"; ffmpeg -f lavfi -i "$C:c=0x202040" -f lavfi -i "color=c=white:s=8x48:r=12:d=2" \
//!     -f lavfi -i "color=c=red:s=64x6:r=12:d=2" -filter_complex "[0][1]overlay=x='t*30':y=0[a];[a][2]overlay=x=0:y='mod(t*84,42)'" \
//!     -c:v hap -format hap source.mov
//!   AV1="-c:v libaom-av1 -strict -2 -cpu-used 8 -crf 24 -pix_fmt yuv420p"
//!   ffmpeg -i source.mov $AV1 -g 6 av1.mp4                 ffmpeg -ss 0.5 -i source.mov $AV1 -g 6 av1_from_half.mp4
//!   ffmpeg -i source.mov -vf fps=24 -vsync cfr $AV1 -g 12 av1_24fps.webm   ffmpeg -i source.mov -vf scale=32:24 -c:v hap -format hap_q hap_q_small.mov
//! (each with -map_metadata -1 -fflags +bitexact)
use deck_media::timemap::{self, thumbnails, unlike};
use deck_media::{Clip, Frames, Player, Software, Span, TimeMap};

fn clip(name: &str) -> Clip {
    let bytes = std::fs::read(format!("{}/tests/fixtures/derivatives/{name}", env!("CARGO_MANIFEST_DIR"))).expect("the fixture");
    Clip::open(bytes.into()).unwrap()
}

/// (file, the source's time at its start, its frame rate)
fn derivatives() -> Vec<(&'static str, f64, f64)> {
    let mut d = vec![("hap_q_small.mov", 0.0, 12.0)];
    if deck_media::AV1 {
        d.extend([("av1.mp4", 0.0, 12.0), ("av1_from_half.mp4", 0.5, 12.0), ("av1_24fps.webm", 0.0, 24.0)]);
    }
    d
}

/// which of the source's frames a picture is: the nearest by thumbnail
fn which(source: &[(f64, Vec<f32>)], th: &[f32]) -> usize {
    (0..source.len()).min_by(|&a, &b| unlike(&source[a].1, th).total_cmp(&unlike(&source[b].1, th))).unwrap()
}

#[test]
fn every_derivative_is_measured_against_its_source() {
    let source = clip("source.mov");
    for (name, offset, fps) in derivatives() {
        let map = timemap::measure(&source, &clip(name)).unwrap();
        assert!((map.offset - offset).abs() < 1e-6, "{name}: starts at {} s in the source, measured {}", offset, map.offset);
        assert!((map.fps - fps).abs() < 0.01, "{name}: {} fps, measured {}", fps, map.fps);
    }
    assert_eq!(timemap::measure(&source, &source).unwrap(), TimeMap::new(0.0, 12.0), "the source against itself");
}

#[test]
fn the_frame_at_the_in_point_is_the_sources_on_every_derivative_and_after_the_wrap() {
    let source = clip("source.mov");
    let theirs = thumbnails(&source, 10.0).unwrap();
    let in_frame = |t: f64| theirs.partition_point(|f| f.0 <= t + 1e-9) - 1;
    for (name, _, _) in derivatives() {
        let d = clip(name);
        let map = timemap::measure(&source, &d).unwrap();
        for (a, b) in [(0.5, 1.0), (0.75, 1.25), (1.25, 2.0), (1.0, 1.5)] {
            let mut p = Player::new(&d, Box::new(Software::default())).unwrap();
            p.set_span(Span { start: map.to_file(a), end: map.to_file(b), speed: 1.0 });
            let mut q = Frames::new(6);
            let lap = p.lap();
            for heard in [0.0, lap, 2.0 * lap] {
                let f = p.frame_for(heard, &mut q).unwrap().expect("a frame");
                let got = which(&theirs, &timemap::thumbnail(&f.picture));
                assert_eq!(got, in_frame(a), "{name}: in {a} s, heard {heard:.3} s – the source's frame {} at the in point", in_frame(a));
            }
        }
    }
}
