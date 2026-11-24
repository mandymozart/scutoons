# kit – the one source of the UI's look

## Always

**Grounds are translucent and never grey.**

- Every ground – a control's, a panel's, a menu's, a popover's, a dialog's, a box's, a card's – is see-through, the stage
  showing through; an opaque grey fill does not exist; a grey ground in a draft or a capture is not the reference
  (owner 2026-11-24). The generator refuses an opaque ground.
- The stage is black by night, white by day.
- A panel is the stage colour (`surface`) at one strength (`panel_opacity`); a menu, a popover and a dialog's box lie
  over panels and take a stronger one (`popover_opacity`), with the ink ring – still see-through.
- A control is the text colour at 6 %, hover 10 %, selected 16 %.
- No signal hue: on, focus, selected are the text colour; colour only says ok, warning, error; what keeps colour is in
  RULES H11.

## What it is

The tokens and components of the deck's UI (deck-kit, `deck/kit`) and the web tools (`shared/tokens`, `shared/ui`) are
written here once; both apps are built from it. Palette D (RULES H11): no accent, no lime, no green switch; the one
primary action is the pressed fill. What keeps its colour: signals, outlet types, a group of spores, the mixer's strips,
thumbnails.

## Files

- `tokens.toml` – every token of both looks: `[color]` and `[size]` for both, `[day.color]` / `[night.color]` for one
  (deck-kit's field names; colours `"#rrggbb"` or `"#rrggbbaa"`; sizes in rem, `base` and `edge` in px, `spacing` in em),
  `[day|night.signals]` (the nine signals), `[day|night.outlets]` (page · stage · target · device), `[groups] colors`
  (a group's six), `[day|night.colorblind]` (ok / warning / error of the colour-blind mode, replacing `go` `warn` `stop`).
- Component states: one table per component and state, `[component.<name>.<state>]`, its parts (`text` `ground`
  `border` `ring` `knob` `mark`) naming a token – `"text"`, or `"text 12%"` for the token see-through – `opacity`
  as a number, and `blur` in rem (the overlay's alone: the generator refuses it elsewhere), e.g. `[component.tab.hover] text = "text"` · `border = "line"`. A ground never names `line`, `muted`,
  `text` or `bright` at full strength (the generator refuses an opaque grey). Written so far: `focus` · `tab` · `pill` ·
  `icon_button` · `button_primary` · `button_plain` · `button_quiet` · `segments` · `chip` (with `ok` `warning` `error`)
  · `toggle` · `slider` · `field` · `select_quiet` · `menu` · `menu_item` (with `head`) · `list_row` · `tile` · `thumb` ·
  `workspace_card` · `panel` · `overlay` · `section` · `drop` · `link` · `status`. Every state written is read by an app –
  the web's `kit.css` or the deck's `Tokens::kit` (`section.disabled` is the deck's alone; `slider`, `thumb` and
  `menu_item.head` the web's); one neither reads leaves the file.

## How each app builds from it

- **Web tools:** `npm run editor-tokens -w @scutoons/tokens` writes `shared/tokens/editor.css` (`--editor-*`,
  `--signal-*`, `--outlet-*`, `--group-1…6`, `--editor-colorblind-*`) and `shared/tokens/kit.css` (the states,
  `--kit-<name>-<state>-<part>`, e.g. `--kit-tab-hover-border`, each naming a `--tool-*` colour);
  `shared/tokens/tool-colours.css` names the colours for the tools (`--tool-*`, `data-colorblind`). The components of
  `shared/ui/src/editor` draw their states from `kit.css`. `npm run check:tools-css` and `npm run check -w
  @scutoons/tokens` fail when either file is stale or edited by hand.
- **Deck:** deck-kit builds this file in as its base (`deck/kit/build.rs`, its parser `deck/kit/src/base/tokens.rs`,
  the states through `Tokens::kit(name, state)`); a test fails on a missing key and on a key the deck does not read. A
  `tokens.toml` beside the editor overrides it live. A build with only `deck/` in its context hands this folder in as
  `DECK_KIT` (Cross.toml, the Dockerfiles). The deck's kit page: `deck-editor --kit`.

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
- **Overlay** (RULES H15; `shared/ui` editor `Overlay`) – rest (everything under it, the bar too, blurred and dimmed: the
  only blur) · reduced (reduce motion, the system's or the app's: no blur, a see-through dim) · box (what it carries – a
  panel taken full close to the window's edges, Search, a dialog). The switcher, Search, a panel taken full and every
  dialog lie on it; Esc and a click outside close it. `check:tools-css` fails on a blur anywhere else.
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
