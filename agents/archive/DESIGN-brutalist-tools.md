# DESIGN – "Brutalist tools" (moved from agents/DESIGN.md on 2026-11-24, #901; frozen)

The tools' look from 2026-10-01 until the kit replaced it (RULES H9–H19, kit/README.md, epic #626). How it is now:
DESIGN.md "The tools' look: the kit".

## Brutalist tools (owner rule, 2026-10-01)

> **Superseded 2026-11-19** (RULES H9, law.md [editor-style], epic #626): the tools move to the editor style of deck-kit's
> editor module and the OSCT editor; the creator becomes desktop-only. This section stays the reference for what is built
> until #626's tokens and kit land, then it is rewritten and moved to `agents/archive/`.

> "The UI should use the font sizes I used in the reader even in all the admin tools (creator, admin, embassy, shop).
> I don't like the tiny stuff; the font doesn't look good too small." · "I want clear brutalism, not something in
> between. It makes it more playful, even if we lose some space."

Applies to the creator, the admin and the embassy (opt-in class `sc-tool` on the app root → `@scutoons/tokens/tools.css`,
inert without it). The shop already uses the reader's rem scale and keeps the reader's look.

**Type = the reader's.**
- The root is the reader's fluid root from base.css: **14px < 480px, 16px, 24px ≥ 768px**; the one exception is tools.css's
  70 % for the tools from 768px (16.8px, owner 2026-10-11). No app sets its own px base (the admin's 16px/14px and the
  creator's 13px are gone).
- Only the type styles: `body` / `button` / `label` 1rem (running text, table cells, inputs, sidebar entries, crumbs,
  buttons); `caption` 0.75rem = **the smallest text** (field labels, table headers, chips, hints, meta);
  `--type-title-size` 1.5rem = the one page title (h1, bold) and big numbers (regular). `nav-micro` is not used in tools.
- **No px font sizes anywhere** (also not 0.6em-style shrinking); sizes come from `--type-*` tokens.
- **Bold only very rarely** (owner rule 2026-10-02: "only use bold very rarely"; it supersedes the 2026-10-01
  "bold sparingly" list, which was not followed):
  - **Default weight is regular** (`var(--font-weight-regular)`) everywhere: body, list / row titles, table cells and
    headers, buttons, tabs, chips / badges, breadcrumbs (incl. the current crumb), field labels, panel / section
    headings, dialog / sheet titles, counts, big numbers, codes, hints.
  - **Bold is allowed only for** the page's single main title (h1) where the page has one, and a rare, deliberate
    emphasis of a whole short phrase where meaning needs it (e.g. a destructive confirmation, a stale-data warning):
    `<strong class="sc-emphasis">`. If unsure, regular.
  - **Hierarchy comes from** size (the type scale tokens), uppercase + letter-spacing (`--tracking-label` 0.08em;
    bold caps, if ever, get `--tracking-label-bold` 0.12em), spacing and ink / muted colour – not weight. Selected /
    current items are shown by inversion, not by bold.
  - Markup: no `<b>`; `<strong>` only as the emphasis above (plain text or a `<span>` otherwise). tools.css sets the
    browser-bold defaults (h2–h6, th, legend, summary, optgroup) to regular; only `h1` is bold.
  - Enforced by `npm run check:tools-css` (bold `font-weight`, `<b>`, `<strong>` without `sc-emphasis`; a line with
    `bold-ok` + the reason is exempt).
- Headings: h1 = title (bold); h2/h3 = 1rem regular uppercase, 0.08em (`.sc-heading`). Field labels, table headers,
  sidebar group heads, legends, summaries = the **tool label**: caption, regular, uppercase, 0.08em, ink
  (`.sc-label`, `.sc-nav-head`).
  Hints / meta = caption in `text-muted` (`.sc-caption`). Badges with data (codes, ids) stay in their case.

**Frames, colour, shape.**
- Structure = **2px ink** (`--border-width-regular` + `--ink`): fields, panels, sections, tables, top bar,
  ActionBar, sidebar edge, tab groups, segmented controls (buttons: the viewer's, see Controls). Table rows are split by 1px ink (`hairline`), never grey.
- Hard ink / paper. `border-muted` only for **inactive** things (the secondary button at rest, the quiet panel, an
  inactive tab).
  No `surface-*` greys and no tinted fills for structure or state: a state is the frame's colour + a thick left rule
  (0.5rem / 0.5em) in `state-error` / `state-warning` / cyan, the text stays ink. Unsaved = an inverted bar.
- Square everything (`radius-none`; `radius-sm` only for avatars). **No shadows** (also no inset box-shadow tricks) –
  two exceptions: the live selection's 2px inset frame (rows would overlap as outlines) and a dragged row's lift.
- **Round vs square** (owner 2026-10-07, every app – creator, admin, embassy, viewer, shop): a **standalone icon-only
  button is round** – a `--control-height` circle, 2px ink ring (`class="sc-round"`, tools.css; `IconButton`,
  `MaximizeButton`; the viewer's top-right toggles; the palette trigger in its icon-only state).
  **Grouped controls are square**: segmented groups (`sc-seg` / `Segmented`), toolbars (the Flow's zoom / fit is one
  `sc-seg`, the ActionBar, canvas tools), tab rows, and every **language switcher** (the creator's text-language
  switch, the viewer menu's languages – one joined frame, the chosen one inverted – the shop's). A control that shows
  text (label, code, "Ctrl K") is square too. **Fold / unfold buttons are quiet** (owner 2026-10-20: they "take a lot of
  visual focus"): `FoldButton`, `Rail` and the creator's menu / list folds are borderless muted squares (`sc-round
  sc-quiet`), ink + the nav outline on hover. Open: is the frameless close cross (`CloseButton`, "Close cross" below) round too? – FEATURES 10.70.
- Cyan (`primary-cyan`) only for **live** states: the "published" badge, the live selection (see "Selection" below),
  the canvas selection. Never for the current nav entry or a selected row outside the live region.
- **Badges are FILLED** (owner 2026-10-01): status chips, tags, counts = a solid fill, **white text where it reaches
  4.5:1, else ink**; caption size, regular weight (status words in uppercase with `--tracking-label`; ids / codes keep
  their case). The colours (the owner's refined palette) live in one place, `shared/tokens/tokens.css`
  `--badge-<name>` + `--badge-<name>-text`:

  | Badge | Fill | Text | Contrast (white / ink) | Used for |
  |---|---|---|---|---|
  | `published` | `#00d1ff` | ink | 1.82 / **11.56**:1 | published, live, signed |
  | `draft` | `#6b6b6b` | white | **5.33** / 3.94:1 | saved draft, unclaimed |
  | `soon` | `#b300ff` | white | **4.82** / 4.36:1 | coming soon, announced |
  | `error` | `#ff1900` | ink | 3.90 / **5.39**:1 | error, revoked, missing |
  | `warning` | `#ff6600` | ink | 2.94 / **7.15**:1 | warning, unsaved, expired, too long |
  | `info` | `#0066cc` | white | **5.57** / 3.77:1 | info |
  | `neutral` | `#000000` | white | **21** / 1:1 | claimed, source language, tags, plain counts |

  `@scutoons/ui` `Badge tone` = these names (aliases: `accent` = published, `ink` = neutral); tools.css `.sc-chip`
  (neutral) + `.is-<name>`.

**Controls.**
- **Sizes in rem** (owner 2026-10-01, no px): fields, nav entries and clickable rows are `--control-height` =
  2rem + the 2px frame (32 · 36 · 52px at the reader's root; the tools on desktop: 37.6px), as tall as a compact button.
- **Touch minimum only on touch screens** (owner 2026-10-01): one rule in tokens.css, `@media (pointer: coarse)`
  raises `--control-height` to max(44px, 2rem + frame); fields, list rows, nav entries and buttons use it as
  `min-height`, so phones and tablets get ≥ 44px targets while a mouse desktop stays rem-only.
- Inputs / selects / textareas: paper, 1rem; **text inputs and textareas a thin grey frame** (1px `border-muted`, ink
  on focus – owner 2026-10-03: tells fields apart from selects and buttons), selects look like secondary buttons (2px `border-muted`, ink on hover / focus, a Feather chevron); padding `0 space-05` (one control height, see below;
  textarea `space-05`), placeholder `text-muted`; disabled = `border-muted` + `text-muted`; invalid = `state-error` frame.
- **Buttons = the viewer's** (owner 2026-10-01, no new invention): primary = ink bg / paper text; secondary (the
  normal one) = paper + 2px `border-muted`, **on hover the outline turns ink**, no inversion on hover; ghost = no frame,
  `border-muted` frame on hover; disabled = opacity 0.5. Height = `--control-height` (see "One control height"), side
  padding as in the viewer `space-3`; compact `space-15` where the bars need it (`Button size="sm"`, the creator's
  plain buttons) – `sm` is narrower, never lower. Square, 1rem.
  On phones (< 480px) the ActionBar narrows its buttons to `space-1` at the sides and shows Info as its icon only.
- **Focus = a 3px ink outline** (`--border-width-focus`) 2px outside the frame (`--focus-offset`), everywhere.
- **Selection** (owner 2026-10-07, BACKLOG 10.40, UI-CONCEPTS E.8) – one look per meaning, app-wide (creator, admin,
  embassy); the only selection CSS is tools.css:

  | Meaning | Look |
  |---|---|
  | **Selected** – the edited object (what the inspector / editor / detail shows) and every picked one (Shift / Ctrl) | inverted: ink bg, paper text – `class="sc-selected"` |
  | **Live selection** – selected AND in the live region (what Delete / arrows / Esc / copy act on) | `--select-live-bg` (= `--surface-tint-cyan`), a 2px **inset** frame in `--select-live-border` (= the primary colour; `box-shadow: inset`, neighbouring rows' outlines would overlap), text `--select-live-text` #005a70 (≈ 7.5:1), not bold – automatic: `[data-live] .sc-selected` |
  | Keyboard highlight in a listbox (palette, pickers, comboboxes, the expression builder) | `sc-selected` in a region that is live while it is open / focused → the live look |
  | Picker tile being chosen (`.sc-card.is-selected`) | the live look (the open sheet is the live region) |
  | Current value of a picker field (`.sc-card.is-current`) | cyan left bar |
  | Nav "you are here" (`.sc-nav-item.is-current` / `aria-current`, the Flow's node, the script outline) | inverted; the live look only when it is also the live selection (the chapter tree's group: both classes) |
  | Toggle on / tab / segment / pressed (`aria-pressed`, `.active` buttons), the pager's current page | inverted (small controls); pager links and menu triggers otherwise look like secondary buttons |
  | Hover | **never inverted**: a 2px ink frame inside the row / a thicker ring on `.sc-round` |
  | Focus (keyboard) | the 3px ink ring everywhere – it wins over a component's `all: unset` (tools.css); a control that draws its own focus is a `role="separator"` or carries `data-focus-own` |
  | Drag / drop | the primary drop line, the moving row lifted (DESIGN "Reorderable lists") |
  | In progress (admin batches' running row) | not a selection: `--surface-tint-cyan`, not inverted |

  **Regions** (`@scutoons/ui` `selection.svelte.ts`): `use:region={'slices'}` (or `{ name, initial, modal }`) on each
  selectable area; `pointerdown` / `focusin` inside it makes it live (`data-live`; the innermost region wins, so the slice
  list inside the chapter tree is its own region); `liveRegion.current` / `liveRegion.is(name)` tell the page which one.
  `initial` = live on page load when nothing else is (the main editor area), `modal` = live while mounted (sheets, the
  palette), closing returns to the region before. Keys (Delete / Backspace / Ctrl+X, arrows, Esc, later copy / paste)
  act on the live region only. One region shows at a time on a phone: it is live, nothing extra shows.
  **Canvas objects** (slice canvas, cover, map) can't be inverted – the art is under them: selected = the kind's frame;
  live = the frame + handles + its tag. **Multi-pick**: every picked row has the selected look (all cyan when live).
- Checkboxes / radios 1.25rem, `accent-color: ink`. fieldset = **no frame** (owner 2026-10-03: framed groups looked like
  another form concept), legend = the tool label above the fields, like a field's label (tools.css).
  details/summary = a regular uppercase label row (`--control-height`).
- **Icons:** Feather via `Icon` (see "Icons"; the close cross is NOT Feather, see "Close cross"): menu, chevrons for prev/next, rotate-ccw/cw for undo/redo,
  arrow-up/down for reorder, plus, info, search, sort arrows; icon-only controls carry `aria-label` + `title`.

**Room.**
- Tables are roomy: cells `space-075 space-1`. `DataTable` `secondary` columns show from **1600px** (they used to from
  768px): at 1440 a list keeps its key columns, the rest is in the detail.
- Generous padding: panels `space-1`, page content `space-15`, gaps between fields `space-1` (`space-15` after a
  `Field`). Inspector forms use **two fields per row**.
- The tool frame (tokens): top bar = ActionBar height `--tool-bar-height` max(48px, 3rem) (48 · 48 · 72px), sidebar
  `--tool-sidebar-width` 12rem (admin 14rem with descriptions), Info / side panels `--tool-panel-width` 16rem. Every
  region (sidebar, the main, the dock panels, the outline, the inspector) follows ONE model – tiles, one Divider,
  one fold to a rail (every rail's button at the height of the region headers' buttons), remembered sizes; several page
  panels side by side; maximize = fold everything else: agents/DOCKING.md.
- The ActionBar is the page's chrome, not a region: it stays while a panel / the inspector is maximized or the main is
  folded to its rail (10.57 – Save / Publish / undo still act on the main's document; DOCKING.md "Maximize").
- ActionBars that don't fit move `start` + `secondary` into "More" (`breakpoint`); the primary actions stay. Filters
  that need more than one line leave the bar and sit above the list (`.toolbar`), the pager stays in the bar.

**Check:** `npm run check:tools-css` (part of `npm run check`): no px font size and nothing under caption in the
tools' CSS (creator, admin, embassy, `@scutoons/ui`, tools.css; a line with `px-ok` + the reason is exempt, e.g. the
lettering preview). `npm run check:tools-ui` (dev servers + `npm run dev:test-user`; `SCUTOONS_CREATOR|ADMIN|EMBASSY`
for other origins): every tool page at 1440 px (fine pointer) and 390 px (touch) – no visible text under the caption
size (comic lettering exempt), and on touch no button / field / nav entry under 44 px; since 2026-10-02 also every
single-line control exactly `--control-height` ("One control height" below).

**Vocabulary** (`tools.css`): `.sc-label`, `.sc-heading`, `.sc-caption`, `.sc-emphasis`, `.sc-chip` (`is-ink`, `is-live`, `is-off`),
`.sc-frame`, `.sc-nav-head`, `.sc-nav-item` (`aria-current` / `.is-current` inverted), `.sc-selected` (+ `[data-sc-region][data-live]`, "Selection" above), `.sc-chip-control`, `.sc-control`
("One control height"), `.sc-checker` (a transparency checker behind cut-out art, `--checker-size`). Creator-wide button classes stay
`.secondary`, `.ghost`, `.active` (creator layout).
