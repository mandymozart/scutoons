//! The fixture clips: ffmpeg's own test pattern and tone, made with (64 × 48, 12 fps, 1 s):
//!   SRC=testsrc2=size=64x48:rate=12:duration=1  TONE=sine=frequency=440:sample_rate=48000:duration=1
//!   ffmpeg -f lavfi -i $SRC -f lavfi -i $TONE -c:v libaom-av1 -strict -2 -cpu-used 8 -crf 40 -g 6 -pix_fmt yuv420p \
//!     -c:a aac -b:a 32k -ac 1 -map_metadata -1 -fflags +bitexact av1.mp4          (-c:a libvorbis … av1.webm)
//!   ffmpeg -f lavfi -i $SRC [-vf format=rgba] -c:v hap -format hap|hap_alpha|hap_q -map_metadata -1 -fflags +bitexact hap*.mov
//! Each with its content hash: a replaced file fails as "fixture changed", not as a decoder difference.

/// (file, its content hash, its golden line)
pub const FIXTURES: [(&str, u64, &str); 5] = [
    ("av1.mp4", 0xa7bbfdb2b9ee06ee, "12:b60700f99cc7c333 49152:085a6ff2cb05ddf9"),
    ("av1.webm", 0x701d286663ab92d4, "12:b60700f99cc7c333 48704:a62806f52b5b1e85"),
    ("hap.mov", 0xd33165abe9a41094, "12:83e10d6e1cf8d384 0:cbf29ce484222325"),
    ("hap_alpha.mov", 0xaaa7f2ec7dc09b84, "12:83e10d6e1cf8d384 0:cbf29ce484222325"),
    ("hap_q.mov", 0x9fd422aa537975ca, "12:a0c6f4bb0565528e 0:cbf29ce484222325"),
];

pub fn fixture(name: &str) -> Vec<u8> {
    let b = std::fs::read(format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).expect("the fixture");
    let want = FIXTURES.iter().find(|f| f.0 == name).map_or(0, |f| f.1);
    assert_eq!(deck_media::fingerprint(&b), want, "fixture changed: {name}");
    b
}
