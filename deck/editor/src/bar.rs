//! The window's top bar (#444, #423: every build shows its version top right, always; owner 2026-11-09: "the editor pane
//! overlaps the version number ... so perhapse this should go to the top"): the whole width, above the stage and the
//! panel, so no region can cover it. The version keeps its place, then the title; the status line gets what is left.
use crate::tokens::Tokens;
use crate::ui::Ui;

const TITLE: &str = "SCUTOONS editor · proof";
/// the bar's font size against the panel's
const FONT: f32 = 0.8;

/// what fits in a bar `width` wide: where the texts start and how many characters each may show
#[derive(Debug, PartialEq)]
struct Layout {
    title_cols: usize,
    status_x: f32,
    status_cols: usize,
    version_x: f32,
}

fn lay(width: f32, adv: f32, pad: f32, title: usize, version: usize) -> Layout {
    let version_x = (width - pad - version as f32 * adv).max(0.0);
    let title_cols = title.min(((version_x - 2.0 * pad) / adv).max(0.0) as usize);
    let status_x = pad + title_cols as f32 * adv + 2.0 * pad;
    let status_cols = ((version_x - pad - status_x) / adv).max(0.0) as usize;
    Layout { title_cols, status_x, status_cols, version_x }
}

/// `text` in at most `cols` characters, an ellipsis where it is cut
fn fit(text: &str, cols: usize) -> String {
    if text.chars().count() <= cols {
        return text.to_string();
    }
    let mut s: String = text.chars().take(cols.saturating_sub(1)).collect();
    if cols > 0 {
        s.push('…');
    }
    s
}

/// the bar across the top of a window `width` wide
pub fn draw(ui: &mut Ui, t: &Tokens, width: f32, status: &str) {
    let (h, pad, px) = (t.size.bar, t.size.pad, t.size.font * FONT);
    ui.rect(0.0, 0.0, width, h, t.color.panel);
    ui.rect(0.0, h - 1.0, width, 1.0, t.color.divider);
    let l = lay(width, ui.atlas.advance(px), pad, TITLE.chars().count(), deck_native::BUILD.chars().count());
    let y = (h - px * 1.25) * 0.5;
    ui.text(pad, y, px, &fit(TITLE, l.title_cols), t.color.text);
    ui.text(l.status_x, y, px, &fit(status, l.status_cols), t.color.muted);
    ui.text(l.version_x, y, px, deck_native::BUILD, t.color.muted);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_ends_at_the_right_edge_however_wide_the_window() {
        for width in [400.0, 1280.0, 3840.0] {
            let l = lay(width, 7.0, 12.0, 23, 27);
            assert_eq!(l.version_x + 27.0 * 7.0 + 12.0, width);
        }
    }

    #[test]
    fn nothing_reaches_the_version() {
        for width in [120.0, 300.0, 400.0, 900.0, 1280.0] {
            let l = lay(width, 7.0, 12.0, 23, 27);
            assert!(l.title_cols == 0 || 12.0 + l.title_cols as f32 * 7.0 <= l.version_x, "the title at {width}");
            assert!(l.status_cols == 0 || l.status_x + l.status_cols as f32 * 7.0 <= l.version_x, "the status at {width}");
        }
    }

    #[test]
    fn a_long_status_is_cut_with_an_ellipsis() {
        assert_eq!(fit("tokens.toml", 7), "tokens…");
        assert_eq!(fit("short", 7), "short");
        assert_eq!(fit("äöü", 3), "äöü");
        assert_eq!(fit("anything", 0), "");
    }
}
