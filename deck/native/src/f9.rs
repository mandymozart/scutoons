//! #609: F9's adapter half – the stage's state (its relay, its scene, the hand-off it came with) into deck-native's
//! handoff.rs, and its answer back: quit for the editor, or the refusal on the HUD. A child of main.rs (it reads the App).
use super::App;
use std::time::Instant;
use winit::event_loop::ActiveEventLoop;

impl App {
    /// #609: F9 – the editor instead of the stage, on this session and scene (handoff.rs); a lean deck, a locked one or a
    /// failed start stays and says why on the HUD
    pub(super) fn hand_off(&mut self, el: &ActiveEventLoop) {
        use deck_native::handoff::{self, Handoff};
        let h = Handoff {
            relay: self.args.relay.clone(),
            scene: self.current_scene().map(str::to_owned),
            ..self.args.handoff.clone()
        };
        match handoff::go(handoff::App::Deck, h, handoff::own_args(std::env::args().skip(1)), &deck_native::switcher::deck_toml()) {
            Ok(code) => {
                self.exit = code;
                el.exit();
            }
            Err(r) => {
                tracing::info!("{}", r.sentence());
                self.refusal = Some((r, Instant::now()));
                self.hud_at = -1.0;
            }
        }
    }
}
