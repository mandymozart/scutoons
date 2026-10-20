# DOCKING – one layout model for the tool shell (creator, admin, embassy, the storyboard studio)

Owner 2026-10-06: "these areas should all behave a bit more like in Blender or here in the Claude Code desktop app, so we
don't have to deal with their sizing and collapsing individually. Closing them is a different story." Status: approved
(owner 2026-10-06: "go go go, sounds good, we can still revise it later") and migrated (merged into `main`) –
every area below is on the model. Closing panels is unchanged (their ✕). Several page panels side by side, the main's
fold and maximize = "fold everything else": BACKLOG 10.57 (owner 2026-10-07, below). The Ask region is gone (owner
2026-10-09, agents/MCP.md – members use their own Claude through the SCUTOONS MCP).

## The model

```
 ┌──────────────────── top bar (crumbs · page actions · Reader · Flow) ────────────────────┐
 ├──────────┬╫────────────────────────────────┬╫─────────────┬╫─────────────┬╫─────────────┤
 │ LEFT     │╫ MAIN                           │╫ RIGHT DOCK: the page panels side by side   │
 │ sidebar  │╫ the page (may split itself:    │╫ detail · History · Problems · Flow ·       │
 │ (tree,   │╫  outline | text, canvas |      │╫ Reader · Info – each its own header, fold, │
 │ children)│╫  inspector); keeps ≥ 24rem;    │╫ maximize, ✕ and Divider; one per kind      │
 │ folds to │╫  folds to a rail only by hand  │╫ (e.g. │ Flow       │╫ Reader     │╫ Info)   │
 │ a rail   │╫  (10.57)                       │╫                                            │
 │          │╠════════════════════════════════╧╩═════════════╧╩═════════════╧╩═════════════╡
 │          │╫ BOTTOM (fill mode only) under the main AND the dock: e.g. the studio's      │
 │          │╫ timeline; its top edge a Divider; folds to a horizontal rail                │
 ├──────────┴╫─────────────────────────────────────────────────────────────────────────────┤
 └──────── bottom ActionBar (page actions – stays, also while maximized) ──────────────────┘
   ╫ ═ = a Divider (the only drag edge)    < 768px: sidebar = drawer, the newest panel covers the page, no bottom
```

