//! #374: the HUD's adapter half – deck-native's platform objects (the renderer, the governor, the audio backend and its
//! glitch counters, the outlet, the sync, the space, the outputs, the tempo's I/O) read into deck-ui's `Status`. A child
//! of main.rs (the bin: it reads the App's fields), not of the lib. No words and no decisions here: deck-ui says every
//! line and item (agents/research/DECK-UI-MODEL.md); a line here that builds HUD text is the defect #374 removed.
use super::App;
use deck_ui::{Audio, Gpu, Quality, Source, Status};

impl App {
    /// what the HUD reads now
    pub(super) fn status(&self) -> Status {
        let show = &self.show;
        let r = self.renderer.as_ref();
        let mut live = Vec::new();
        #[cfg(feature = "live")]
        if let Some(l) = &show.live {
            live = l.hud_lines();
        }
        #[cfg(feature = "events")]
        let events = show.events.as_ref().map(|e| [e.counters.routed.load(std::sync::atomic::Ordering::Relaxed), e.counters.dropped()]);
        #[cfg(not(feature = "events"))]
        let events = None;
        #[cfg(feature = "outputs")]
        let outputs = show.outputs.as_ref().map(|o| o.summary.clone());
        #[cfg(not(feature = "outputs"))]
        let outputs = None;
        #[cfg(feature = "sync")]
        let (sync, push) = (show.sync.as_ref().map(|y| y.line()), show.sync.as_ref().and_then(|y| y.push_line()));
        #[cfg(not(feature = "sync"))]
        let (sync, push) = (None, None);
        #[cfg(feature = "space")]
        let space = show.space.as_ref().map(|s| deck_ui::Space {
            mood: s.being.mood_line(),
            line: s.hud_line(&show.graph),
            session: s.session_lines(),
            joined: s.join != deck_native::space::Join::Alone,
        });
        #[cfg(not(feature = "space"))]
        let space = None;
        Status {
            error: self.error.clone(),
            live,
            live_note: show.live_note.clone(),
            scene: show.scene_name.clone(),
            gpu: r.map(|r| Gpu { name: r.adapter.name.clone(), backend: format!("{:?}", r.adapter.backend) }),
            quality: self.governor.as_ref().map(|g| Quality {
                level: g.level().name().into(),
                ceiling: g.ceiling().name().into(),
                auto: g.auto(),
                scale: g.scale(),
                render: r.map_or(1.0, |r| r.render_scale()),
            }),
            audio: match (&show.audio, &show.glitches) {
                (Some(a), Some(g)) => Some(Audio { name: a.name.into(), rate: a.rate, buffer: g.device_frames(), lead: self.lead(), clock_rate: show.clock.as_ref().map_or(48_000.0, |c| c.rate) }),
                _ => None,
            },
            report: self.stats.feed.report.clone(),
            events,
            tempo: show.tempo.tempo_line(),
            source: Source { name: show.tempo.source.name().into(), status: show.tempo.status(), midi_out: show.tempo.clock_out().is_some() },
            outputs,
            outputs_note: show.outputs_note.clone(),
            sync,
            push,
            space,
            fft: show.controls.fft(),
            music: show.music.iter().map(|s| s.lock().unwrap().clone()).collect(),
            slices: r.map(|r| r.slides_status()).unwrap_or_default(),
        }
    }
}
