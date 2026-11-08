//! #438: how an `Envelope` crosses QUIC – once, here, for the relay's door and every client (RULES A4). The reliable lane
//! is one bidirectional stream of newline-delimited JSON (JSON escapes its newlines, so a line is a message); only
//! `deck.values` may take a datagram, and only when it fits `DATAGRAM_MAX`.

/// a reliable line longer than this ends the connection
pub const MAX_LINE: usize = 1 << 20;
/// the largest envelope sent as a datagram; a longer one takes the stream. QUIC guarantees 1200 bytes of path, minus its
/// own and WebTransport's headers.
pub const DATAGRAM_MAX: usize = 1100;

/// `text` as one line of the stream
pub fn line(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len() + 1);
    out.extend_from_slice(text.as_bytes());
    out.push(b'\n');
    out
}

/// The stream's bytes, as they come, back into messages.
#[derive(Debug, Clone, Default)]
pub struct Lines {
    buf: Vec<u8>,
}

/// a line over `MAX_LINE`: the peer is not speaking this protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooLong;

impl Lines {
    /// The complete lines `bytes` finishes, in order; a part of one stays for the next call. Bytes that are not UTF-8
    /// come through replaced, and fail as JSON where any bad message fails.
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<String>, TooLong> {
        let mut out = Vec::new();
        let mut rest = bytes;
        while let Some(end) = rest.iter().position(|b| *b == b'\n') {
            self.buf.extend_from_slice(&rest[..end]);
            rest = &rest[end + 1..];
            if self.buf.len() > MAX_LINE {
                return Err(TooLong);
            }
            out.push(String::from_utf8_lossy(&self.buf).into_owned());
            self.buf.clear();
        }
        self.buf.extend_from_slice(rest);
        if self.buf.len() > MAX_LINE {
            return Err(TooLong);
        }
        Ok(out)
    }

    pub fn clear(&mut self) {
        self.buf.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_cross_any_chunking() {
        let mut l = Lines::default();
        assert_eq!(l.push(b"{\"a\"").unwrap(), Vec::<String>::new());
        assert_eq!(l.push(b":1}\n{\"b\":2}\n{\"c").unwrap(), ["{\"a\":1}", "{\"b\":2}"]);
        assert_eq!(l.push(b"\":3}\n").unwrap(), ["{\"c\":3}"]);
    }

    #[test]
    fn a_framed_message_comes_back_whole() {
        let text = serde_json::json!({ "say": "line one\nline two" }).to_string();
        let mut l = Lines::default();
        assert_eq!(l.push(&line(&text)).unwrap(), [text]);
    }

    #[test]
    fn a_line_past_the_limit_is_refused() {
        let mut l = Lines::default();
        assert_eq!(l.push(&vec![b'x'; MAX_LINE + 1]), Err(TooLong));
    }
}
