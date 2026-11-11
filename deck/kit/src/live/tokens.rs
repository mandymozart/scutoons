//! The `live` module's token sets (#485; owner 2026-11-11 [ui-kit]: "Since the live mode is audience facing it will need
//! special style"): the base's token shape with the live looks' values – the HUD's arcade look the deck's menu wears
//! (#386), the reader's dark panel the deck's dialogs wear (#481). The editor's set is the base's default.
use crate::base::tokens::{Colors, Sizes, Tokens};
use crate::live::hud::{FINGER, GLYPH, ROW, SMALL};

const fn rgb(v: u32) -> [f32; 4] {
    [((v >> 16) & 255) as f32 / 255.0, ((v >> 8) & 255) as f32 / 255.0, (v & 255) as f32 / 255.0, 1.0]
}

/// The HUD's look: the black ground (a little see-through over the scene), the HUD's white, a muted grey for what is not
/// there yet, the red selection bar (`selected`), the pairing code's mint (`accent`), the dimmed scene behind a page
/// (`stage`).
pub fn hud() -> Tokens {
    let black = rgb(0x000000);
    Tokens {
        color: Colors {
            stage: [0.0, 0.0, 0.0, 0.55],
            surface: [0.0, 0.0, 0.0, 0.92],
            surface_head: black,
            line: rgb(0x3a3a3a),
            text: rgb(0xebebeb),
            muted: rgb(0x7a7a7a),
            control: black,
            hover: rgb(0x262626),
            pressed: rgb(0x262626),
            selected: rgb(0xe8302a),
            within: rgb(0x262626),
            accent: rgb(0x5ef2c8),
            ..Colors::default()
        },
        size: Sizes { edge: 1.0, pad: 4.0, font: GLYPH, small: SMALL, row: ROW, tree_row: FINGER, radius: 0.0, ..Sizes::default() },
    }
}

/// The reader's dark panel: ink white, paper black, the muted greys, the cyan accent; a control is 36 high, the lines 2
/// wide, the corners square.
pub fn panel() -> Tokens {
    let black = rgb(0x000000);
    Tokens {
        color: Colors {
            stage: black,
            surface: black,
            surface_head: black,
            line: rgb(0x555555),
            text: rgb(0xffffff),
            muted: rgb(0xa6a6a6),
            control: black,
            hover: rgb(0x222222),
            pressed: rgb(0x222222),
            selected: rgb(0x222222),
            within: rgb(0x333333),
            accent: rgb(0x00d1ff),
            // the reader's --state-warning and --state-error
            warn: rgb(0xff6600),
            danger: rgb(0xff1900),
            ..Colors::default()
        },
        size: Sizes { edge: 2.0, pad: 16.0, font: 14.0, small: 12.0, row: 36.0, radius: 0.0, ..Sizes::default() },
    }
}
