//! H.264 / HEVC samples as a byte stream with start codes (Annex B), what hardware decoders (Media Foundation, VA-API)
//! take: MP4 and Matroska store each NAL unit behind its length and the parameter sets in the configuration record
//! (`avcC`, `hvcC`); here the lengths become start codes and the parameter sets go before every key frame, so a decoder
//! can start at any key frame (a seek, a loop).
use crate::demux::Codec;
use crate::{err, Error};

const START: [u8; 4] = [0, 0, 0, 1];

/// A stream's framing: the bytes of a NAL unit's length, the parameter sets (start codes included).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnexB {
    length: usize,
    sets: Vec<u8>,
}

fn u16_at(b: &[u8], at: usize) -> Result<usize, Error> {
    match b.get(at..at + 2) {
        Some(x) => Ok(usize::from(u16::from_be_bytes([x[0], x[1]]))),
        None => err("the configuration record ends early"),
    }
}

/// a parameter set of `len` bytes at `at`, start-coded into `sets`; returns where the next begins
fn take(b: &[u8], at: usize, sets: &mut Vec<u8>) -> Result<usize, Error> {
    let len = u16_at(b, at)?;
    let Some(nal) = b.get(at + 2..at + 2 + len) else { return err("a parameter set runs past the configuration record") };
    sets.extend_from_slice(&START);
    sets.extend_from_slice(nal);
    Ok(at + 2 + len)
}

impl AnnexB {
    /// The framing of a `codec` stream from its configuration record. None: no record (the samples are Annex B already),
    /// or a codec without NAL units.
    pub fn new(codec: &Codec, config: &[u8]) -> Result<Option<AnnexB>, Error> {
        if config.is_empty() {
            return Ok(None);
        }
        let mut sets = Vec::new();
        let length = match codec {
            Codec::H264 => {
                if config.len() < 7 {
                    return err("H.264: the avcC record is too short");
                }
                let mut at = 6;
                for _ in 0..config[5] & 0x1f {
                    at = take(config, at, &mut sets)?;
                }
                let pps = *config.get(at).ok_or_else(|| Error("H.264: avcC ends before its picture parameter sets".into()))?;
                at += 1;
                for _ in 0..pps {
                    at = take(config, at, &mut sets)?;
                }
                usize::from(config[4] & 3) + 1
            }
            Codec::Hevc => {
                if config.len() < 23 {
                    return err("HEVC: the hvcC record is too short");
                }
                let mut at = 23;
                for _ in 0..config[22] {
                    let n = u16_at(config, at + 1)?;
                    at += 3;
                    for _ in 0..n {
                        at = take(config, at, &mut sets)?;
                    }
                }
                usize::from(config[21] & 3) + 1
            }
            _ => return Ok(None),
        };
        if length == 3 {
            return err("NAL units behind 3-byte lengths are not read");
        }
        Ok(Some(AnnexB { length, sets }))
    }

    /// `sample`'s NAL units with start codes into `out` (cleared first), the parameter sets before a key frame.
    pub fn convert(&self, sample: &[u8], key: bool, out: &mut Vec<u8>) -> Result<(), Error> {
        out.clear();
        if key {
            out.extend_from_slice(&self.sets);
        }
        let mut at = 0;
        while at < sample.len() {
            let Some(len) = sample.get(at..at + self.length) else { return err("a NAL unit's length runs past its sample") };
            let len = len.iter().fold(0usize, |n, &b| n << 8 | usize::from(b));
            at += self.length;
            let Some(nal) = sample.get(at..at + len) else { return err("a NAL unit runs past its sample") };
            out.extend_from_slice(&START);
            out.extend_from_slice(nal);
            at += len;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avcc_lengths_become_start_codes_with_the_sets_before_a_key_frame() {
        // version, profile, compatibility, level, 4-byte lengths, one SPS (3 bytes), one PPS (2 bytes)
        let avcc = [1, 100, 0, 40, 0xff, 0xe1, 0, 3, 0x67, 1, 2, 1, 0, 2, 0x68, 9];
        let a = AnnexB::new(&Codec::H264, &avcc).unwrap().unwrap();
        let sample = [0, 0, 0, 2, 0x65, 7, 0, 0, 0, 1, 0x06];
        let mut out = Vec::new();
        a.convert(&sample, true, &mut out).unwrap();
        assert_eq!(out, [0, 0, 0, 1, 0x67, 1, 2, 0, 0, 0, 1, 0x68, 9, 0, 0, 0, 1, 0x65, 7, 0, 0, 0, 1, 0x06]);
        a.convert(&sample, false, &mut out).unwrap();
        assert_eq!(out, [0, 0, 0, 1, 0x65, 7, 0, 0, 0, 1, 0x06], "no sets before a frame that is not a key");
        assert!(a.convert(&[0, 0, 0, 9, 1], false, &mut out).is_err(), "a length past the sample");
    }

    #[test]
    fn hvcc_arrays_give_the_vps_sps_and_pps() {
        let mut hvcc = vec![0u8; 21];
        hvcc[0] = 1;
        // 2-byte lengths, three arrays of one NAL unit each
        hvcc.extend([0xfd, 3]);
        for (kind, nal) in [(32u8, [0x40, 1]), (33, [0x42, 1]), (34, [0x44, 1])] {
            hvcc.extend([0x80 | kind, 0, 1, 0, 2]);
            hvcc.extend(nal);
        }
        let a = AnnexB::new(&Codec::Hevc, &hvcc).unwrap().unwrap();
        let mut out = Vec::new();
        a.convert(&[0, 3, 0x26, 1, 5], true, &mut out).unwrap();
        assert_eq!(out, [0, 0, 0, 1, 0x40, 1, 0, 0, 0, 1, 0x42, 1, 0, 0, 0, 1, 0x44, 1, 0, 0, 0, 1, 0x26, 1, 5]);
        assert_eq!(AnnexB::new(&Codec::Av1, &[1, 2, 3]).unwrap(), None, "AV1 has no NAL units");
        assert_eq!(AnnexB::new(&Codec::H264, &[]).unwrap(), None, "no record: Annex B already");
    }
}
