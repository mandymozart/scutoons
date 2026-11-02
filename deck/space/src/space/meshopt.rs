//! EXT_meshopt_compression's two codecs, decoded in plain Rust (the rooms' build compresses every buffer view with
//! meshopt – agents/research/LOCATION-SCENES.md §7; the `meshopt` crate binds the C++ library, this needs no compiler).
//! The bitstreams follow the extension's specification (KhronosGroup/glTF, extensions/2.0/Vendor/EXT_meshopt_compression,
//! "Appendix A: Bitstream"): mode ATTRIBUTES (vertex codec, version 0) and mode TRIANGLES (index codec, versions 0 and
//! 1). Mode INDICES (index sequences) and the filters (OCTAHEDRAL, QUATERNION, EXPONENTIAL) are not used by the rooms'
//! build and are refused with a clear error – a room exported with them can be re-exported without (deck/native/README.md).
use anyhow::{bail, ensure, Result};

const VERTEX_HEADER: u8 = 0xa0;
const INDEX_HEADER: u8 = 0xe0;
const BYTE_GROUP: usize = 16;
const BLOCK_MAX: usize = 256;
const BLOCK_BYTES: usize = 8192;
const TAIL_MAX: usize = 32;

/// Mode ATTRIBUTES: `count` elements of `stride` bytes (stride a multiple of 4, ≤ 256).
pub fn decode_vertices(src: &[u8], count: usize, stride: usize) -> Result<Vec<u8>> {
    ensure!(stride > 0 && stride <= 256 && stride % 4 == 0, "meshopt: a vertex stride of {stride}");
    ensure!(src.len() > stride, "meshopt: the vertex stream is too short");
    let version = src[0] & 0x0f;
    ensure!(src[0] & 0xf0 == VERTEX_HEADER && version == 0, "meshopt: vertex codec header {:#x} (only version 0)", src[0]);
    let mut out = vec![0u8; count * stride];
    let mut last = [0u8; 256];
    last[..stride].copy_from_slice(&src[src.len() - stride..]);
    let block = ((BLOCK_BYTES / stride) & !(BYTE_GROUP - 1)).min(BLOCK_MAX);
    let mut pos = 1;
    let mut buf = [0u8; BLOCK_MAX];
    let mut start = 0;
    while start < count {
        let n = block.min(count - start);
        let aligned = n.div_ceil(BYTE_GROUP) * BYTE_GROUP;
        for k in 0..stride {
            pos = decode_bytes(src, pos, &mut buf[..aligned])?;
            let mut p = last[k];
            for i in 0..n {
                let z = buf[i];
                // un-zigzag, then the delta from the previous vertex's byte
                p = p.wrapping_add((z >> 1) ^ (z & 1).wrapping_neg());
                out[(start + i) * stride + k] = p;
            }
            last[k] = p;
        }
        start += n;
    }
    let tail = stride.max(TAIL_MAX);
    ensure!(src.len() - pos == tail, "meshopt: {} bytes left after the vertex blocks, expected the {tail}-byte tail", src.len() - pos);
    Ok(out)
}

/// One byte stream of a block: a 2-bit header per 16-byte group (0 = zeros, 1 = 2 bits, 2 = 4 bits, 3 = raw bytes; an
/// all-ones value is an escape – the byte follows the packed group).
fn decode_bytes(src: &[u8], mut pos: usize, out: &mut [u8]) -> Result<usize> {
    let groups = out.len() / BYTE_GROUP;
    let header = pos;
    pos += groups.div_ceil(4);
    ensure!(pos <= src.len(), "meshopt: a truncated byte group header");
    for g in 0..groups {
        let bits = (src[header + g / 4] >> ((g % 4) * 2)) & 3;
        let dst = &mut out[g * BYTE_GROUP..(g + 1) * BYTE_GROUP];
        match bits {
            0 => dst.fill(0),
            3 => {
                ensure!(pos + BYTE_GROUP <= src.len(), "meshopt: a truncated raw byte group");
                dst.copy_from_slice(&src[pos..pos + BYTE_GROUP]);
                pos += BYTE_GROUP;
            }
            _ => {
                let width = 1usize << bits; // 2 or 4 bits per value
                let packed = BYTE_GROUP * width / 8;
                ensure!(pos + packed <= src.len(), "meshopt: a truncated packed byte group");
                let escape = (1u8 << width) - 1;
                let mut var = pos + packed;
                for (i, d) in dst.iter_mut().enumerate() {
                    let bit = i * width;
                    let v = (src[pos + bit / 8] >> (8 - width - bit % 8)) & escape;
                    *d = if v == escape {
                        ensure!(var < src.len(), "meshopt: a truncated escape byte");
                        var += 1;
                        src[var - 1]
                    } else {
                        v
                    };
                }
                pos = var;
            }
        }
    }
    Ok(pos)
}

