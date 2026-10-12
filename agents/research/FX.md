Status 2026-10-12: phases 1–4 BUILT (how they work: the part from "later the same day" on); open: an item's own pickup sound (FEATURES 10.56). The title's "not started" is history.

# FX — reader feedback cues (plan, not started)

Owner 2026-10-07: plan only; **built after the rest of the backlog** (BACKLOG 10.56). The reader's own effects – no
external MIDI, devices or network (that is agents/LIVE.md, the performance mode, which builds on this). Drafts: the
"Feedback cues" board of the artifact "Scutoons Live drafts" https://claude.ai/artifact/GNwrpUc1VxpYT7bohnTEFD
(pick up · can't take it · nudge, as phone frames).

## Feedback cues (hints, pickups, reveals, impacts)

One vocabulary of **cues** instead of one-off effects. A cue = motion + sound + haptic, fired together.

| Moment | Cue |
|---|---|
| Hint ("tappable") | `hint.nudge`: after ~4 s unseen-tapped, one wiggle + sparkle + soft tick, once per page view; with "Show hints" also the contour and repeats |
| Pickup ok | `pickup.collect`: lift, pop, arc-flight to the inventory button (off-screen: nearest edge + a "+1" pocket chip), count bump, the item's sound, "+1 Gurke" |
| Pickup refused | `pickup.refuse`: shake + dull thunk, then today's notice; the item stays |
| Text box / item appears | entrances `pop` / `slam` / `fade` / `typeOn` next to `shiver` / `pulse` / `wobble`, plus an optional sound |
| Scene impact | new VN event `fx`: `shake` / `flash` / `zoomPunch` / `vignette` on slice, page or one box / item, optional sound |

**Rules (owner 2026-10-07)**
- Sound is part of a scutoon – there is no "sounds off". The account has controls: **sound effects volume** and
  **haptics on / off**. No separate reduced-motion setting.
- Motion: anything goes **within the visible viewport** (owner 2026-10-07) – an effect may overshoot its box and its
  slice (an item flying across slices, a flash over the whole screen), but never past the viewport: no horizontal
  scroll, no scrollbars, no growing page. Technique: transforms / opacity / filters only; whatever leaves its slice is
  drawn in one fixed, viewport-sized overlay layer (`position: fixed; inset: 0; overflow: hidden; pointer-events:
  none`) – the effect is lifted into it at its on-screen position and flies there. The overlay sits at the app root,
  outside `.slice`: `.slice` has `container-type: inline-size`, which makes it the containing block for
  `position: fixed` (MEMORY.md) – a fixed layer inside a slice would be clipped to that slice. Never a layout change, never the page
  scrolling or jumping (BACKLOG M.2).
- Placed items are cut-outs with **transparency**: every cue follows the alpha shape – `filter: drop-shadow`, never
  box-shadow / outline (the tap glow already does it, `shared/reader/src/items/slice-item.svelte`); the "Show hints"
  contour is a drop-shadow outline, not the dashed box; sparkles spawn on the shape (alpha mask, sampled once per icon,
  cached). Open: alpha hit-testing (ignore taps on transparent pixels, ~0.5rem tolerance) vs. the full box.
- Audio unlock: tap cues always sound; scroll-fired (intersect) cues stay silent until the first interaction.

**Structure**: `@scutoons/reader` `cues.ts` – registry `id → { keyframes (WAAPI), sound?, haptic? }`, one
`playCue(el, id, opts)` for viewer and creator preview. Sound: placement / event override → catalog `ItemDef.sound` →
the cue's default. Data: `ItemDef.sound?` (catalog column, migration), `SliceItem.sound?`, `showTextBox.params.sound?`,
the `fx` event (new type, no migration). Creator: one "Sound" slot per pickup and per showTextBox, the `fx` editor with ▶.

**Status**
- Phase 1 + 2 built (2026-10-07): `@scutoons/reader` `fx/` – `sounds.ts` (the effect sounds synthesized with WebAudio,
  no files; `configureFx`: the viewer routes them into Howler's master gain, so the music's mute applies; the volume +
  vibration switch), `shape.ts` (an icon's alpha read once with CORS: taps on transparent pixels miss with 0.5rem
  tolerance, sparkles start on the outline; unreadable = the box), `cues.ts` (`fxLayer` = the one fixed viewport layer
  on `body`; `liftCue` → `fly` / `refuse` / `drop`, `refuseCue`, `nudgeCue` + `sparkles`, `impactCue`). `SliceItem`
  gives `ontap(item, index, el, shape)` and `onidle` (4 s on screen at 60 %, untapped); the viewer's slice lifts at
  once, flies into the inventory button on a grant (off screen: the bottom right, fading), shakes on a refusal (the
  notice stays the "why"), nudges one pickup per slice and page view. "+1": the existing notice (open decision 3).
  "Show hints" draws the cut-out's contour. Account page "Soundeffekte": volume (0–100 %, "Anhören") + "Vibration",
  on this device (localStorage `scutoons_fx_volume` / `scutoons_fx_haptics`; members and guests). Check:
  `npm run check:fx` (headless; nudge, alpha miss, collect, refuse with a faked max_reached, account).
- Phase 3 (2026-10-07): text box entrances `pop` / `slam` / `fade` / `typeOn` (`TextBoxEffect`; reader
  `effects/effect-enter.svelte` – slam lands with a thud and shakes its slice through `impactCue`, typeOn wipes in one
  step per letter with key clicks) and `ShowTextBoxParams.sound` (`@scutoons/content` fx.ts `FX_SOUNDS`: none set = the
  effect's own – pop / thud / soft / key clicks for the entrances, nothing for the old ones –, `none` = silent; validated).
  Creator: four more "How it comes in" presets + a "Sound" select under them. No migration: new optional values only.
  Still open from phase 3: an item's own pickup sound in the catalog (`ItemDef.sound`) – the pickup plays "pling".
- Phase 4 (2026-10-07): the VN event `fx` – `{ type: "fx", params: { effect: shake | flash | punch | vignette, target?:
  slice | screen, duration? (ms, ≤ 3000), sound?, wait? } }` (`@scutoons/content` `FX_IMPACTS`, validated, schemas
  regenerated; no migration – a new type). Viewer: the script runner calls the VN context's `playFx` – `slice` = the slice
  across the screen's middle (a slice's own event is normally that one), `screen` = `.chapter-view`; `wait` waits for
  its length (it ends by itself – no dead end). Whatever moves sideways is clipped by its parent meanwhile (also a lifted
  / refused / nudged pickup), so the page never gets wider. Creator: `fx` in the event list (effect, on, sound, duration,
  wait, ▶ on the slice canvas). `npm run check:fx` 13/13 (+ the four impacts by rewriting the slice-24 tap in the API
  answer).
- Pickup sound (2026-10-07, owner: "an override for the sound pickup defaults on item level in catalog and on slice
  level"): catalog `ItemDef.sound` (admin → Catalog → Items, "Pickup sound"; none = pling, `none` = silent; validated
  in CatalogValidator – `FX_SOUNDS` kept equal to the TS list by CatalogTest; stored in the item's JSON, no migration)
  and `SliceItem.sound` (creator: the placed item's "Sound in the bag" with ▶, placeholder = the item's). The reader
  plays the placement's, else the item's, else pling. Fixed with it: the stored chapter format (`VNEventDoc`) lacked
  `fx`, so the creator couldn't save an fx event.
- Slice impacts without gaps (owner 2026-10-07): shake / punch on a slice move its content inside its own box (every
  child, zoomed just enough to cover the edges, the box clips) – the box stays, no white gaps to the neighbours; the
  whole column moving together is `target: "screen"`. A slice's vignette is a soft spotlight in the viewport layer
  (clear on the slice, darkening across its neighbours, following it while scrolling) – no hard edges.
- Local test content: `node scripts/dev/fx-test-content.mjs` writes a DRAFT of catpope chapter 1 (slices 1–14, one
  variation each with a de / en label: pickups with the three sound levels and an edge item, can't take it, nudge,
  item tap events, the four entrances on scroll, sound overrides, the old effects with sounds, impacts by tap / scroll,
  a 3 s vignette, wait-then-slam) on the local API as the test user; read it with the draft preview. Never published.
- typeOn reworked (owner 2026-10-07): the box opens like an old screen – a 2px square, a 2px line across, the full
  height (60 + 220 + 220 ms, clip-path) – then its text is typed letter by letter (the drawn text nodes emptied and
  filled back, 18–55 ms a letter so a long text stays under ~2.5 s) with a key click per letter, or the event's one
  sound; the event completes when the typing has (at least its duration).
- Show hints highlighters (owner 2026-10-08): tap / hover / sound areas are marked only by wandering lights
  (viewer `components/toon/fireflies.ts` + `hint-areas.css`; the dashed boxes, corners and icons went), more on bigger
  areas (one per --sc-firefly-area rem², at most --sc-firefly-max). Chosen per chapter in the creator (meta panel
  "Hinweise (Tippflächen)", `Chapter.highlight`, `@scutoons/content` fx.ts `HIGHLIGHTERS` – a list to extend):
  `fireflies` (default; round warm glows after nerdmanship's Miyazaki pen), and from the owner's attention.svg
  (`alien-shapes.ts`, loaded only when used): `alienflies` (its 35 filled brush strokes flying like the fireflies),
  `specks` (a random third – 46 – of its specks, about 1–3rem each) and `trails` (a streak of light running along one of
  its 21 thin arcs – a non-scaling 2px stroke –, fading, then the next one elsewhere; --sc-trail-density × the fireflies'
  count, default 2, at most --sc-trail-max 24). Each instance random (shape, size, turn); the first "alienshine" (all
  mixed) was dropped. Colours / blend from the owner's tuning: --sc-firefly-core #000, --sc-firefly-glow #ec60cf,
  --sc-hint-blend difference; --sc-hint-style on :root overrides the chapter's choice in the inspector. Everything
  re-reads live when the variables change.

**Phases**
1. `pickup.collect` + `pickup.refuse` with default sounds – viewer only, no data change.
2. `hint.nudge` + the "Show hints" contour; the viewport overlay layer.
3. Sounds per item (catalog `ItemDef.sound`, migration) and per reveal; the new entrances; the account's sound effects
   volume + haptics toggle.
4. The `fx` event (type, validation, creator editor with ▶ preview).

**Open owner decisions**
1. Alpha hit-testing for placed items, or the full box?
2. Default sounds (pling, thunk, tick, whoosh, thud): own sound design or a licensed library?
3. The "+1" chip: the shared toast model, or a lighter reader-only chip at the bag?
4. Nudge always on (once per visit), or only with "Show hints"?

## Reader effects per item (owner 2026-10-08)

"Different drugs hit differently … configure these effects per item": an item's `effect: { kind, duration_min }`
chooses from `@scutoons/content` substances.ts `READER_EFFECTS` (server `CatalogValidator::READER_EFFECTS`, CatalogTest
keeps both equal; unknown kinds refused), set in admin → Catalog → Items ("Leser-Effekt beim Benutzen" + minutes).
- `psychedelic` ("Farbrausch" – LSD, magic mushrooms): a moving sine-plasma cloud (computed at ~96 px, blurred into a
  cloud) fixed over everything, `mix-blend-mode` --sc-farbrausch-blend (overlay) at --sc-farbrausch-opacity (0.55),
  --sc-farbrausch-speed; no tilting frames any more (owner). Sound: a high-pass sweeping ≈ 160–600 Hz into a 380 ms echo.
- `haze` ("Dunst" – weed, set locally; production: the owner in admin): a light veil with a slight blur and big soft
  haze banks drifting (--sc-haze-color / -opacity / -blur / -blend). Sound: muffled (low-pass ≈ 700–1300 Hz) with a
  soft 90 ms echo.
- testo: no visual effect yet. Next idea (owner): wearables with an effect while worn – sunglasses = everything orange.
Audio: `viewer/src/lib/audio/audio-effects.ts` puts the effect's chain between a master gain and the speakers on both
outputs – the music's own AudioContext (AdaptiveMusicPlayer `musicOutput`) and Howler's (samples + effect sounds;
re-attached when Howler replaces its context) – with 1.2 s crossfades in and out. DEV hook `__scutoonsAudioFx()` shows
the graph.
- Later the same day (owner): a running effect can't be ended – the bar only says how long it lasts (no "Effekt
  beenden"); it lives in the root layout, so it goes wherever the reader goes (account page too). Eating sobers up:
  catalog `sober` (0 < x < 1; admin "Ernüchtert: kürzt laufende Effekte um (%)") – each piece takes that share off every
  running effect's remaining time (server ReaderState::use + client applySubstance for guests; a candy item at 50 %
  halves it, cucumbers e.g. 15 %). The plasma: blend difference, colours only purple → blue → green → yellow. LSD's echo:
  feedback 0.65, its time drifting with two slow LFOs (≈ 210–550 ms); Dunst's echo: feedback 0.6. Fixed: the sound
  didn't come back after leaving a page while an effect ran (taking a chain out disconnected the output 1.2 s later,
  the new chain included).
- LSD sound, final for now (owner): feedback 0.78, the echo time drifting ≈ 120–640 ms (two LFOs), and a stereo panner
  swinging left / right at a pace that speeds up and slows down (an LFO on the swing LFO's rate, ≈ 0.05–0.45 Hz).
  Slam's shake: `showTextBox.shake` slice | screen | none. Repeated reader notices count up (× 2) instead of stacking.
- LSD and scrolling (owner 2026-10-08): the faster the reader scrolls, the faster the echo repeats – its time and drift
  divided by a pace 1 … 4 (≈ 2500 px/s and more = 4), smoothed, gliding ≈ 0.8 s up and ≈ 0.6 s back when the scrolling
  stops (`followScroll` in audio-effects.ts, only while psychedelic runs). Screen shake / punch: zoomed to cover the
  sides, around the middle of the screen.
- Wearables that tint (owner 2026-10-08: "sunglasses will make everything orange hued"): catalog `worn_tint` (`#rrggbb`,
  admin "Tönung beim Tragen"; validated TS + PHP): while an item with it is worn, the reader lays that colour over
  everything (reader-effect.svelte, multiply, --sc-tint-blend / --sc-tint-opacity 0.6), under any running effect.
  Items: `punschkrapfen` (food, sober 0.5 – halves a running trip, appetite −10) and `sunglasses` (face, worn_tint
  #ff9a2e) – created in the local catalog; production: the owner creates them in admin (icons to upload).
- Variants with their own tint and wear (owner 2026-10-08: "use variants and have colour as variant"): `ItemVariant.worn_tint`
  and `.wear` (none = the item's; validated TS + CatalogValidator; admin: a tint field per variant row, wear in the
  advanced JSON). A story can grant a variant: `SliceItem.variant` (pickup; the slice and the creator canvas show the
  variant's picture), `giveItem` / a choice's `give` `variant`; `POST /inventory/grant/ { variant }` (422
  unknown_variant), guests keep it locally, the guest import carries it (one piece per variant). Reader: the tint and
  the avatar overlay come from the worn entry's variant first. Creator: a "Variante" select on placed items and gives.
  The sunglasses: ONE item `sunglasses` with six colour variants (orange, green, purple, red, black, blue).
