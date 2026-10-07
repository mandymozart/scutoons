# UI concepts — the refactors behind BACKLOG E.1–E.6 (drafted 2026-10-07, for the owner to pick)

Written by the night run from the code as it is (file:line refs as of commit 964d6676). Nothing here is built yet –
HANDOVER's UI rule: design drafts before bigger UI. Each concept: **today**, **proposal**, **steps** (in dependency
order), **owner decides**. Order = the order to build them in (each one frees room or context the next one needs).

---

## E.1 One notification model (M.5 + M.13)

**Built 2026-10-07 (BACKLOG 10.44)** – as proposed; owner: toasts bottom-right, the language chip next to Publish
(phones `de ▾`, Save reads "Save"). Now: `creator/src/lib/toasts.ts` + the layout's ToastStack; `DocSession.notify`;
`status/live-chip.svelte`; `page/doc-notices.svelte` = conflict / autosave / converted only; the shared toast's
`action`. The rule: DESIGN.md "Notifications". (The "Today" below is the state before.)

**Today.** The creator's messages are `.sc-bar` rows **in the page flow** above the editor
(`creator/src/lib/components/page/doc-notices.svelte`): unsaved changes (inverted), autosave recovery, converted
document, event ids assigned, conflict (Reload / Overwrite), and `DocSession.notice` – one string with a × for
"Published", "Saved", "Nothing to save", errors (`editor/doc-session.svelte.ts:50`, set at :100–:162). Plus the
chapter editor's cast notice (Undo) and the file status row (`status/file-status.svelte`: a chip per language that
isn't live). All of them push the canvas down; the slice canvas then fits what is left, so every message shrinks the
slice. **Toasts already exist** in `@scutoons/ui` (`toasts.svelte.ts`, `toast-stack.svelte`: fixed bottom-right,
aria-live, 4 s, errors stay) – admin and embassy use them, **the creator doesn't**.