/// Mode TRIANGLES: `count` indices (a multiple of 3) as u32, whatever the stored width.
pub fn decode_triangles(src: &[u8], count: usize) -> Result<Vec<u32>> {
    ensure!(count % 3 == 0, "meshopt: {count} indices are not triangles");
    let tris = count / 3;
    ensure!(src.len() >= 1 + tris + 16, "meshopt: the index stream is too short");
    let version = src[0] & 0x0f;
    ensure!(src[0] & 0xf0 == INDEX_HEADER && version <= 1, "meshopt: index codec header {:#x}", src[0]);
    let fecmax = if version >= 1 { 13 } else { 15 };
    let mut edges = [[0u32; 2]; 16];
    let mut verts = [0u32; 16];
    let (mut eo, mut vo) = (0usize, 0usize);
    let (mut next, mut last) = (0u32, 0u32);
    let codeaux = src.len() - 16;
    let mut data = 1 + tris;
    let mut out = Vec::with_capacity(count);
    let push_edge = |e: &mut [[u32; 2]; 16], o: &mut usize, a: u32, b: u32| {
        e[*o] = [a, b];
        *o = (*o + 1) & 15;
    };
    let push_vert = |v: &mut [u32; 16], o: &mut usize, x: u32, cond: bool| {
        v[*o] = x;
        *o = (*o + cond as usize) & 15;
    };
    for t in 0..tris {
        ensure!(data <= codeaux, "meshopt: the index data runs into the code table");
        let code = src[1 + t];
        if code < 0xf0 {
            let fe = (code >> 4) as usize;
            let [a, b] = edges[(eo + 15 - fe) & 15];
            let fec = (code & 15) as usize;
            let c = if fec < fecmax {
                let c = if fec == 0 { next } else { verts[(vo + 15 - fec) & 15] };
                if fec == 0 {
                    next += 1;
                }
                push_vert(&mut verts, &mut vo, c, fec == 0);
                c
            } else {
                last = if fec != 15 {
                    // 13, 14 → last − 1, last + 1
                    last.wrapping_add((fec as i32 - (fec as i32 ^ 3)) as u32)
                } else {
                    decode_index(src, &mut data, last)?
                };
                push_vert(&mut verts, &mut vo, last, true);
                last
            };
            push_edge(&mut edges, &mut eo, c, b);
            push_edge(&mut edges, &mut eo, a, c);
            out.extend_from_slice(&[a, b, c]);
        } else if code < 0xfe {
            let aux = src[codeaux + (code & 15) as usize];
            let (feb, fec) = ((aux >> 4) as usize, (aux & 15) as usize);
            let a = next;
            next += 1;
            let b = if feb == 0 { next } else { verts[(vo + 16 - feb) & 15] };
            if feb == 0 {
                next += 1;
            }
            let c = if fec == 0 { next } else { verts[(vo + 16 - fec) & 15] };
            if fec == 0 {
                next += 1;
            }
            out.extend_from_slice(&[a, b, c]);
            push_vert(&mut verts, &mut vo, a, true);
            push_vert(&mut verts, &mut vo, b, feb == 0);
            push_vert(&mut verts, &mut vo, c, fec == 0);
            push_edge(&mut edges, &mut eo, b, a);
            push_edge(&mut edges, &mut eo, c, b);
            push_edge(&mut edges, &mut eo, a, c);
        } else {
            let aux = src[data];
            data += 1;
            if aux == 0 {
                next = 0;
            }
            let (fea, feb, fec) = (if code == 0xfe { 0 } else { 15 }, (aux >> 4) as usize, (aux & 15) as usize);
            let mut take = |fe: usize| {
                if fe == 0 {
                    next += 1;
                    next - 1
                } else {
                    verts[(vo + 16 - fe) & 15]
                }
            };
            let mut a = if fea == 0 { take(0) } else { 0 };
            let mut b = take(feb);
            let mut c = take(fec);
            if fea == 15 {
                last = decode_index(src, &mut data, last)?;
                a = last;
            }
            if feb == 15 {
                last = decode_index(src, &mut data, last)?;
                b = last;
            }
            if fec == 15 {
                last = decode_index(src, &mut data, last)?;
                c = last;
            }
            out.extend_from_slice(&[a, b, c]);
            push_vert(&mut verts, &mut vo, a, true);
            push_vert(&mut verts, &mut vo, b, feb == 0 || feb == 15);
            push_vert(&mut verts, &mut vo, c, fec == 0 || fec == 15);
            push_edge(&mut edges, &mut eo, b, a);
            push_edge(&mut edges, &mut eo, c, b);
            push_edge(&mut edges, &mut eo, a, c);
        }
    }
    ensure!(data == codeaux, "meshopt: the index data ends at {data}, the code table starts at {codeaux}");
    Ok(out)
}

