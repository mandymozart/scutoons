Status 2026-10-30: concept (#186, owner: option 1 – DECISIONS 2026-10-30). Built: the protocol (media.preload / media.ready / typed cells) and the deck's native outlet (deck/stage/README.md "Live outlet"). Not built: the stage builder (§6 is a design draft), the set data model (#193), the signed media route (§7).

# LIVE-LENSES — storyboard, slice editor and Live as one

Owner 2026-10-29: show media are "assigned to outlets in the Live conductor … a concept still to make (storyboard,
slice editor and Live views together)". Owner 2026-10-30: the reference example is Cat Pope s1c2, the sermon in the
cathedral (#186 comment, #188, #190). Builds on LIVE.md B.5 (show, outlets, cells, layers), B.6 (protocol), #193 (sets).

## 1. One flow, three lenses
A toon has **one flow**: the rows of the creator's FlowIndex (scene keys, story order, branches). Three editors look at
the same rows – they never copy them – and share **one Timeline** (research/TIMELINE.md) underneath.

| Lens | Answers | Edits | A row is … |
|---|---|---|---|
| **Storyboard** | what is drawn | scenes → shots (frames) → generated art → slices | its shots (`sb-25-4`) |
| **Slice editor** | what a reader sees | slices, text boxes, events, music | its slices (`Slice.shot` links back) |
| **Stage builder** (Live) | what every outlet / device does when the flow is here | cells, layers, cues, looks, device messages | one cell per outlet |

- **The selection is the row.** Switching lens keeps the selected row (and, inside it, the nearest slice ↔ shot ↔ cue:
  `Slice.shot` and the cue's `at.slice` say which). The lens is a tab in the creator header, not a new app.
- **The Timeline is the row's time:** shots (storyboard), slice enters (slice editor), cues and cell changes (stage
  builder) sit on one axis; scrubbing it in any lens moves the same playhead.
- **Each lens only adds:** the storyboard never touches the show, the stage builder never touches slices (B.5.1 "the
  show only adds"); a pick follows the toon's edits.

## 2. Pickable layer sources
A stage-builder layer is a **pick**, never a copy (B.5.2 `ToonSrc` / `LiveSrc`). One picker drawer (`Sheet`, B.5.5)
with four shelves – one per lens plus the sets:

| Shelf | Picks | On the wire (deck/protocol `LayerSrc`) |
|---|---|---|
| Storyboard | a shot's frame (its current art: frame → still → video) | `image` / `video` of a preloaded medium (the conductor resolves the frame's file) |
| Slices | the chapter running, a slice, a panel (crop), the surtitles | `scroll`, `slice` (reading-order index), `panel` (+ crop), `surtitles` |
| Sets (#193) | a set's spot in a mode: still / video / 3D | `set { set, spot, mode }` |
| Live | show media (videos, stems, PDFs), text, colour, a camera | `video` / `image` / `model3d` (preload id), `text`, `colour`, `camera` |

The show document stores the pick (slice id, shot id, set + spot); the **conductor resolves** it into the wire `Cell`
(slice index, preload id) when it sends – so an output needs no show document and no creator code. `mirror` is resolved
there too (an output sees only its own outlet).

## 3. A deck as a screen outlet
- The show names outlets logically (`left`, `nave`); the **venue patch** (B.5.4) maps an outlet to a machine. A paired
  deck (#180) appears in the patch as a screen target with its **class badge** (A x86 · B ARM · C ARM small,
  CYBERDECKS §1), its name and its state (online, ready n/m). Class C has no video: the builder greys video / 3D layers
  for an outlet patched to a C deck, with the reason.
- The deck joins the relay as `output` of its outlet (group = outlet), gets `media.preload` at doors, fetches with its
  deck token and answers `media.ready { ready, missing, why }` – the conductor's **ready light** per outlet (green = all,
  amber = some missing with the reason, red = offline). Then `cell` on every row change; a reconnecting deck gets the
  snapshot (the cell and the last preload) and shows the right cell at once.
- The deck renders natively (deck-stage's `outlet` node): flat layers into one plane, a 3D set as the space node behind
  it, crossfades by the layers' `enter` / `exit`. Not yet: video (S4, a set spot's still stands in), video-in, audio
  layers (the deck's audio graph – a later step).

## 4. Sets grow: frame → still → video → 3D (#193)
A **set** is one staging of a place (the cathedral); a **spot** a named point in it (`spot_aisle`, `spot_altar`). Every
spot carries variants that arrive over time, and every use names set + spot, never a file:

```
storyboard frame  →  still (the rendered picture)  →  video (a rendered loop)  →  3D (the set's .glb, the camera at the spot)
   (the sketch)        reader: loading / static        reader: viewport video      Live: a deck's space node
```

- The stage builder's layer has a **mode** chip `still · video · 3D`; a mode the spot lacks yet is shown dimmed with
  what it falls back to (3D → video → still → the storyboard frame). The conductor sends the asked mode; the deck plays
  what its class and modules can (a C deck: still).
- The deck resolves set + spot + mode through one small trait (`live::media::Resolve`): today `LocalSets` – the rooms
  folder for 3D, `<cache>/sets/<set>/<spot>.<mode>.<ext>`, else the preload item's `url`; once #193's model exists, an
  API resolver (`GET /api/sets/<id>` with the deck token, cached on DECKDATA) – no change to cells or the protocol.

## 5. Worked example: the sermon (Cat Pope s1c2)
Outlets: `left` (a deck, class A, the projector over the nave), `phones` (audience group), `lights` (DMX). The rows are
s1c2's scene keys. Runnable for the `left` outlet: `deck/stage/examples/sermon.json` with `live-cue` (README "Live
outlet"); the slice numbers are reading-order indices of today's chapter.

| # | Row (flow) | Storyboard lens | Slice editor lens | Stage builder lens: `left` cell |
|---|---|---|---|---|
| 1 | the walk to the cathedral | shots of the street | slices 58–60 | `slice` 59, contain (the comic, as the reader sees it) |
| 2 | "Die Predigt stand bevor" | PLF on the pulpit | slice 61 | `slice` 61 – the last comic beat before the live part |
| 3 | the sermon: ROAR | the cathedral wide shot | slices 62–66 (ROAR, KLING KLONG) | `set cathedral spot_aisle 3D` (enter 2.5 s) + `text` "ROARRRR" – the conductor being with PLF's head in the nave (#188 ph. 4–5), the decks' beings around it; DMX look "crescendo"; the crescendo drives tempo and energy (#189 sends) |
| 4 | the sermon's lines | – | slice 67 (the sermon text) | `set` (same spot – nothing reloads) + `text` "Neuer Tag, neues Spiel – mein Pfad, meine Mission." + `panel` of slice 67 |
| 5 | the choir: "Cloud" | the choir shot | slice 69 (the song) | `set` + `set cathedral choir video` (S4 – its still until then) + `colour` wash + `text` "Ich wär so gern eine Cloud, laut, laut, laut."; `phones`: colour pulses on the beat |
| 6 | the cat interrupts | the cat | slices 71–72 (MIAU) | `slice` 72, enter 1.5 s – the cells switch back to the comic |

Step by step, as the system runs it:
1. **Doors.** The conductor sends `media.preload { outlet: left, items: [nave = cathedral/spot_aisle/3D, choir =
   cathedral/choir/video] }`; the deck answers `media.ready { ready: [nave], missing: [choir], why: { choir: "… not on
   this deck – no set data yet (#193)" } }` → amber light, the reason in the conductor and the deck's HUD.
2. **Rows 1–2.** The reader's flow reaches the rows; the conductor sends `cell` per row; the deck draws the slices from
   its library cache (public, no token).
3. **Row 3.** The cell's `set … 3D` layer → the space node loads the cathedral (already loaded: only the spot moves),
   the plane fades over 2.5 s, the text layer on top. Conductor sends (#189) are not cells – they ride beside them.
4. **Rows 4–5.** Same set and spot → only the flat layers change (a 0.6 s crossfade); the choir's video layer shows its
   still (or nothing) and the HUD says why.
5. **Row 6.** No set in the cell → the space fades out behind the comic's slice.
6. **A deck reboots mid-show.** It joins again, the snapshot carries row 5's cell and the doors' preload: the right
   picture at once, the cache answers the preload without the network.

## 6. Design draft: the stage builder's row view with a deck outlet (not built)
The creator shell (B.5.5): sidebar = the rows (flow order, the lens tabs above), main = the selected row's outlets at
their real proportions, right dock = the inspector, the Timeline below. A deck outlet is a screen canvas with the
deck's badge and its ready light; its layers stack beside it.

```
┌ creator ─ [Storyboard] [Slices] [Stage] ───────────── s1c2 · show "Tour cut" · venue "Dom" ───────────┐
│ ROWS             │ ROW 3 · the sermon: ROAR          ▶ row   ⟳ rehearse │ LAYER  room                    │
│  1 walk          │ ┌ left ── deck K7 "nave" [A] ● 1/2 ┐  ┌ phones ─┐   │ source  set · cathedral        │
│  2 Predigt…      │ │                                   │  │ ▒▒▒▒▒▒▒ │   │ spot    [spot_aisle ▾]         │
│▸ 3 sermon: ROAR  │ │      (cathedral, 3D, spot_aisle)  │  │ colour  │   │ mode    [still|video|3D]       │
│  4 lines         │ │            ROARRRRRR              │  │ pulse   │   │ box     full · fit cover       │
│  5 choir: Cloud  │ │                                   │  └─────────┘   │ enter   2.5 s   exit 0.6 s     │
│  6 the cat       │ └───────────────────────────────────┘  lights: look  │ ─────────────────────────────  │
│                  │  layers (back → front)                 "crescendo"   │ deck K7 "nave" · class A       │
│  + in-between    │   ◧ room   set cathedral · 3D   live                │ ready 1/2 · choir: no set data │
│                  │   T roar   text "ROARRRRRR"     live                │ (#193) · S4 video not built    │
│                  │   + From the toon…  + Live media…  + Set…            │                                │
├──────────────────┴──────────────────────────────────────────────────────┴────────────────────────────────┤
│ TIMELINE  ── shots ▮▮ ▮ ── slices 62 63 64 65 66 ── cues ◆crescendo ◆roar ── cells: left │ phones │ lights │
└──────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

- The outlet header carries the patched deck: name, class badge `[A]`, the ready light `● 1/2` (green / amber / red);
  an unpatched outlet shows "not patched" and a simulated canvas (rehearsal, B.8).
- Layers show `toon` / `live` / `set` tags (B.5.5); a layer the patched deck can't play (class C video, S4) carries the
  same reason the deck's HUD shows.
- The matrix mode (rows × outlets) stays B.5.5's; a deck column shows the badge in its header.
- Kit only (B.5.5): AppShell, Tabs, Divider, Sheet, Field, Badge, Sortable; rem sizes. Show it on the real page (phone +
  desktop) before building (RULES "Working with the owner").

## 7. Left (issues)
- **Signed media route:** show media under `live/<toon>/` need the deck token on the CDN side – a server route that
  answers a deck token with a short signed URL (bunny token auth) or proxies. Until then `url`s must be public or local.
- **The set data model** (#193): sets, spots, variants per mode; the API resolver for decks.
- **The stage builder UI** (§6) and the conductor resolving picks into wire cells (the conductor app, LIVE §1.2).
- Deck side: video (S4), video-in, audio layers on the deck's audio graph, mirror, a show clock for cells' `at`.