- **Regions tile, never overlap** (from 768px): every open region pushes its neighbour; nothing is positioned over the
  page. The shell (`@scutoons/ui` `AppShell`, in creator, admin, embassy AND the storyboard studio) owns the grid:
  `sidebar | content | dock` (`--sc-shell-columns`), in fill mode with an optional row under content and dock ("Bottom
  region"). Pages never position panels themselves.
- **Panels live in regions**: a component hands its open panel to the right dock with `useDock(() => ({ kind, panel |
  content, size, title, icon, close }) | null)` (`@scutoons/ui` dock.svelte.ts, a module-level registry – one shell per
  app; one `useDock` per panel – the chapter editor calls it for the Flow, the Reader, Problems and History). Several
  panels are open at once, side by side (10.57, "Several panels" below); ONE per kind: a panel of the same kind from
  another owner replaces the shown one (its `close`, so that page's toggle follows).
- **Panels across pages** (UI-CONCEPTS E.3 b, 2026-10-07): a panel docked with a `kind` and a `content` (`{ component,
  props }` instead of the `panel` snippet) is mounted by the SHELL (`DockContent`), not by the page. When its page
  leaves, the dock HOLDS it (still shown, its last props); the next page that docks the same kind takes the mounted
  instance over and hands it its own props – no remount, no flash, the fold kept. Pages ask `shellDock.has(kind)` on
  init and open their own panel of that kind then (the Flow stays open toon → season → chapter without `?flow=1`). Every
  kind on its own (10.57): the Flow, the Reader, Problems and Info held together are taken over together, each in its
  place, with its width, its fold and its recency (`used`); the main's fold goes on across the navigation too.
  Nobody takes it: it closes after that navigation's task; a route still loading (`{#await languages}`) puts a
  `DockHold` in its pending branch, so the held panel waits until the page has mounted. Leaving the page ends a
  maximize. Kinds: `flow` (creator `FlowDock` – toon / season / chapter, the zoom kept across levels and pages),
  `problems` (`ProblemsDock` – chapter editor, meta pages), `reader` (`ReaderDock` – the chapter editor, below), `info` (the ActionBar's `InfoPanel` – every app). Page
  bound (a snippet, gone with the page): History (another document's history), the admin's list detail.
- **The Reader panel** (BACKLOG 10.49, UI-CONCEPTS E.6, 2026-10-07): the chapter editor's round `smartphone` button
  (top bar, left of the Flow's) docks `kind: 'reader'` (creator `lib/components/reader/reader-dock.svelte`, width
  `sc-panel-width:reader`, 24rem): the viewer's draft preview in an iframe – a phone, 390 × 844 CSS px, or a desktop
  window, 1440 × 810 (the header's square phone / desktop group, `sc-reader-screen` per device), scaled into the panel
  (the frame's viewport must be the device's exactly, whatever the creator's root size – a device size, not a UI size,
  so px there). Group → group, slice → slice and chapter → chapter (another toon too) keep the instance and the frame:
  the place goes by message, the frame does NOT reload. It reloads only after a stored draft of its toon (every
  `ContentDoc.save` reports to `draftSaves`, debounced 300 ms – a save of several files = one reload),
  or a change of language / screen; after each load it re-shows the place. The language: the header's text language
  when the reader has it (de / en), else the chapter's first of those.
  **Messages** (`@scutoons/content` reader-link.ts, only two): viewer → creator `scutoons:reader-ready` (sent with the
  creator's origin as target, so a foreign page framing the viewer gets nothing); creator → viewer
  `scutoons:reader-show { toon, chapter, group, slice }` (sent with the viewer's origin as target; group / slice are
  indexes of the saved draft – a group the draft hasn't got: nothing happens). The viewer
  (`viewer/src/lib/creator-link.svelte.ts`, listening in the root layout) accepts a message only when ALL hold: it is
  framed, `event.source === window.parent`, `event.origin` is the creator's configured origin (`CREATOR_URL` –
  `VITE_CREATOR_URL`, production https://creator.scutoons.com, dev http://localhost:10000), its draft preview is on, and
  the data parses (`parseReaderShow`: the type, a toon id `[a-z0-9_-]`, integer indexes in range). It then only moves
  the reader – another chapter: an in-app navigation; a group of another chain: its `?label=`; then it scrolls to the
  slice (a full-width strip's later slice: into the strip's sideways travel). The creator accepts only "ready", only
  from its frame's own `contentWindow` and the viewer's origin.
  **Session (security):** no token travels – not in the address, not in a message. The frame signs in with the shared
  single-sign-on cookie `scutoons_sid` (PLATFORM.md "SSO"): the creator and the viewer are ONE site (scutoons.com;
  locally localhost), so the viewer's API calls from inside the frame are same-site and the browser sends the
  `SameSite=Lax`, HttpOnly cookie as it does in a tab; the draft data still needs the role (`?draft=1` is checked by the
  API, Roles::CONTENT_EDITORS). A browser that refuses the cookie (the creator's Bearer fallback) shows the published
  reader in the frame (the viewer's preview switches itself off on 401 / 403) – no token is handed over to fix that,
  deliberately (a token in a URL leaks via history, logs and referrers; one sent by postMessage would put a session into a
  second origin's storage). `?draft=1` turns the viewer origin's per-device draft preview on, as the "Preview" links in
  More always did (`?preview=draft`). The frame has `referrerpolicy="no-referrer"` and `allow="autoplay; fullscreen"`
  (the reader's sound, a strip's full screen). Only the creator may frame the viewer (owner 2026-10-07): `viewer/static/.htaccess` sends
  `Content-Security-Policy: frame-ancestors` limited to the creator (PLATFORM.md; the messages are origin-checked too). Check: `npm run check:creator-reader`.
- **Several panels** (BACKLOG 10.57, owner 2026-10-07: "why is it either Flow or phone preview? … jumping through the
  Flow would also directly show the final result in the preview"): the page panels no longer take turns – any of them
  can be open together (the Flow + the Reader is the case that matters: a click in the Flow moves the main AND the
  Reader; Problems, History, Info and the admin's list detail join the same way). Rules:
  1. **One per kind, a fixed place** (`DOCK_ORDER`, from the main outwards): list detail · History · Problems · Flow ·
     Reader · Info. What belongs to the page's document sits next to it, the Flow drives the main, the Reader
     shows the result, help is outermost. A panel opens in its place – nothing jumps when another opens or
     closes (Blender's areas, not a tab stack); an app's own kinds (the storyboard studio's panels) between the
     Reader and Info by their `order` (0–499, clamped – never past Info), equal places by opening order.
  2. **Each panel is a region of its own**: its header (title – cut with … first when narrow –, its buttons), its
     `DockFold` (fold to its rail, maximize), its ✕, and a `Divider` on its left edge; dragging it resizes that panel
     (the panels left of it move, the main gives or takes the room; it stops where the main would get narrower than
     `minContent`). Widths remembered per kind as before.
  3. **Recency** (`used`): opening, unfolding, a click on its rail and maximizing make a panel the most recently used.
  4. **Keyboard**: Ctrl+Space maximizes the panel that has focus; from the main (outside a maximizable region) the most
     recently used open panel. Note: the Reader's frame can hold the focus – keys then go to the viewer (click the
     creator page first).
  5. **Phones**: one panel at a time – the most recently used covers the page, the others stay open (mounted) behind it
     and show again when it closes.
- **The main folds – by hand only** (10.57, owner 2026-10-07: "with the phone preview and Flow or the agent available,
  the main could also fold"): the leftmost open panel's header has a round "Fold the page" button (`chevrons-left`,
  `page-fold`) and Ctrl+Shift+\ toggles it (while a panel is open). Folded, the main is a `Rail` labelled with the
  page's place (the deepest crumb) – its button or a click on it, the key again, or the last panel closing bring it
  back; the panels take its room (the most recently used grows into the rest). It never folds by itself (the person
  decides when the page may go); not remembered across loads, but kept across navigation (a Flow box still navigates the
  folded main and the Reader follows). A Problems row's "go to" unfolds it (the field must show). The folded main stays
  mounted and laid out at its last width, only hidden – scroll, selection, unsaved edits stay. The ActionBar stays.
- **Bottom region** (owner 2026-10-19: "Actually the timeline underneath the other windows was also good" – supersedes
  decision 2): AppShell `bottom` (snippet) + `bottomSize` (`panelHeight(key, normalRem = 10)`), `bottomLabel` /
  `bottomIcon` (its landmark and rail), `bottomResizeLabel`, `bottomUnfoldLabel`. **Fill mode only** (page mode ignores
  it). A row under the content AND the dock – the sidebar keeps its full height, the dock's Dividers end on it. Its top
  edge is a `Divider` (`edge="top"`: ↑ / ↓, Home / double-click = the normal height, row-resize) from the normal height
  up to half the shell's frame (between the top bar and the ActionBar – `panelHeight.room`, set by the shell); a
  remembered height taller than that shrinks with a smaller window. It folds to a horizontal `Rail` (as thick as a rail
  is wide: the button 1rem from the left, icon, label) by the page's fold button (a `FoldButton side="bottom"` in the
  page's own header – the shell has none) or a drag below 60 % of the normal height; a drag from the rail only opens
  it. A rail while any region is maximized (a click on it restores). On its rail it stays MOUNTED (laid out at its
  height, hidden). Height remembered per key; the fold is the page's to remember (the studio: per step). Ctrl+Space
  inside it or on its edge does nothing (it maximizes nothing; Esc and Ctrl+Space still restore). Toasts sit above it
  while it shows (`--sc-shell-bottom-lift` on the root, read by `ToastStack`). Phones: hidden (still mounted) – the
  dock covers the page there; the studio is a desktop tool. First user: the storyboard studio's timeline – the shared
  `Timeline` (`@scutoons/ui` timeline.svelte, research/TIMELINE.md), which the creator adopts later.
- **ONE `Divider`** (`@scutoons/ui` divider.svelte) for every drag edge: `role="separator"`, 0.75rem hit zone on the
  2px border, 3px ink line on hover / focus, pointer drag, ← / → 16 px (Shift 64; the bottom region's top edge ↑ / ↓),
  Home or double-click = normal size, `html.sc-resizing` during a drag. It drives any size object (`resizable`,
  `dockRegion`, `panelWidth`, `panelHeight`). Placed in its region's container (no fixed mode any more).
- **ONE fold-to-rail** (`dockRegion`, dock-region.svelte.ts + `FoldButton` + `Rail`): the sidebar, the script outline,
  the right dock and the bottom region fold. The `FoldButton` (round, chevrons pointing where the region folds to) – the
  sidebar's at its top right, the outline's in its heading, the dock's (`DockFold`) in the docked panel's header next to
  its ✕, the bottom region's in the header of the page's content there; Ctrl+\ (sidebar only); or drag the edge below
  60 % of its normal size (snap, like Blender / VS Code); a drag that starts on the rail only opens it. The `Rail`
  (`--sc-shell-rail-width`: the round unfold button + 0.5rem either side): the unfold button on top, the region's icon
  (dock) and its label written vertically (sidebar: where you are; outline: "Outline"; dock: the panel's title – Flow,
  Problems, History, Info, the detail's title; the bottom region's: in one row); a click anywhere on it unfolds.
  Remembered: the sidebar's and the outline's fold (`sc-sidebar-folded:<app>`, `sc-script-outline-folded`); the dock's
  is not – a panel always opens unfolded (owner 2026-10-06 decision 3 + "closing is a different story"); the bottom
  region's is the page's. Folded "for the page's sake" (`foldWhen`) is not remembered.
  A dock panel, the main and the bottom region on their rail stay MOUNTED (10.57): laid out at their last size, only
  hidden – the Flow's zoom, the Reader's frame, a scroll position survive the rail. Every rail's round button sits at
  the height of the region headers' buttons (a dock panel's: one control row with `--space-05` above it; owner
  2026-10-07 – `check:creator-dock` / `check:tools-dock` measure it, ±1 px).
- **Sizes remembered per panel kind** (localStorage, per browser – owner rule: no server state for chrome):
  `sc-sidebar-width:<app>`, `sc-sidebar-folded:<app>`, `sc-panel-width:chapter` (Problems/History in the chapter,
  16rem), `:meta` (meta pages, 24rem), `:flow` (every level, 28rem), `:info` (16rem), `:detail` (admin list detail,
  24rem), `:slice-inspector` (22rem), `sc-script-outline-width` (16rem) / `-folded`,
  `sc-sheet-width:<kind>` (right drawers: picker, assets, history, problems, asset, form), the bottom region's height
  per `panelHeight` key (`storyboard-studio:timeline`, 12rem). Normal size = the minimum; max = half the window
  (sidebar a third; the inspector half the editor's area; the bottom region half the shell's frame).
- **Running out of room** – who gives way, in order (10.57: several panels):
  1. the sidebar never folds for room by itself (decision 1); it folds only while the page's `foldWhen` holds (no page
     sets it today) or a region is maximized – not remembered, unfolding by hand wins;
  2. a panel dragged wider stops where the main would get narrower than `minContent` (creator 24rem, admin / embassy
     20rem) next to the sidebar and the other panels (`panelWidth.room`, set by the shell);
  3. the open panels get narrower together – each by the same share of what it is above its floor – down to 12rem each
     (or its normal width, if smaller), while the main keeps `minContent`; a kind whose contents collide below that sets
     its own floor, `DockPanel.min` in rem (the storyboard studio's inspectors) – it folds sooner instead;
  4. even the floors don't fit: the LEAST RECENTLY USED panel folds to its rail (`dock-rail-squeezed`, not remembered),
     then the next – the newest you opened always shows. A click on a squeezed panel's rail makes it the most recently
     used: it comes back and the now least recently used folds instead;
  5. the last panel alone gets narrower than its floor only while the main keeps half its minimum;
  6. the main never folds by itself (only by hand, above); inside it the page's own rules (the slice inspector moves
     below the canvas under 1280px, the script outline above the text under 900px).
  At 1440 (60rem at the tools' 24px root) the Flow + Reader don't both fit next to a 24rem main – the Flow (opened
  first) folds to its rail, "Fold the page" gives both room (`check:creator-reader`).
- **Maximize = fold everything else** (owner 2026-10-06; 10.57, owner 2026-10-07: "maximize = fold everything else"):
  any dock panel (Flow, Reader, Problems, History, Info, the admin's list detail) and the chapter editor's slice
  inspector can be MAXIMIZED for a moment: a round button in its header (Feather `maximize-2` "Maximize" / while
  maximized `minimize-2` "Restore", de "Maximieren" / "Wiederherstellen", `title` + "(Ctrl + Space)"; the dock's sits in
  `DockFold` between the fold button and ✕ – the fold button hides while maximized; the inspector's right of its tabs).
  Maximized, it takes the room and EVERY other region goes to its rail – the sidebar, the main (a dock panel
  maximized), the other panels, the bottom region – one mechanism with the folds above (the shell derives the rails
  from `shellDock.maximized`; nothing's own fold state changes, so restore brings back exactly what was open before).
  The railed regions stay mounted, laid out at their last width, hidden: widths, folds, scroll positions, selection,
  unsaved edits, the Reader's frame stay. A click on any rail while maximized restores. Back: the button, Esc (not while
  a dialog / popover / menu / open combobox shows, nor in a text field – that Escape is theirs; a maximized Info / list
  detail restores on Esc instead of closing), Ctrl+Space, or a click in a Flow (a box, a dead place) or on a Problems
  row: restore AND navigate the main there. Ctrl+Space: maximizes the region that has focus (`data-sc-maximize`:
  `dock:<id>` or a main region's id), from anywhere else the most recently used open panel – from inside the bottom
  region or on its edge nothing (it never maximizes); focus moves to the maximized region's button and comes back on a
  keyboard restore; nothing in the apps uses Ctrl+Space (IME composition is left alone). One maximized region at a time
  (`shellDock.maximized`); a panel opened while another is maximized takes the maximize over (the other stays open on
  its rail); closing the panel, leaving the slice (inspector) or the page ends it. Not remembered across page loads.
  Shared: `shellDock.maximize/restore`, `MaximizeButton`, AppShell (`max-dock` / `max-main`, `main-railed`); a main
  region (the inspector) hides the rest of its page itself (`.columns.inspector-max`; the editor's file bar stays).
  Phones: no button.
  **The ActionBar stays** (10.57 (2), owner: "when in maximize mode the footer disappears – not 100 % intended"): it
  vanished only as a side effect – the page renders it inside the main, and hiding the main (`visibility`) hid it too.
  Kept on purpose now (`.main-railed … .sc-action-bar { visibility: visible }`): it is the page's chrome like the top
  bar, not a region; Save / Publish / undo / redo / the live chip / Info still act on the main's document, and the
  maximized Flow / Reader / inspector are exactly the views that drive or show that document (an edit in the inspector
  wants its Save; the Reader reloads on Save). No reason to hide it on any width: phones have no maximize, and the
  shell already reserves its height. The same for the main folded by hand.
- **Phones (< 768px)**: no dividers, no fold buttons, no maximize, no main fold, no bottom region (hidden, still
  mounted); the sidebar is the burger drawer, the most recently used panel covers the page (closed by ×; the other open
  panels wait behind it).
- **Modal drawers stay modal**: `Sheet side="right"` (pickers, the admin's history / problems / asset / new batch) is a
  choice or a side task, not a docked panel – its edge is the same `Divider` (inside the drawer), its width remembered
  per kind (`widthKey`).

## Why (the models, what fits "less UI is better UI")

- **Blender** – areas tile the window, a border is shared by its neighbours and dragging it resizes both; regions
  (toolbar, sidebar) collapse to a thin strip with a small arrow and snap shut when dragged below their minimum;
  hotkeys toggle them (T / N); editors are swappable per area. Taken: tiling, shared borders, snap-to-fold, a hotkey.
  Not taken: free splitting / joining and editor-type switching – the creator's areas are fixed per page (less UI).
- **VS Code** – primary sidebar, secondary sidebar, bottom panel; the activity bar is a rail; Ctrl+B toggles the
  sidebar; one "sash" look everywhere (hover highlight, double-click resets); sizes per workspace; a view dragged below
  its minimum hides. Taken: one divider component, the rail, a keyboard toggle, double-click reset, one panel per side.
- **Claude desktop app** – a session sidebar that collapses, side panes that push the transcript (no overlays), drag
  dividers, remembered widths, panes close with their own ×. Taken: panes push, widths remembered, closing separate
  from sizing.

## Areas (2026-10-06, migrated)

| Area | Where | Region | Size / fold / keys | Remembered |
| --- | --- | --- | --- | --- |
| Left sidebar + rail | `shared/ui/src/app-shell.svelte` (creator, admin) | shell left | `dockRegion`: Divider, 12rem (admin 14rem) … ⅓ window; folds (FoldButton, drag < 60 %, Ctrl+\\); folds by itself for `foldWhen` / a maximized region | `sc-sidebar-width/-folded:<app>` |
| Chapter tree | in the sidebar (`usePageNav({ sidebar })`) | = sidebar | = sidebar | = sidebar |
| Script outline | `script/script-editor.svelte` + `script-outline.svelte` | page left | `dockRegion` + Divider + Rail ("Outline"), 16rem … ½ page; above the text < 900px | `sc-script-outline-width/-folded` |
| Slice inspector | `chapter/chapter-editor.svelte` (`.inspector-region`) | page right | `panelWidth` + Divider, 22rem … ½ the editor's area; below the canvas < 1280px; maximizes | `sc-panel-width:slice-inspector` |
| Problems / History / Flow (chapter) | `chapter-editor.svelte` → `useDock` | shell dock | `panelWidth(… { foldable })` 16rem / Flow 28rem; DockFold + rail | `:chapter`, `:flow` |
| Problems / History / dock (meta pages, season Flow) | `page/meta-page.svelte` → `useDock` | shell dock | 24rem / 28rem; DockFold + rail | `:meta`, `:flow` |
| Flow (toon page) | `[toonId]/+page.svelte` → `useDock` | shell dock | 28rem; DockFold + rail | `:flow` |
| Flow across pages | `flow/flow-dock.svelte` (`kind: 'flow'`), `shared/ui` dock.svelte.ts / dock-content / dock-hold | shell dock | one instance toon → season → chapter, zoom kept | `:flow` |
| Info (help) | `shared/ui/src/action-bar.svelte` → `useDock`, `info-panel.svelte` | shell dock | 16rem, drags, DockFold + rail; next to the page's panels, outermost | `:info`, open state `infoKey` |
| List + detail | `shared/ui/src/split-view.svelte` (admin catalog, codes) → `useDock` (`kind: 'detail'`) | shell dock | 24rem, drags, DockFold + rail, Escape / ✕ close; next to Info; on phones covers the page | `:detail` |
| Reader | `reader/reader-dock.svelte` (`kind: 'reader'`) | shell dock | 24rem, DockFold + rail; next to the Flow | `:reader` |
| Newsletter preview | admin `lib/components/newsletter-preview.svelte` (`kind: 'newsletter-preview'`, #98) | shell dock | 30rem, DockFold + rail, phone / desktop width; between the Reader's place and Info | `:newsletter-preview` |
| Main (folded) | AppShell `main-rail` (`shellDock.foldMain`) | shell main | by hand: the leftmost panel's "Fold the page", Ctrl+Shift+\; rail with the page's place | not remembered |
| Bottom region (timeline) | AppShell `bottom` + `panelHeight` (`shell-bottom.svelte`; storyboard studio: `Timeline`, its FoldButton) | shell bottom (fill) | Divider on top, 12rem … ½ the frame; folds (the page's FoldButton, drag < 60 %), a rail while maximized; hidden on phones | `storyboard-studio:timeline`; the fold per step (the studio's) |
| Right drawers | `shared/ui/src/sheet.svelte` (`side="right"`) | modal | Divider inside the drawer, normal … ½ window | `sc-sheet-width:<kind>` |

Gone with the migration: the creator's `usePageDock` / `nav.dock` (→ `useDock`), AppShell's `dock` / `dockSize` props,
the fixed Info panel (`--info-panel-top/-width`, its 1100px page margin), `--ask-inset`, `--ask-top`, `--ask-width`,
`html.sc-ask-open`, the embassy's own Ask margins (and on 2026-10-09 the Ask region itself: `ask` snippet, `:ask`,
`ask-railed`), SplitView's sticky pane + its narrow-screen Sheet + `breakpoint`
(+ `--split-view-top/-bottom`), the script outline's own fold state / separator / px sizes, the Sheet's own drag edge
markup, Divider's `fixed` mode.

Checks (10.57 additions): `npm run check:creator-reader` (Flow · Reader side by side at 1440 + 1920, the Flow
folds for room at 1440 and "Fold the page" gives both room, a Flow box moves the main and the Reader – same frame,
maximize folds everything else and keeps the Reader's frame and the ActionBar, Esc restores; shots `DOCK2_SHOTS`
dock2-*), `check:creator-dock` (three panels in their places, the least recently used fold first and a squeezed rail's
click brings it back, every rail's button at one height (the panels' header buttons) ±1 px at 1440 / 1920, Ctrl+Shift+\, the ActionBar
while a panel or the inspector is maximized, another panel takes the maximize over), `check:tools-dock` (Info next to
the list detail, the admin's rails aligned, the maximized detail's rails + ActionBar).
Checks: `npm run check:creator-flow-dock` (panels across pages: the same Flow toon → season → chapter → chapter, group
→ group keeps the graph, the zoom kept, a box navigates, a page without a Flow closes the held one, Problems chapter →
season, the admin's Info codes → assets); `npm run check:creator-dock` (maximize: Flow + inspector by button / Esc / Ctrl+Space, the main's unsaved edit,
scroll and widths survive, a Flow box restores + navigates, another panel stays maximized, closing ends it; dock at 1280 / 1600 / 1920, sidebar fold, dock rail by button / Enter /
drag, outline fold + drag, inspector region, phone) and `npm run check:tools-dock` (36: admin list detail in the
dock at 1280 / 1600 / 1920, Info next to it, rail, drag, Escape, catalog form, phone; embassy AppShell, phone). `DOCK_SHOTS=<dir>` (+ `DOCK_PREFIX`) for screenshots.

## Decisions (owner 2026-10-06)

1. The sidebar does NOT fold for room by itself (then: only an unfolded dock AND Ask together folded it; with Ask gone,
   2026-10-09, only `foldWhen` / a maximized region). (Prototyped and dropped: folding it whenever the content gets
   narrow hid the chapter tree while the Flow was open.)
2. No bottom region for now – superseded 2026-10-19 (owner: "Actually the timeline underneath the other windows was
   also good"): "Bottom region" above.
3. The right dock folds to a rail like the sidebar (fold button, drag narrow, the rail shows the panel's icon + title,
   a click unfolds; ✕ still closes). Not remembered: a panel opens unfolded.

4. (10.57, owner 2026-10-07) Several page panels side by side, one per kind, in fixed places; the room runs out → the
   least recently used folds first; the main folds only by hand (never by itself); maximize = fold everything else to
   its rail; the ActionBar stays while maximized; every rail's button at the height of the region headers' buttons (then Ask's, now the dock panels').

## Later options

- Ctrl+\ for the dock / the outline too (today only the sidebar has a key).
- The dock's fold remembered per panel kind, if folding turns out to be a habit rather than a moment.
- The main folding by itself as the last step when the room runs out (owner's alternative; today only by hand).
- A shortcut per panel kind (open / focus the Flow, the Reader).
