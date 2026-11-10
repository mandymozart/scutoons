//! The window's top bar (#444, #423: every build shows its version top right, always; owner 2026-11-09: "the editor pane
//! overlaps the version number ... so perhapse this should go to the top"): the whole width, above the stage and the
//! panel, so no region can cover it. The version keeps its place, then the title; the status line gets what is left.
use deck_kit::controls::{self, Rect};
use deck_kit::draw::DrawList;
use deck_kit::tokens::Tokens;

/// the logo's side
const LOGO: f32 = 22.0;

/// what fits in a bar `width` wide: where the texts start and how many characters each may show
#[derive(Debug, PartialEq)]
struct Layout {
    title_cols: usize,
    status_x: f32,
    status_cols: usize,
    version_x: f32,
}

/// `left` = where the title starts, `switch` = the width of the Play / Wire switch after the title
fn lay(width: f32, adv: f32, pad: f32, left: f32, title: usize, switch: f32, version: usize) -> Layout {
    let version_x = (width - pad - version as f32 * adv).max(0.0);
    let title_cols = title.min(((version_x - left - pad) / adv).max(0.0) as usize);
    let status_x = left + title_cols as f32 * adv + 2.0 * pad + switch + pad;
    let status_cols = ((version_x - pad - status_x) / adv).max(0.0) as usize;
    Layout { title_cols, status_x, status_cols, version_x }
}

/// what the bar shows
pub struct Bar<'a> {
    /// #91: the workspaces (Build, Show, Perform) after the title, this one on
    pub workspaces: &'a [&'a str],
    pub workspace: usize,
    pub title: &'a str,
    pub status: &'a str,
    pub wire: bool,
    pub logo_frame: usize,
    pub pointer: (f32, f32),
    /// how much of a frame's time the CPU and the GPU work (0..1); the GPU where it can be measured
    pub cpu: f32,
    pub gpu: Option<f32>,
}

/// the bar; where its play button and its workspace pills landed
pub fn draw(ui: &mut DrawList, t: &Tokens, width: f32, b: &Bar) -> (Rect, Vec<Rect>) {
    let Bar { title, status, wire, logo_frame: frame, pointer, cpu, gpu, workspaces, workspace } = *b;
    let (h, pad, px) = (t.size.top, t.size.pad, t.size.font);
    ui.rect(0.0, 0.0, width, h, t.color.surface_head);
    ui.rect(0.0, h - 1.0, width, 1.0, t.color.line);
    ui.logo(pad, (h - LOGO) * 0.5, LOGO, frame);
    let left = pad + LOGO + pad;
    let adv = ui.advance(px);
    let pills_w: f32 = workspaces.iter().map(|w| ui.advance(t.size.small) * w.chars().count() as f32 + 2.0 * pad + 2.0).sum();
    let switch = t.size.row + pills_w + pad;
    let l = lay(width, adv, pad, left, title.chars().count(), switch, deck_native::BUILD.chars().count());
    let y = (h - px * 1.25) * 0.5;
    ui.text(left, y, px, &controls::fit(title, l.title_cols), t.color.text);
    ui.text(l.status_x, y, px, &controls::fit(status, l.status_cols), t.color.muted);
    ui.text(l.version_x, y, px, deck_native::BUILD, t.color.muted);
    // how busy: left of the version, where there is room
    let room = l.version_x - (l.status_x + status.chars().count() as f32 * adv) - 2.0 * pad;
    if room > 200.0 {
        let mut mx = l.version_x - 200.0;
        mx += controls::meter(ui, t, mx, y, "CPU", cpu) + 2.0 * pad;
        if let Some(g) = gpu {
            controls::meter(ui, t, mx, y, "GPU", g);
        }
    }
    let sx = left + l.title_cols as f32 * adv + 2.0 * pad;
    let pills = controls::pills(ui, t, sx, (h - t.size.row + 4.0) * 0.5, t.size.row - 4.0, workspaces, workspace);
    let sx = sx + pills_w + pad;
    // one button, as in Unreal: green starts playing, red stops it
    let at = Rect { x: sx, y: (h - t.size.row) * 0.5, w: t.size.row, h: t.size.row };
    let button = if wire {
        controls::icon_button(ui, t, at, "play-fill", t.color.go, pointer)
    } else {
        controls::icon_button(ui, t, at, "stop-fill", t.color.stop, pointer)
    };
    (button, pills)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_ends_at_the_right_edge_however_wide_the_window() {
        for width in [400.0, 1280.0, 3840.0] {
            let l = lay(width, 7.0, 12.0, 12.0, 23, 112.0, 27);
            assert_eq!(l.version_x + 27.0 * 7.0 + 12.0, width);
        }
    }

    #[test]
    fn nothing_reaches_the_version() {
        for width in [120.0, 300.0, 400.0, 900.0, 1280.0] {
            let l = lay(width, 7.0, 12.0, 12.0, 23, 112.0, 27);
            assert!(l.title_cols == 0 || 12.0 + l.title_cols as f32 * 7.0 <= l.version_x, "the title at {width}");
            assert!(l.status_cols == 0 || l.status_x + l.status_cols as f32 * 7.0 <= l.version_x, "the status at {width}");
        }
    }
}
