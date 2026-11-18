//! The editor's own window frame onto winit: which platform draws it, the undecorated window, the resize band's
//! directions and cursors. Where a press lands and how the buttons look is deck-kit's (`editor::window`).
use deck_kit::editor::window::{Button, Clicks, Edge, Frame, Side, Zone};
use std::sync::Arc;
use winit::event_loop::ActiveEventLoop;
use winit::window::{CursorIcon, ResizeDirection, Window, WindowAttributes};

/// where the editor draws the window buttons itself: Windows, windowed. Fullscreen (the deck) and Linux keep the
/// system's. macOS later: `Side::Left`, with a transparent, full-size title bar so the system's traffic lights sit in the row.
pub fn own(fullscreen: bool) -> Option<Side> {
    (cfg!(windows) && !fullscreen).then_some(Side::Right)
}

/// the window without the system's title bar and frame, keeping its shadow
pub fn undecorated(attrs: WindowAttributes) -> WindowAttributes {
    let attrs = attrs.with_decorations(false);
    #[cfg(windows)]
    let attrs = {
        use winit::platform::windows::WindowAttributesExtWindows;
        attrs.with_undecorated_shadow(true)
    };
    attrs
}

fn direction(e: Edge) -> ResizeDirection {
    match e {
        Edge::North => ResizeDirection::North,
        Edge::South => ResizeDirection::South,
        Edge::East => ResizeDirection::East,
        Edge::West => ResizeDirection::West,
        Edge::NorthEast => ResizeDirection::NorthEast,
        Edge::NorthWest => ResizeDirection::NorthWest,
        Edge::SouthEast => ResizeDirection::SouthEast,
        Edge::SouthWest => ResizeDirection::SouthWest,
    }
}

/// the resize cursor over the band, else none of the frame's
pub fn cursor(f: &Frame, (x, y): (f32, f32)) -> Option<CursorIcon> {
    let Zone::Resize(e) = f.hit(x, y) else { return None };
    Some(match e {
        Edge::North | Edge::South => CursorIcon::NsResize,
        Edge::East | Edge::West => CursorIcon::EwResize,
        Edge::NorthWest | Edge::SouthEast => CursorIcon::NwseResize,
        Edge::NorthEast | Edge::SouthWest => CursorIcon::NeswResize,
    })
}

/// the frame's state between frames: the last drawn toolbar's places, the run's last press, a window button held
#[derive(Default)]
pub struct Own {
    pub frame: Frame,
    clicks: Clicks,
    held: Option<Button>,
}

impl Own {
    /// a press at `at`, `now` seconds: true when the frame took it (resize, a button, a drag or a double click)
    pub fn press(&mut self, w: &Window, at: (f32, f32), now: f64) -> bool {
        match self.frame.hit(at.0, at.1) {
            Zone::Resize(e) => {
                if let Err(err) = w.drag_resize_window(direction(e)) {
                    tracing::warn!("resize: {err}");
                }
            }
            Zone::Button(b) => self.held = Some(b),
            Zone::Drag if self.clicks.press(now, at) => w.set_maximized(!w.is_maximized()),
            Zone::Drag => {
                if let Err(err) = w.drag_window() {
                    tracing::warn!("drag: {err}");
                }
            }
            Zone::App => return false,
        }
        true
    }

    /// the release: a held button acts when it is let go over itself; true when the frame took it
    pub fn release(&mut self, w: &Arc<Window>, el: &ActiveEventLoop, at: (f32, f32)) -> bool {
        let Some(b) = self.held.take() else { return false };
        if self.frame.hit(at.0, at.1) == Zone::Button(b) {
            match b {
                Button::Minimise => w.set_minimized(true),
                Button::Maximise => w.set_maximized(!w.is_maximized()),
                Button::Close => el.exit(),
            }
        }
        true
    }
}
