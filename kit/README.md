# kit – the one source of the UI's look

The tokens and components of the deck's UI (deck-kit, `deck/kit`) and the web tools (`shared/tokens`, `shared/ui`) are
written here once; both apps are built from it. Palette D (RULES H11): no accent, no lime, no green switch; on, focus,
active and selected are the text colour, the one primary action is the pressed fill, colour says only ok, warning and
error. Every ground is see-through, never an opaque grey: a control's, a row's, a head's is the text colour at a low
alpha; a menu's, a popover's, a dialog's is the stage colour at 85 % (`surface`); a floating panel's is that colour at
`panel_opacity`. The generator refuses an opaque ground. What keeps its colour: signals,
outlet types, a group of spores, the mixer's strips, thumbnails.

## Files

- `tokens.toml` – every token of both looks: `[color]` and `[size]` for both, `[day.color]` / `[night.color]` for one
  (deck-kit's field names; colours `"#rrggbb"` or `"#rrggbbaa"`; sizes in rem, `base` and `edge` in px, `spacing` in em),
  `[day|night.signals]` (the nine signals), `[day|night.outlets]` (page · stage · target · device), `[groups] colors`
  (a group's six), `[day|night.colorblind]` (ok / warning / error of the colour-blind mode, replacing `go` `warn` `stop`).
- Component states (to come, one table per component and state, naming tokens): `[component.<name>.<state>]`,
  e.g. `[component.tab.hover] border = "text"`.

## How each app builds from it

- **Web tools:** `npm run editor-tokens -w @scutoons/tokens` writes `shared/tokens/editor.css` (`--editor-*`,
  `--signal-*`, `--outlet-*`, `--group-1…6`, `--editor-colorblind-*`); `shared/tokens/tool-colours.css` names them for
  the tools (`--tool-*`, `data-colorblind`). `npm run check:tools-css` and `npm run check -w @scutoons/tokens` fail when
  `editor.css` is stale or edited by hand.
- **Deck:** deck-kit reads this file as its base (its `tokens.toml` parser, `deck/kit/src/base/tokens.rs`). Until it
  does (TODO(#869)), `tokens.rs` and this file are two sources: change both.

## Components and their states

States as drawn: rest · hover · pressed/on · focus · disabled · dragged, plus a component's own.

- **Pill** – rest (the ground of a group or toolbar: line + panel ground)
- **PillTab** (item strip) – rest (muted) · hover (a rounded border in the text colour) · on (the text colour, no border) · focus · disabled · dragged (the border)
- **Workspace row** (PillTab, words) – rest · hover · on · focus · disabled · dragged
- **IconButton** – rest · hover · on · focus · disabled · alone (round) · in a group · with a dot
- **Button primary** – rest · hover (the fill dims) · on · focus · disabled (the pressed fill, faded)
- **Button plain** (framed) – rest · hover · on · focus · disabled · small
- **Button quiet** (words only) – rest · hover · on · focus · disabled · destroy
- **Segments** – rest · hover · on (the pressed fill) · focus · disabled
- **Chip** (the badge) – rest · hover · on · focus · disabled · with an icon
- **Status badges** – ok · warning · error (the Chip; the only ones with a hue) · colour-blind
- **Toggle** – off · hover · on (the pressed fill) · focus · disabled (off and on)
- **Field** – rest · hover · on · focus · disabled · error (the stop outline)
- **Search** – rest · hover · focus · disabled · selected text
- **Select** – rest · hover · open (the text-colour line, the chevron up) · focus · disabled
- **Select quiet** (a filter or sort) – rest (no box) · hover · open (a rounded border) · focus · disabled
- **MenuItem** – rest · hover · ticked (a text dot) · focus · disabled · head
- **ListRow** – rest · hover · selected (the pressed fill) · focus · disabled · dragged (with the insert mark) · selected + focus
- **Tree row** – rest · hover · selected · focus · disabled · dragged
- **Tile** – rest · hover · on · focus · disabled · dragged
- **Workspace card** – rest · hover · front (a text-colour frame) · focus · dragged · with a map
- **Thumb** – rest · hover · on · focus · disabled · dragged · empty
- **Panel head** (FloatingPanel) – tabs only: rest · hover · on · focus · dragged
- **Section fold** – rest · hover · open · focus · disabled
- **Drop · insert** – drop zone rest · drop zone on · the insert mark between rows
- **Link** – rest · hover · focus · disabled (no hue: the underline says link)
- **Status bar** – none · hover · with messages (ok / warning / error)
- **Signals** – event · control · clock · audio · picture · address · vote · scene · far (colour and shape)
- **Outlet types** – page · stage · target · device (lane and column heads only)
- **Spores view** – its ground of dots · the reveal under the pointer · plain
- **Group** – rest · hover · another colour · folded (as a pill)
- **Spore** (a card) – rest · hover · selected (the text colour) · focus (a flash, a ring closing in) · off · a free item
- **Spore · forms** – with pins · collapsed to its head · from far
- **Spore · face** – pending (with the hook chip) · a take shown
- **Spore · run** – waiting · running · done · failed
- **Plug** (a port's pin) – wired · hover · off · may (a cable in the hand may go in)
- **Pore** – free · wired
- **Cable** – rest · hover · lit · pulse · refused · in the hand
- **Take** – rest · hover · picked (the ring)
