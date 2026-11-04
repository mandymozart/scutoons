//! The vocabulary of the layout tables (#372): a field of a binary record is a name, a byte offset and a type. The tables
//! themselves sit next to the code that reads and writes the bytes (wire.rs, signals.rs; the state block in clock.rs) –
//! `src/bin/codecs-ts.rs` writes shared/live-core's wire.ts, signals.ts and clock-state.ts from them, so the browser's
//! codecs cannot drift from the core's layout.

/// A field's type in a record, little-endian.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ty {
    U8,
    U16,
    U32,
    F32,
}

impl Ty {
    /// the bytes it takes
    pub const fn size(self) -> usize {
        match self {
            Ty::U8 => 1,
            Ty::U16 => 2,
            Ty::U32 | Ty::F32 => 4,
        }
    }
}

/// One field of a record's header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Field {
    pub name: &'static str,
    pub at: usize,
    pub ty: Ty,
}

impl Field {
    pub const fn new(name: &'static str, at: usize, ty: Ty) -> Field {
        Field { name, at, ty }
    }

    /// the field of a record that starts at `bytes[0]`, as a number (f32 as its value)
    pub fn read(&self, bytes: &[u8]) -> f64 {
        let b = &bytes[self.at..self.at + self.ty.size()];
        match self.ty {
            Ty::U8 => f64::from(b[0]),
            Ty::U16 => f64::from(u16::from_le_bytes([b[0], b[1]])),
            Ty::U32 => f64::from(u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
            Ty::F32 => f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]])),
        }
    }
}

/// `lower_camel` for the TypeScript names (`per_bar` → `perBar`).
pub fn camel(snake: &str) -> String {
    let mut out = String::new();
    let mut up = false;
    for c in snake.chars() {
        if c == '_' {
            up = true;
        } else if up {
            out.extend(c.to_uppercase());
            up = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// `UPPER_SNAKE` for the TypeScript constants (`per_bar` → `PER_BAR`).
pub fn upper(snake: &str) -> String {
    snake.to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(camel("per_bar"), "perBar");
        assert_eq!(camel("heard_ms"), "heardMs");
        assert_eq!(camel("beat"), "beat");
        assert_eq!(upper("midi_lost"), "MIDI_LOST");
    }

    #[test]
    fn fields_read_little_endian() {
        let b = [1u8, 2, 3, 0, 0x78, 0x56, 0x34, 0x12, 0, 0, 0x80, 0x3f];
        assert_eq!(Field::new("a", 0, Ty::U8).read(&b), 1.0);
        assert_eq!(Field::new("b", 2, Ty::U16).read(&b), 3.0);
        assert_eq!(Field::new("c", 4, Ty::U32).read(&b), f64::from(0x1234_5678u32));
        assert_eq!(Field::new("d", 8, Ty::F32).read(&b), 1.0);
    }
}
