//! F9's and OPEN CREATOR's adapter half – the stage's state (its relay, its scene, the hand-off it came with) into
//! deck-native's handoff.rs, and its answer back: quit for the editor or the browser, or the refusal on the HUD. A child
//! of main.rs (it reads the App).
use super::App;
use std::time::Instant;
use winit::event_loop::ActiveEventLoop;

impl App {
    /// #609: F9 – the editor instead of the stage, on this session and scene (handoff.rs); a lean deck, a locked one or a
    /// failed start stays and says why on the HUD
    pub(super) fn hand_off(&mut self, el: &ActiveEventLoop) {
        use deck_native::handoff;
        let done = handoff::go(handoff::App::Deck, self.leaving(), handoff::own_args(std::env::args().skip(1)), &deck_native::switcher::deck_toml());
        self.leave(el, done);
    }

    /// the menu's OPEN CREATOR: deck-session opens the browser on the creator and starts the deck again on this session
    /// and scene once it closes
    pub(super) fn open_creator(&mut self, el: &ActiveEventLoop) {
        use deck_native::handoff;
        let done = handoff::creator(self.leaving(), handoff::own_args(std::env::args().skip(1)));
        self.leave(el, done);
    }

    /// what the stage hands over when it leaves
    fn leaving(&self) -> deck_native::handoff::Handoff {
        deck_native::handoff::Handoff {
            relay: self.args.relay.clone(),
            scene: self.current_scene().map(str::to_owned),
            ..self.args.handoff.clone()
        }
    }

    fn leave(&mut self, el: &ActiveEventLoop, done: Result<i32, deck_ui::handoff::Refusal>) {
        match done {
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
