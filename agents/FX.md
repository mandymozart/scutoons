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