/// a free index: a zigzag varint delta from the last free index
fn decode_index(src: &[u8], pos: &mut usize, last: u32) -> Result<u32> {
    let mut v = 0u32;
    for shift in (0..35).step_by(7) {
        let Some(&byte) = src.get(*pos) else { bail!("meshopt: a truncated index") };
        *pos += 1;
        v |= ((byte & 127) as u32) << shift;
        if byte < 128 {
            break;
        }
    }
    let d = (v >> 1) ^ (v & 1).wrapping_neg();
    Ok(last.wrapping_add(d))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_deltas_repeat_the_tail_vertex() {
        // 20 vertices of 4 bytes: one block of 32 aligned bytes per byte stream = 2 groups, header 0 → zeros
        // one header byte per byte stream for its two groups (both 0)
        let mut s = vec![VERTEX_HEADER, 0, 0, 0, 0];
        let mut tail = vec![0u8; 28];
        tail.extend_from_slice(&[1, 2, 3, 4]);
        s.extend_from_slice(&tail);
        let v = decode_vertices(&s, 20, 4).unwrap();
        assert!(v.chunks(4).all(|c| c == [1, 2, 3, 4]));
    }

    #[test]
    fn packed_groups_and_escapes_decode() {
        // 16 vertices × 4 bytes. Byte 0: 2-bit group with zigzag deltas 2 (= +1) each → 1..=16 from a base 0;
        // byte 1: raw group; bytes 2, 3: 4-bit group, value 15 escapes to the next byte.
        let mut s = vec![VERTEX_HEADER];
        s.push(1); // header: group 0 = 2 bits
        s.extend_from_slice(&[0b1010_1010; 4]); // sixteen 2s
        s.push(3);
        s.extend_from_slice(&[4u8; 16]); // zigzag 4 = +2
        for _ in 0..2 {
            s.push(2);
            let mut packed = [0u8; 8];
            packed[0] = 0xf0; // first value escaped, second 0
            s.extend_from_slice(&packed);
            s.push(200); // the escape byte (zigzag 200 = +100)
        }
        s.extend_from_slice(&[0u8; 32]);
        let v = decode_vertices(&s, 16, 4).unwrap();
        let col = |k: usize| v.chunks(4).map(|c| c[k]).collect::<Vec<u8>>();
        assert_eq!(col(0), (1..=16).collect::<Vec<u8>>());
        assert_eq!(col(1), (1..=16).map(|i| i * 2).collect::<Vec<u8>>());
        assert_eq!(&col(2)[..2], &[100, 100]);
    }

    #[test]
    fn a_fresh_triangle_strip_decodes() {
        // two triangles: (0 1 2) as a code-table triangle with three new vertices, then (2 1 3) as edge 0 (c, b
        // pushed last: (2, 1)) + a new vertex
        let mut s = vec![INDEX_HEADER | 1, 0xf0, 0x10];
        let mut table = [0u8; 16];
        table[0] = 0x00; // feb = 0, fec = 0: both new
        // data: none; code 0x10 = fe 1 (the edge before the last pushed), fec 0
        s.extend_from_slice(&table);
        let idx = decode_triangles(&s, 6).unwrap();
        assert_eq!(&idx[..3], &[0, 1, 2]);
        // after (0 1 2) the edge fifo holds (1 0), (2 1), (0 2); fe 1 = the second newest = (2, 1)
        assert_eq!(&idx[3..], &[2, 1, 3]);
    }

    #[test]
    fn bad_headers_are_errors() {
        assert!(decode_vertices(&[0x50; 64], 4, 4).is_err());
        assert!(decode_triangles(&[0x50; 64], 3).is_err());
    }
}
