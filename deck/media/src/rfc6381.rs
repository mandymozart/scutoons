//! Codec strings (RFC 6381) and the out-of-band description: what a platform decoder that is configured by name takes
//! (the browser's WebCodecs). The core asks the questions (`probes`) and names each stream (`codec_string`); an adapter
//! only passes them on.
use crate::backend::Stream;
use crate::demux::Codec;

/// The codecs the core asks a platform about, each by a string a 1080p derivative of it carries.
pub fn probes() -> Vec<(Codec, &'static str)> {
    vec![(Codec::H264, "avc1.640028"), (Codec::Hevc, "hvc1.1.6.L120.90"), (Codec::Vp9, "vp09.00.40.08"), (Codec::Av1, "av01.0.08M.08")]
}

/// the codec of a probe's string, None: a string the core did not ask
pub fn probed(s: &str) -> Option<Codec> {
    probes().into_iter().find(|(_, p)| *p == s).map(|(c, _)| c)
}

/// `stream`'s codec string, read from its configuration record; without a record, the probe's string of its codec.
/// None: a codec no platform decoder is asked for.
pub fn codec_string(stream: &Stream) -> Option<String> {
    let c = &stream.config;
    let read = match stream.codec {
        Codec::H264 if c.len() >= 4 => Some(format!("avc1.{:02x}{:02x}{:02x}", c[1], c[2], c[3])),
        Codec::Hevc if c.len() >= 13 => Some(hevc(c)),
        // MP4's vpcC (a full box: version 1, no flags); Matroska's private data for VP9 is another form
        Codec::Vp9 if c.len() >= 7 && c[1..4] == [0, 0, 0] => Some(format!("vp09.{:02}.{:02}.{:02}", c[4], c[5], c[6] >> 4)),
        Codec::Av1 if c.len() >= 3 => {
            let depth = match (c[2] & 0x40 != 0, c[2] & 0x20 != 0) {
                (true, true) => 12,
                (true, false) => 10,
                _ => 8,
            };
            Some(format!("av01.{}.{:02}{}.{depth:02}", c[1] >> 5, c[1] & 0x1f, if c[2] & 0x80 != 0 { 'H' } else { 'M' }))
        }
        _ => None,
    };
    read.or_else(|| probes().into_iter().find(|(k, _)| *k == stream.codec).map(|(_, p)| p.to_string()))
}

/// ISO/IEC 14496-15 Annex E: `hvc1.<space><profile>.<compatibility, bits reversed>.<tier><level>.<constraints>`
fn hevc(c: &[u8]) -> String {
    let space = ["", "A", "B", "C"][usize::from(c[1] >> 6)];
    let tier = if c[1] & 0x20 != 0 { 'H' } else { 'L' };
    let compat = u32::from_be_bytes([c[2], c[3], c[4], c[5]]).reverse_bits();
    let mut constraints: Vec<u8> = c[6..12].to_vec();
    while constraints.last() == Some(&0) {
        constraints.pop();
    }
    let mut s = format!("hvc1.{space}{}.{compat:x}.{tier}{}", c[1] & 0x1f, c[12]);
    for b in constraints {
        s.push_str(&format!(".{b:x}"));
    }
    s
}

/// The configuration record a decoder takes beside the samples: H.264's `avcC` and HEVC's `hvcC` (their samples carry
/// NAL units behind lengths); None: the samples stand alone (VP9, AV1, Annex B streams).
pub fn description(stream: &Stream) -> Option<&[u8]> {
    (matches!(stream.codec, Codec::H264 | Codec::Hevc) && !stream.config.is_empty()).then_some(&stream.config[..])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stream(codec: Codec, config: &[u8]) -> Stream {
        Stream { codec, width: 1920, height: 1080, config: config.to_vec() }
    }

    #[test]
    fn each_stream_is_named_by_its_record() {
        let avcc = [1, 100, 0, 40, 0xff, 0xe1, 0, 3, 0x67, 1, 2, 1, 0, 2, 0x68, 9];
        assert_eq!(codec_string(&stream(Codec::H264, &avcc)).unwrap(), "avc1.640028");
        assert_eq!(description(&stream(Codec::H264, &avcc)), Some(&avcc[..]));
        // Main profile, compatibility flags 0x60000000, level 4.0 (120), progressive + frame-only (0x90)
        let mut hvcc = vec![1, 0x01, 0x60, 0, 0, 0, 0x90, 0, 0, 0, 0, 0, 120];
        hvcc.extend([0; 10]);
        assert_eq!(codec_string(&stream(Codec::Hevc, &hvcc)).unwrap(), "hvc1.1.6.L120.90");
        // vpcC: version + flags, profile 0, level 4.1, 8 bits
        assert_eq!(codec_string(&stream(Codec::Vp9, &[1, 0, 0, 0, 0, 41, 0x80, 0])).unwrap(), "vp09.00.41.08");
        assert_eq!(description(&stream(Codec::Vp9, &[1, 0, 0, 0, 0, 41, 0x80, 0])), None);
        // av1C: marker + version, Main profile level 4.0 (8), Main tier, 8 bits
        assert_eq!(codec_string(&stream(Codec::Av1, &[0x81, 0x08, 0x0c, 0])).unwrap(), "av01.0.08M.08");
        assert_eq!(codec_string(&stream(Codec::Av1, &[0x81, 0x29, 0xc0, 0])).unwrap(), "av01.1.09H.10");
    }

    #[test]
    fn without_a_record_the_probes_string_and_every_probe_reads_back() {
        assert_eq!(codec_string(&stream(Codec::H264, &[])).unwrap(), "avc1.640028");
        assert_eq!(description(&stream(Codec::H264, &[])), None, "Annex B: nothing beside");
        assert_eq!(codec_string(&stream(Codec::Hap(crate::Hap::Rgb), &[])), None);
        for (c, s) in probes() {
            assert_eq!(probed(s), Some(c));
        }
        assert_eq!(probed("mp4a.40.2"), None);
    }
}
