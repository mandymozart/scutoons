//! Draft 1 of #481 painted by the real parts (`panel.rs`, `raster.rs`), to compare with the drawn draft: one raw RGBA
//! file per frame (`<name>-<w>x<h>.rgba`).
//!   cargo run -p deck-kit --example panel-draft -- <out dir>
use ab_glyph::FontRef;
use deck_kit::draw::DrawList;
use deck_kit::panel::{self, Part};
use deck_kit::raster;

const NATIVE: (f32, f32) = (640.0, 400.0);
const PHONE: (f32, f32) = (300.0, 560.0);

fn frame(out: &str, name: &str, screen: (f32, f32), grip: bool, parts: &[Part], notice: Option<(&str, &str)>) -> std::io::Result<()> {
    let t = panel::tokens();
    // the deck's mono face (deck-outlet's assets)
    let font = FontRef::try_from_slice(include_bytes!("../../outlet/assets/IBMPlexMono-Bold-latin.ttf")).expect("IBM Plex Mono");
    let mut d = DrawList::new(raster::per_px(&font));
    // the scene behind: a dark ground
    d.rect(0.0, 0.0, screen.0, screen.1, [0.09, 0.07, 0.16, 1.0]);
    match notice {
        Some((left, right)) => {
            panel::notice(&mut d, &t, screen, left, right);
        }
        None => {
            panel::draw(&mut d, &t, screen, parts, grip);
        }
    }
    let img = raster::raster(&d, &font, (0.0, 0.0), screen, 2.0);
    std::fs::write(format!("{out}/{name}-{}x{}.rgba", img.w, img.h), &img.rgba)
}

fn main() -> std::io::Result<()> {
    let out = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let kept = "Du spielst deine eigene Version (top · 2). Sie bleibt behalten.";
    let suggestion = [Part::Caption("Flash · Vorschlag", "von ana"), Part::Title("top · Version 3"), Part::Line(kept, true)];
    let keys = [("Enter", "annehmen"), ("Esc", "ablehnen"), ("< >", "wechseln")];
    let mut native = suggestion.to_vec();
    native.extend([Part::Buttons(&["Annehmen", "Ablehnen"], Some(0)), Part::Keys(&keys)]);
    frame(&out, "1-vorschlag-nativ", NATIVE, false, &native, None)?;
    let mut phone = suggestion.to_vec();
    phone.push(Part::Buttons(&["Annehmen", "Ablehnen"], None));
    frame(&out, "2-vorschlag-telefon", PHONE, true, &phone, None)?;
    let forced = [
        Part::Caption("Flash · erzwungen", "von ana"),
        Part::Title("top · Version 3 läuft"),
        Part::Line("Deine Version (top · 2) ist behalten.", true),
        Part::Buttons(&["OK", "Zurück zu meiner Version"], Some(0)),
        Part::Keys(&[("Enter", "OK"), ("< >", "wechseln")]),
    ];
    frame(&out, "3-erzwungen", NATIVE, false, &forced, None)?;
    frame(&out, "4-gast-zeile", PHONE, false, &[], Some(("Geflasht: top · Version 3", "von ana")))?;
    let keep = [
        Part::Caption("Flash · dieses Deck", "läuft: top · 2"),
        Part::Switch("Bitte nicht flashen", true),
        Part::Field("Probe läuft", "warum"),
        Part::Line("Der Conductor sieht den Grund. Ein erzwungener Flash geht trotzdem durch.", true),
        Part::Buttons(&["Speichern", "Abbrechen"], None),
    ];
    frame(&out, "5-nicht-flashen", NATIVE, false, &keep, None)?;
    let choice = [
        Part::Caption("Rolle", "Show: top"),
        Part::Mark("Publikum", "jetzt", true),
        Part::Mark("Musiker:in", "", false),
        Part::Mark("Licht", "nur live", false),
        Part::Buttons(&["Übernehmen"], None),
    ];
    frame(&out, "6-auswahl", PHONE, true, &choice, None)
}