**Proposal – three kinds, three places, none moves the layout:**
1. **Transient info → toast** (fades by itself): Published, Saved, Nothing to save / publish, review submitted, event
   ids assigned. The creator mounts the shared ToastStack (like admin's `+layout.svelte:221`) **above the ActionBar**
   (bottom-right, over the content, never in flow).
2. **State that needs doing → in the ActionBar**, where the doing happens: unsaved changes = the Save button's dot
   (already "Save draft •") – the inverted bar goes; not-live languages = a compact status chip next to Publish
   ("de · draft") that opens the file-status list as a popover; the cast notice's Undo = a toast with an action.
3. **Blocking problems → one bar, only then:** conflict (Reload / Overwrite), autosave recovery, converted document –
   rare, they must be answered; they stay in flow but only these three.
Errors: a toast that stays until dismissed (the shared stack already does that).

**Steps.** (a) `DocSession.notice` → `toast()` (one context in the creator, like admin's `lib/toasts.ts`), the bar
goes; (b) the dirty bar → the ActionBar's Save state; (c) `file-status` → a chip + popover in the ActionBar (meta pages
too); (d) the cast notice → a toast with Undo (the shared toast needs an `action` – a small addition); (e) checks:
creator-chapter / creator-dock look for these bars by testid – update.
**Size:** medium, creator only + one toast option in shared/ui. **Frees:** the rows above the slice (M.13).
**Owner decided (2026-10-07):** toasts bottom-right; the language status as a chip next to Publish – built.

---

## E.2 Header + breadcrumbs (M.14)

**Built 2026-10-07 (BACKLOG 10.45)** – the "…" menu (not a scroll). `@scutoons/ui` `Breadcrumbs` (`items`, `label`,
`moreLabel`, `root` snippet, `keep` = 2); the creator layout passes `nav.crumbs` with the logo as root, `.brand-name`
is gone; the season crumb from `CreatorNav.chapterSeason` (the season whose sequence lists the chapter). Under 768px the
crumbs have their own row under the header and the text language + Flow sit at the top of the ☰ drawer. When even the
last two crumbs don't fit, the trail wraps. The text below is the concept as written.

**Today.** The creator builds its top bar itself (`creator/src/routes/+layout.svelte:257–290`): logo + "CREATOR"
(`.brand-name`) + `<nav class="crumbs">` from `lib/nav.svelte.ts` (`base` from the route: toon, **season** (hard-coded
season 1 for chapters, `:106–107`), chapter …; `extra` from the page: group, "Slice n"). Overflow: `nowrap`, middle
crumbs shrink with an ellipsis each (min 3em), under 479 px only the current crumb. No "…" menu.

**Proposal.** One crumb trail SCUTOONS › Toon › Season › Chapter › Group › Slice:
- the logo is the root crumb (SCUTOONS) – the "CREATOR" word goes (owner: "the creator title can go"); the app name
  stays in the tab title and the sidebar's top;
- when the trail doesn't fit, the left crumbs (after the root) collapse into one **"…" button** that opens a small menu
  of them (keyboard: Enter / arrows); the last two crumbs never collapse; each crumb keeps its full text (no
  per-crumb ellipsis – that made them unreadable);
- the season comes from the chapter's real season (the season list knows where the chapter is), not "1".
A shared `Breadcrumbs` component in `@scutoons/ui` (admin / embassy use the same pattern later).

**Steps.** (a) `Breadcrumbs.svelte` (measures with a ResizeObserver, collapses from the left); (b) the creator's
layout uses it, `.brand-name` goes; (c) the season crumb from the chapter's season; (d) Alt+↑ (`parent`) unchanged.
**Size:** small–medium. **Needed by** E.3 (the trail is the hierarchy).
**Owner decides:** the "…" menu vs a horizontal scroll of the crumbs.

---

## E.3 Navigation hierarchy (M.6)

**Today.** Two navigations: the left sidebar (chapter tree → selection, the URL follows by `replaceState`,
`chapter-editor.svelte:385–424`) and the Flow in the right dock. The chapter Flow's level is the deepest selection
unless picked; **every selection change remounts the whole graph** – `flow-panel.svelte:118`
`{#key `${level}:${current}:${slice}`}` → SvelteFlow + dagre again → `flow-controls.svelte:18–30` centres the
current box at zoom 1 again. That is the "jump". The toon / season Flow navigates with `goto(…?flow=1)`, which
remounts the dock panel on the next page.

**Proposal.** The sidebar + the main area ARE the hierarchy (with E.2's trail); the Flow and Ask only **read** the
current place and never navigate by themselves except when the author clicks a box (an explicit "go there").
- **The Flow keeps its viewport**: no `{#key}` remount on a selection change – the graph stays, the current box is
  highlighted, and the view only pans (animated, no zoom change) when the current box is outside it. A level change
  (chapter ↔ group ↔ slice) is the only rebuild, and it keeps the zoom.
- The Flow's level follows the main area by default (as now), the Segmented stays as "look wider"; a click on a box =
  select it in the main area (as now), never a level switch by itself.
- Toon / season Flow: the same panel instance across `goto` (the dock remembers the panel kind; the page hands it the
  new input) – no remount when moving toon → season → chapter.
**Steps.** (a) **the bug fix first** (small, can go ahead without the rest): replace the `{#key}` by updating the
graph's nodes / edges in place + "pan only when off-screen"; (b) the dock keeps a panel across navigation when the new
page registers the same kind; (c) Ask reads the trail (it already uses the crumbs).
**Size:** (a) small, (b) medium (dock.svelte.ts). **Owner decides:** (a) can be done right away – yes?
**Status (2026-10-07): (a) built** (47a5c8cd – the `{#key}` and the zoom-1 first view described under "Today" are gone;
`flow-panel.svelte` rebuilds only for another level). **(b) built** (BACKLOG 10.46): a dock panel with a `kind` + a
`content` (component + props) is mounted by the shell (`@scutoons/ui` dock.svelte.ts `DockContent`, app-shell.svelte);
its page leaving HOLDS it, the next page docking the same kind takes the instance over (no remount, fold kept), nobody
→ it closes after the navigation (`DockHold` in a loading route's `{#await}`); pages open their own when
`shellDock.has(kind)`. The Flow is ONE `FlowDock` (creator `flow/flow-dock.svelte`) for toon / season / chapter: the
header, width and fold stay, a new level (toon ↔ season ↔ chapter ↔ group ↔ slice, another chapter / toon) is rebuilt
at the kept zoom, group → group on one level keeps the graph and its view; the toon / season boxes `goto` without
`?flow=1` (a deep link still opens it). Also kept: Problems (`ProblemsDock`) and the ActionBar's Info (every app);
History and the admin's list detail stay page-bound. **(c) built**: Ask already took its context from `nav.crumbs`
(the Breadcrumbs' source, the real season since E.2); it now also sends the deepest crumb's address as the agent's
`ref` (`/catpope/chapter/2?group=…`). Check: `npm run check:creator-flow-dock` (19). agents/DOCKING.md "Panels across
pages".

---

## E.4 Slice editor room (M.4)

**Built 2026-10-07 (BACKLOG 10.47)** – the owner's draft (strip left of the canvas, not over the art; 50–300 %).
`slice/slice-toolbar.svelte` = the strip: two square vertical groups (`sc-seg is-column`; `Segmented` `vertical`).
**View**: the grid button (pressed = snap, its size in the corner; hold, right-click, the corner number or → open the
sizes 5 / 10 / 15 / 30 – a pick also switches snap on), the zoom button (flyout: zoom out · % · zoom in · fit · 1:1 –
one square group like the Flow's since 10.52, not the draft's round buttons), and with `{…}` texts an eye button whose
flyout holds the sample-reader select. **Content**: text / item / sound / event. One flyout at a time beside its
button; Esc / ← / a click outside closes it (focus back). Zoom = `ChapterView.zoom` (null = fit, else a factor of the
slice's 1x width, `scutoons_creator_zoom`, try/catch) – `slice-canvas.svelte` `.canvas-wrap` is laid out at
`--slice-w × --zoom` inside the scrolling `canvas-area` (no transform: boxes, handles, snapping stay in %); fit = the
area's height, capped at 2x (a phone: its full width). Ctrl / Cmd + wheel over the area zooms around the pointer
(`wheelZoom` in chapter-editor, flyout open or not). The toolbar row is gone. Phones: the same strip at the canvas's
left, buttons ≥ 2.75rem. Checks: check:creator-chapter's E.4 part; shots `e4-*`.

**Today.** `slice/slice-toolbar.svelte`: Snap toggle + a grid select (5/10/15/30, localStorage) + a Segmented of the
tools (text / item / sound / event). `slice-canvas.svelte` fits the column's height and is **capped at the slice's 1x
width** (`max-width: min(<width>px, 100cqh / ratio)`) – on a big screen it never grows; no zoom. The inspector is a
22rem region beside it (below it under 1280 px).

**Proposal.**
- **A vertical tool strip** on the canvas's left edge (round icon buttons, the same tools), the toolbar row goes;
- **Snap = one small button** (the grid icon, pressed = on); a click-and-hold / a small chevron opens the sizes (5 /
  10 / 15 / 30 px);
- **the canvas fills** the area (cap at 2x the slice's width instead of 1x) with **zoom**: −/+ / fit buttons at the
  bottom-right like the Flow's (the same look, `flow-controls` vocabulary), Ctrl + wheel, the zoom remembered per
  device; handles and snapping work in slice coordinates (they already do – the canvas scales them);
- with E.1 the rows above the canvas are gone – the slice takes that room too.
**Steps.** (a) the tool strip + the Snap button (layout only); (b) the cap 1x → 2x + a `zoom` factor on
`.canvas-wrap` (CSS transform-free: width × zoom, the scroll area scrolls); (c) the zoom controls (a small shared
`ZoomControls` the Flow could use too).
**Size:** medium. **Owner decides:** zoom range (50–300 %?), the strip left of the canvas or inside it.

---

## E.5 The VN message box + end actions (M.3)

**Status: built** (2026-10-07, BACKLOG 10.48 – (a) earlier with M.3 part 1): the end actions are choices of the message
box on every chapter page (VN or not), handed over as a snippet (`vn.setEnd`); the separate bar is gone, the characters
stand on the box. DESIGN.md "Follow-up bar" has the details.

**Today.** The message box is fixed at the bottom (`toon/vn/message.svelte:83–99`, slides out by `translateY`); its
height is reported (`vn.setBoxHeight`) and the avatar / portrait stand on the box's top
(`characters.svelte:47–50`: ground = box height + 1rem). `offsetHeight` ignores the slide-out transform, so **with
no box showing they still stand where the box's top would be** – the owner's "should sit lower". The end actions
(`chapter/next-bar.svelte`) are a separate fixed bar (z 110 > the characters' 101), appear without a transition and
cover the characters' feet.

**Proposal.**
- **ground = the visible box's top, else the page bottom**: the VN context reports 0 while no box is shown (the box's
  `shown` state, not its offsetHeight) → the characters slide down to `1rem`; when a box (or the end actions) appears,
  they slide up with it (they already animate `bottom` 0.3 s).
- **The end actions inside the message box** (owner's rethink): the next bar's choices (next chapter, back to the toon,
  the season's music, buy …) become the box's choice buttons – the same style as `showChoices`, so the reader learns
  one gesture: "the box at the bottom is where I act". The chapter's last text box and the end actions can share the
  box (text above, the actions as choices).
- The separate fixed bar goes; `next-host`'s reserved 9rem stays (the page can scroll its end clear of the box).
**Steps.** (a) the ground fix (small, viewer only – could go first); (b) the next bar's content as a "box mode" of
`message.svelte` (choices + the cover / music row); (c) check-next-bar / chain-vn checks.
**Owner decides:** (a) right away? (b) drafts on the real page (phone + desktop) first – which actions in the box.

---

## E.6 Strip preview for vertical groups (M.9) and the viewer as a dock panel (M.10)

**Strip preview today** (`chapter/strip-preview.svelte`): one reader screen (phone 390:844 / desktop 16:9), the
slices side by side at screen height with the reader's own `Slice`, wheel → sideways; mounted only for a horizontal
full-width group (`chapter-editor.svelte:800–802`). **For vertical groups:** the same component with a vertical track
(slices stacked at the screen's width, `overflow-y`), the frame's fixed height, no wheel remap; mount it for every
group. Small. Better after E.4 (the same editor's room).

**The viewer as a panel (exploration):** a third dock panel next to Flow / Ask: an iframe of the viewer's draft
preview (`?draft=1`, the creator's session) at phone width, reloaded on every saved draft (DocSession's save event),
and a `postMessage` "show group / slice" when the selection changes (the viewer scrolls there). Needs: the viewer to
accept the message (only from the creator's origin), the dock's E.3(b) (a panel that survives navigation). Explore
after E.3. **Owner decides:** worth a spike?

---

## E.7 Copy / cut / paste (BACKLOG 10.39)

**Built 2026-10-07 (BACKLOG 10.39, steps 1–8).** `lib/editor/clipboard.svelte.ts` (the DOM copy / cut / paste events,
the live region's handler, Shift+V, the menu's copy / paste) + `clip.ts` (the JSON with a marker, fresh ids, media
references across toons, settings only) – handlers: the slice canvas + inspector (`slice-clip.ts`), the slice list,
the chapter tree, a group's VN events. Phones: the ActionBar's "More" holds Copy · Cut · Paste · Paste settings; a
right-click / long-press opens the same menu (`clip-menu.svelte`). Item limits (`placements`, `max_world`) in the
catalog, the API and the creator. DESIGN.md "Copy / paste", ASSETS.md "Paths in content", ITEMS.md "Item limits".
The text below is the concept as written.

**Today:** one canvas selection (`ChapterView.selection`, chapter-state.svelte.ts), each list keeps its own
multi-selection (`picked`, multi-select.ts – groups, slices, events); Ctrl+X / Delete / Backspace delete the canvas
selection (chapter-editor.svelte onKeydown); no copy, no duplicate, no context menu.

**Proposal:**
- **The live area:** the selection model of E.8 – the region clicked / focused last is "live", its selection is drawn
  in the live look (light cyan, dark blue text); Cmd/Ctrl+C / X / V act on the live selection only; with none, nothing
  happens. In a text field the keys stay the browser's. Canvas and the inspector's text box / item list are ONE region.
- **What pastes where:**

  | Active selection | V (as new) | Shift+V (settings only, onto the selection) |
  |---|---|---|
  | text box | a new box (same slice: one grid step offset) | the look: variant, frame, shape, tail, font, stroke, rotation, scale |
  | placed item | a new placement (random id) | action, qty, repeat, respawn, events |
  | sound / event area | a new area | trigger + params |
  | VN events (several) | after the selected event | params of the same type + `if` |
  | slices | after the selected slice, with boxes, items, events | audio + events |
  | groups | after the selected group | renderer, `end`, `continues` |

- **Feedback:** a toast ("3 events copied", "pasted – uses 4 files from catpope", "only 1 placement allowed – also in
  catpope ch. 3"). With E.1 (one notification model).
- **Later, optional:** the same actions in a right-click menu on the canvas and the lists.

- **Toast:** the shared ToastStack (`@scutoons/ui`), success style (cyan left bar): "Text box copied · s1c1-2-3",
  "2 events copied", "Slice copied · 3"; above the ActionBar (E.1).
- **Phone:** only one region shows at a time (the slice list is in the menu drawer) and phones have no Cmd keys –
  copy / paste would be desktop-only until the optional menu. The toast must not cover the inspector's last row (short-lived
  or at the top on narrow screens).

**Steps:** E.8 first (the live region is what the keys act on) → text boxes → the rest (BACKLOG 10.39).
**Decided (owner 2026-10-07):** the live look as in the E.8 draft. **Owner decides:** phone desktop-only for now, or
the menu?

---

## E.8 One selection model, app-wide (creator, admin, embassy; BACKLOG 10.40)

**Built 2026-10-07 (BACKLOG 10.40), steps 1–6.** Tokens `--select-live-bg / -border / -text`; tools.css `.sc-selected`
+ `[data-sc-region][data-live] .sc-selected` (table rows framed by their cells; a non-live region nested in the live
one stays inverted) – the only selection CSS; `@scutoons/ui` `region` action + `liveRegion` store
(`selection.svelte.ts`: `initial`, `modal`; the innermost region wins). Regions: creator chapter editor `tree` ·
`slices` · `slice` (canvas + the whole inspector: Layers, audio samples, slice / item events – one canvas selection) ·
`events` (group script); characters · people · `map` (nav lists + map, one region) · `cover` (canvas + Lines); every
DataTable (admin); modal: Sheet, the palette, the character / person comboboxes, the expression builder. The chapter
editor's keys and the map's Esc / arrows act only while their region is live. Hover never inverts (menu items, people
rows, map nav, `.sc-round`, the palette trigger), focus = the 3px ring everywhere (wins over `all: unset`; toggle 3px).
The cover box: focus is not selection (Enter / Space select). Not migrated (as decided): nav "you are here" (sidebar,
script outline, Flow node, admin / embassy nav) stays inverted; the tree's group row is not selected
while one of its slices is open; the map canvas markers keep their halo (a canvas object). Checks: creator-dock
(the live region on load, a click in the slice list), tools-dock (the admin's open row is the live selection, at most
one live region in the embassy), creator-chapter (the keys with the slice list live). The text below is the concept as
written.

**Why:** copy / paste (E.7) has to show WHICH selection the keys act on. The inventory (2026-10-07, every editing app)
shows that the inverted look carries six meanings, while "selected" is drawn four ways:

- **Inverted ink / paper today means:** nav "you are here" (`.sc-nav-item.is-current` / `aria-current`, admin + embassy
  nav, the Flow node `.current`, the script outline `.current`) · the edited object (slice `li.selected`, DataTable
  `tr.active`, slice-inspector + cover-inspector lists, people rows) · the keyboard highlight in a listbox (command
  palette, character + person pickers) · a toggle that is on (tabs, segmented, `.ghost.active`, `.on`, aria-pressed) ·
  **hover** (`.sc-round:hover`, menu items, people-panel `.row:hover`) · a row in progress (admin batches `tr.current`).
- **"Selected" is drawn as:** inverted (above) · a 2px ink border (VN event `.event.selected`, sample `.sample.selected`,
  characters `.item.active`) · a kind-coloured border + cyan tint (slice canvas `.box.selected`) · a cyan ring / halo
  (cover box, map place marker, map route).
- **Multi-pick ("picked")** = a 2px ink inset outline in slice list / groups / events, shown inconsistently (the slice
  list excludes the selected one, groups / events only from 2 on); DataTable checks = `--surface-hover`.
- **No region model:** the chapter editor's Delete / Backspace / Ctrl+X / arrows act on the canvas selection even when
  focus is in the slice list or the tree (chapter-editor.svelte onKeydown, no region check). The only region notion is
  `data-sc-maximize` (app-shell.svelte, Ctrl+Space).
- **Focus gaps:** rows with `all: unset` beat the `:where(.sc-tool)` focus rule (map-builder `.item`, characters `.item`,
  people `.row` have no visible focus); toggle.svelte's ring is 2px, not 3px; the menu draws focus as inversion; the
  cover box treats focus as selection.

### The vocabulary: one look per meaning

| Meaning | Look | Tokens |
|---|---|---|
| **Selected** (content: what the inspector / editor shows; every picked one too) | inverted, the brutalist look, unchanged | `--ink` bg, `--paper` text |
| **Live selection** (selected AND in the live region: what the keys act on) | light cyan background, 2px inset border in the primary colour, dark blue text, not bold | new `--select-live-bg` (= `--surface-tint-cyan`), `--select-live-border` (= `--primary-color`, owner 2026-10-07), `--select-live-text` (#005a70, tested on the page) |
| **Keyboard highlight in a listbox** (palette, pickers, comboboxes, expr-builder) | = live selection (focus is in that list) | as above |
| **Picker tile being chosen** (`.sc-card.is-selected`, asset / expression tiles) | = live selection (the open sheet is the live region) | as above |
| **Current value of a picker field** (`.sc-card.is-current`) | cyan left bar (unchanged) | `--primary-color` |
| **Nav "you are here"** (sidebar, tree's current group, Flow node, outline, admin / embassy nav) | inverted, unchanged (owner 2026-10-07: "keep as is"); when it is ALSO the live selection (the tree's group in the live tree region) it gets the live look | `--ink` / `--paper` |
| **Toggle on / tab / segmented / pressed** | inverted: small controls, stays | `--ink` / `--paper` |
| **Hover** | never inverted: 2px ink border / outline (as `.sc-nav-item:hover` does) | `--ink` |
| **Focus (keyboard)** | 3px ink outline everywhere (fix the gaps) | `--border-width-focus` |
| **Drag / drop** | cyan drop line, moving 0.5 (unchanged) | `--primary-color` |
| **Dimmed** (other layer, filtered) | opacity (unchanged) | – |
| **In progress** (admin batches row) | not a selection: `--surface-tint-cyan` + its status text, not inverted | – |

**Canvas objects** (slice canvas, cover canvas, map) can't be inverted, the art is under them. Selected = the kind's
frame without handles; live = the frame + handles + the tag (arrows / Delete / Cmd+C act on it). The cover box stops
treating focus as selection (focus = the focus ring; a click selects).

**Multi-pick:** every picked item has the selected look (inverted, or live when its region is live); the slice list's
"the shown one vs the others" distinction goes (the main area shows which one is open). Copy / Delete / drag take all
picked ones. DataTable's checked rows = the selected look.

### The live region

- **One mechanism in `@scutoons/ui`:** a `region` action (`use:region={'slices'}`) on each selectable area + one shared
  store `liveRegion` (`selection.svelte.ts`). `pointerdown` / `focusin` inside a region makes it live; the region gets
  `data-live`. Components mark selected items with ONE class `sc-selected` (replacing `.selected` / `.active` /
  `.is-current`-as-selection / `.picked` / `.item.active` …). tools.css then holds the only rules: `.sc-selected`
  (inverted) and `[data-live] .sc-selected` (live look). No per-component selection CSS any more.
- **Keys act on the live region only:** Cmd/Ctrl+C / X / V (E.7), Delete / Backspace, arrows (nudge / move), Esc
  (clears the live selection). The chapter editor's onKeydown asks `liveRegion` instead of acting on the canvas
  regardless, which fixes "Delete in the slice list deletes the canvas box".
- **Modal sheets / dialogs / the palette** are live while open; closing returns live to the region before.
- **Default:** on page load the main editor region is live (the canvas on a slice, the events on a VN group, the list
  on admin catalog pages).
- **Phone:** one region visible at a time; it is live, no extra state shows.

### Where the regions are

- **Creator:** chapter tree (groups) · slice list · slice canvas + its inspector box / item list (one region) · slice
  audio samples · VN events · script outline · map builder: lists + map canvas · characters list · cast · people panel ·
  cover canvas + Lines list (one region). The Flow panel is nav only (no selection); season sequence has hover only.
- **Admin:** each DataTable (catalog lists, codes, assets, users …) · the detail panel's own lists.
- **Embassy:** only its nav, no regions needed.
- **Shared:** command palette, menu, the picker sheets (asset, character, item), comboboxes.

### Steps (each one shippable)

1. **Tokens + tools.css:** `--select-live-*`, `.sc-selected`, `[data-live] .sc-selected`; the hover / focus fixes
   (`.sc-round:hover` not inverted; the focus ring must win over `all: unset`); DESIGN.md "Active / selected =
   inverted" rewritten to the vocabulary above.
2. **`@scutoons/ui`:** `selection.svelte.ts` (the store) + the `region` action; DataTable (`tr.active` / `tr.checked`
   → `sc-selected`, the table is a region); command palette + menu (highlight = live look, hover not inverted);
   toggle's 3px focus ring; `.sc-card.is-selected` → live look.
3. **Creator chapter editor:** slice list, chapter tree, VN events, slice canvas + inspector lists, slice audio →
   `region` + `sc-selected`; picked = selected look; onKeydown scoped to the live region.
4. **Creator other pages:** script outline, map builder (+ the focus fix), characters, cast, people panel, cover editor
   (focus ≠ selection), pickers.
5. **Admin + embassy:** the DataTables, admin nav, the batches progress row (not inverted).
6. **Checks:** the browser checks that assert `.selected` / `.active` / `.picked` (creator-chapter, creator-dock, events,
   admin) move to `.sc-selected` / `[data-live]`; one check per app that at most one region has `data-live`.

Then E.7 (copy / paste) builds on `liveRegion`.

**Draft (2026-10-07, CSS injected on catpope ch1 slice 3, no code):** live = `--surface-tint-cyan` background, 2px
inset border in `--primary-color`, text `#005a70`; selected elsewhere = inverted as today. **Owner: "like it"; border =
the primary colour.** Contrast: text ≈ 7.5:1 on the tint; the cyan border alone on white is ≈ 1.8:1 – the background +
text carry the state.

**Multi-select draft** (headless, slices 2–4 picked): list live = three cyan rows, each with its own inset border (draw
the border as `box-shadow: inset 0 0 0 2px`, not an outline – outlines of neighbouring rows overlap); canvas live = the
three slices inverted as one black block, the text box row cyan. The slice open in the editor stands out only by its
+ / × tools.

**Decided (owner 2026-10-07):** nav "you are here" stays inverted; only the live selection gets the cyan treatment;
the border is the primary colour.
**Owner decides:** picked = the selected look (the multi-select draft) – fine?
