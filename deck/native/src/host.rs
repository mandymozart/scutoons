//! A deck-native window (#443): made the way the stage makes it – fullscreen borderless on a deck, 1280×720 windowed on
//! a dev machine –, its display's sizes for the settings page (#220), and the frame-rate cap's wait (#220: an early frame
//! waits, the loop sleeps until it is due). main.rs and the native editor's proof (#444) make their windows here.
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Fullscreen, Window};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// the windowed size on a dev machine
pub const WINDOWED: (f64, f64) = (1280.0, 720.0);

/// a window titled `title`: fullscreen borderless on the current monitor, or `WINDOWED` when `windowed`
pub fn window(el: &ActiveEventLoop, title: &str, windowed: bool) -> Result<Arc<Window>, winit::error::OsError> {
    let mut attrs = Window::default_attributes().with_title(title);
    if windowed {
        attrs = attrs.with_inner_size(winit::dpi::LogicalSize::new(WINDOWED.0, WINDOWED.1));
    } else {
        attrs = attrs.with_fullscreen(Some(Fullscreen::Borderless(None)));
    }
    el.create_window(attrs).map(Arc::new)
}

/// #220: the display's sizes from its modes (quality.rs `sizes`: largest first, each once; the largest is the hardware's
/// maximum); empty without a monitor
pub fn display_sizes(window: &Window) -> Vec<(u32, u32)> {
    window.current_monitor().map(|m| crate::quality::sizes(m.video_modes().map(|v| (v.size().width, v.size().height)).collect())).unwrap_or_default()
}

/// #220: the next frame in `ms` – no draw, no spin
pub fn wait(el: &ActiveEventLoop, ms: f32) {
    el.set_control_flow(ControlFlow::WaitUntil(Instant::now() + Duration::from_secs_f32(ms.max(0.0) / 1000.0)));
}
