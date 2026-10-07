# DOCKING – one layout model for the tool shell (creator, admin, embassy)

Owner 2026-10-06: "these areas should all behave a bit more like in Blender or here in the Claude Code desktop app, so we
don't have to deal with their sizing and collapsing individually. Closing them is a different story." Status: approved
(owner 2026-10-06: "go go go, sounds good, we can still revise it later") and migrated on branch `ui/docking-areas` –
every area below is on the model. Closing panels is unchanged (their ✕).

## The model

```
 ┌──────────────────────────── top bar (crumbs · page actions · Flow · Ask) ─────────────────────────────┐
 ├──────────┬╫──────────────────────────────────────────┬╫──────────────────────┬╫────────────────────────┤
 │ LEFT     │╫ MAIN                                     │╫ RIGHT DOCK           │╫ ASK                   │
 │ sidebar  │╫ the page (may split itself:              │╫ ONE page panel:      │╫ app-wide, outermost   │
 │ (tree,   │╫  outline | text, canvas | inspector)     │╫ Problems / History / │╫ region of the shell   │
 │ children)│╫                                          │╫ Flow / Info / a      │╫ (`ask`)               │
 │ folds to │╫ keeps ≥ minContent (24rem) when a dock   │╫ list's detail; folds │╫ closes (×)            │
 │ a rail   │╫ is dragged wider                         │╫ to a rail, closes ×  │╫                        │
 ├──────────┴╫──────────────────────────────────────────┴╫──────────────────────┴╫────────────────────────┤
 └──────────────────────────────── bottom ActionBar (page actions) ──────────────────────────────────────┘
   ╫ = a Divider (the only drag edge)                   < 768px: sidebar = drawer, dock + Ask cover the page
```

- **Regions tile, never overlap** (from 768px): every open region pushes its neighbour; nothing is positioned over the
  page. The shell (`@scutoons/ui` `AppShell`, in creator, admin AND embassy) owns the grid: `sidebar | content | dock |
  ask` (`--sc-shell-columns`). Pages never position panels themselves; no app reads Ask's width (`--ask-inset`,
  `--ask-top`, `html.sc-ask-open` are gone).
- **Panels live in regions**: a component hands its open panel to the right dock with `useDock(() => ({ panel, size,
  title, icon, close }) | null)` (`@scutoons/ui` dock.svelte.ts, a module-level registry – one shell per app). ONE at a
  time: opening one calls the shown one's `close`, so its page's toggle follows (Problems / History / Flow / a page's
  own dock, the ActionBar's Info, the admin's list detail take turns). Info restored from localStorage never displaces a
  panel the page opened itself (`?flow=1`); a panel that takes the dock from Info leaves Info remembered.
- **Panels across pages** (UI-CONCEPTS E.3 b, 2026-10-07): a panel docked with a `kind` and a `content` (`{ component,
  props }` instead of the `panel` snippet) is mounted by the SHELL (`DockContent`), not by the page. When its page
  leaves, the dock HOLDS it (still shown, its last props); the next page that docks the same kind takes the mounted
  instance over and hands it its own props – no remount, no flash, the fold kept. Pages ask `shellDock.has(kind)` on
  init and open their own panel of that kind then (the Flow stays open toon → season → chapter without `?flow=1`).
  Nobody takes it: it closes after that navigation's task; a route still loading (`{#await languages}`) puts a
  `DockHold` in its pending branch, so the held panel waits until the page has mounted. Leaving the page ends a
  maximize. Kinds: `flow` (creator `FlowDock` – toon / season / chapter, the zoom kept across levels and pages),
  `problems` (`ProblemsDock` – chapter editor, meta pages), `reader` (`ReaderDock` – the chapter editor, below), `info` (the ActionBar's `InfoPanel` – every app). Page
  bound (a snippet, gone with the page): History (another document's history), the admin's list detail. Ask is a region
  of the layout and never remounts; it reads the place from the crumbs (E.3 c).
- **The Reader panel** (BACKLOG 10.49, UI-CONCEPTS E.6, 2026-10-07): the chapter editor's round `smartphone` button
  (top bar, left of the Flow's) docks `kind: 'reader'` (creator `lib/components/reader/reader-dock.svelte`, width
  `sc-panel-width:reader`, 24rem): the viewer's draft preview in an iframe – a phone, 390 × 844 CSS px, or a desktop
  window, 1440 × 810 (the header's square phone / desktop group, `sc-reader-screen` per device), scaled into the panel
  (the frame's viewport must be the device's exactly, whatever the creator's root size – a device size, not a UI size,
  so px there). Group → group, slice → slice and chapter → chapter (another toon too) keep the instance and the frame:
  the place goes by message, the frame does NOT reload. It reloads only after a stored draft of its toon (every
  `ContentDoc.save` and every Ask save report to `draftSaves`, debounced 300 ms – a save of several files = one reload),
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
  (the reader's sound, a strip's full screen). The viewer sends no frame-blocking header today; a
  `Content-Security-Policy: frame-ancestors` limited to the creator would need the static hosts' .htaccess – not done
  (the messages are origin-checked anyway). Check: `npm run check:creator-reader`.
- **ONE `Divider`** (`@scutoons/ui` divider.svelte) for every drag edge: `role="separator"`, 0.75rem hit zone on the
  2px border, 3px ink line on hover / focus, pointer drag, ← / → 16 px (Shift 64), Home or double-click = normal width,
  `html.sc-resizing` during a drag. It drives any size object (`resizable`, `dockRegion`, `panelWidth`). Placed in its
  region's container (no fixed mode any more).
- **ONE fold-to-rail** (`dockRegion`, dock-region.svelte.ts + `FoldButton` + `Rail`): the sidebar, the script outline
  and the right dock fold. The `FoldButton` (round, chevrons pointing where the region folds to) – the sidebar's at its
  top right, the outline's in its heading, the dock's (`DockFold`) in the docked panel's header next to its ✕; Ctrl+\
  (sidebar only); or drag the edge below 60 % of its normal width (snap, like Blender / VS Code); a drag that starts on
  the rail only opens it. The `Rail` (2.5rem): the unfold button on top, the region's icon (dock) and its label written
  vertically (sidebar: where you are; outline: "Outline"; dock: the panel's title – Flow, Problems, History, Info, the
  detail's title); a click anywhere on it unfolds. Remembered: the sidebar's and the outline's fold
  (`sc-sidebar-folded:<app>`, `sc-script-outline-folded`); the dock's is not – a panel always opens unfolded (owner
  2026-10-06 decision 3 + "closing is a different story"). Folded "for the page's sake" (`foldWhen`) is not remembered.
- **Sizes remembered per panel kind** (localStorage, per browser – owner rule: no server state for chrome):
  `sc-sidebar-width:<app>`, `sc-sidebar-folded:<app>`, `sc-panel-width:chapter` (Problems/History in the chapter,
  16rem), `:meta` (meta pages, 24rem), `:flow` (every level, 28rem), `:info` (16rem), `:detail` (admin list detail,
  24rem), `:slice-inspector` (22rem), `:ask` (24rem), `sc-script-outline-width` (16rem) / `-folded`,
  `sc-sheet-width:<kind>` (right drawers: picker, assets, history, problems, asset, form). Normal width = the minimum;
  max = half the window (sidebar a third; the inspector half the editor's area).
- **Running out of room** – who gives way, in order:
  1. the sidebar folds to its rail when an unfolded dock AND Ask are open (owner rule, decision 1: no fold without
     Ask; the shell does it for every app; a dock folded to its rail needs no room; unfolding by hand wins);
  2. a dock dragged wider stops where the content would get narrower than `minContent` (creator 24rem, admin / embassy
     20rem) next to the sidebar and left of Ask (`panelWidth.room`, set by the shell; a smaller window: the same in CSS);
  3. only when even its normal width leaves the content less than half of that, the dock gets narrower than normal;
  4. inside the main region the page's own rules (the slice inspector moves below the canvas under 1280px, the script
     outline above the text under 900px);
  5. Ask never shrinks by itself (the person sized it).
- **Maximize, never fold the main** (owner 2026-10-06): the MAIN region never folds. Instead any dock panel (Flow,
  Problems, History, Info, the admin's list detail) and the chapter editor's slice inspector can be MAXIMIZED for a
  moment: a round button in its header (Feather `maximize-2` "Maximize" / while maximized `minimize-2` "Restore", de
  "Maximieren" / "Wiederherstellen", `title` + "(Ctrl + Space)"; the dock's sits in `DockFold` between the fold button
  and ✕ – the fold button hides while maximized; the inspector's right of its tabs). Maximized, it fills everything
  between the sidebar (folded to its rail, `foldWhen`, not remembered) and Ask (unchanged). The main area and the other
  regions are only HIDDEN (`visibility`, the grid stays as it was – the maximized region spans the content's and the
  dock's columns above the hidden one): nothing unmounts, widths, folds, scroll positions, selection and unsaved edits
  stay. Back: the button, Esc (not while a dialog / popover / menu / open combobox shows, nor in a text field – that
  Escape is theirs; a maximized Info / list detail restores on Esc instead of closing), Ctrl+Space, or a click in a
  Flow (a box, a dead place) or on a Problems row: restore AND navigate the main there. Ctrl+Space: maximizes the
  region that has focus (`data-sc-maximize`), from anywhere else the open dock panel (not from Ask); focus moves to the
  maximized region's button and comes back on a keyboard restore; nothing in the apps uses Ctrl+Space (IME composition
  is left alone). One maximized region at a time (`shellDock.maximized`: `'dock'` or a main region's id); another panel
  taking the dock keeps the maximize (the dock is what's maximized); closing the panel, leaving the slice (inspector)
  or the page ends it. Not remembered across page loads. Shared: `shellDock.maximize/restore`, `MaximizeButton`,
  AppShell (`max-dock` / `max-main`); a main region (the inspector) hides the rest of its page itself
  (`.columns.inspector-max`; the editor's file bar stays). Phones: no button (the dock covers the page anyway).
- **Phones (< 768px)**: no dividers, no fold buttons, no maximize; the sidebar is the burger drawer, the dock and Ask cover the page
  (closed by ×).
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
| Left sidebar + rail | `shared/ui/src/app-shell.svelte` (creator, admin) | shell left | `dockRegion`: Divider, 12rem (admin 14rem) … ⅓ window; folds (FoldButton, drag < 60 %, Ctrl+\\); `foldWhen` + dock & Ask | `sc-sidebar-width/-folded:<app>` |
| Chapter tree | in the sidebar (`usePageNav({ sidebar })`) | = sidebar | = sidebar | = sidebar |
| Script outline | `script/script-editor.svelte` + `script-outline.svelte` | page left | `dockRegion` + Divider + Rail ("Outline"), 16rem … ½ page; above the text < 900px | `sc-script-outline-width/-folded` |
| Slice inspector | `chapter/chapter-editor.svelte` (`.inspector-region`) | page right | `panelWidth` + Divider, 22rem … ½ the editor's area; below the canvas < 1280px; maximizes | `sc-panel-width:slice-inspector` |
| Problems / History / Flow (chapter) | `chapter-editor.svelte` → `useDock` | shell dock | `panelWidth(… { foldable })` 16rem / Flow 28rem; DockFold + rail | `:chapter`, `:flow` |
| Problems / History / dock (meta pages, season Flow) | `page/meta-page.svelte` → `useDock` | shell dock | 24rem / 28rem; DockFold + rail | `:meta`, `:flow` |
| Flow (toon page) | `[toonId]/+page.svelte` → `useDock` | shell dock | 28rem; DockFold + rail | `:flow` |
| Flow across pages | `flow/flow-dock.svelte` (`kind: 'flow'`), `shared/ui` dock.svelte.ts / dock-content / dock-hold | shell dock | one instance toon → season → chapter, zoom kept | `:flow` |
| Info (help) | `shared/ui/src/action-bar.svelte` → `useDock`, `info-panel.svelte` | shell dock | 16rem, drags, DockFold + rail; takes turns with the page's panels | `:info`, open state `infoKey` |
| List + detail | `shared/ui/src/split-view.svelte` (admin catalog, codes) → `useDock` | shell dock | 24rem, drags, DockFold + rail, Escape / ✕ close; on phones covers the page | `:detail` |
| Ask | `shared/ask/src/ask-panel.svelte` via the shell's `ask` (creator, admin, embassy) | shell outermost right | `panelWidth` 24rem … ½, its own Divider; full screen < 768px | `:ask` |
| Right drawers | `shared/ui/src/sheet.svelte` (`side="right"`) | modal | Divider inside the drawer, normal … ½ window | `sc-sheet-width:<kind>` |

Gone with the migration: the creator's `usePageDock` / `nav.dock` (→ `useDock`), AppShell's `dock` / `dockSize` props,
the fixed Info panel (`--info-panel-top/-width`, its 1100px page margin), `--ask-inset`, `--ask-top`, `--ask-width`,
`html.sc-ask-open`, the embassy's own Ask margins, SplitView's sticky pane + its narrow-screen Sheet + `breakpoint`
(+ `--split-view-top/-bottom`), the script outline's own fold state / separator / px sizes, the Sheet's own drag edge
markup, Divider's `fixed` mode.

Checks: `npm run check:creator-flow-dock` (panels across pages: the same Flow toon → season → chapter → chapter, group
→ group keeps the graph, the zoom kept, a box navigates, a page without a Flow closes the held one, Problems chapter →
season, the admin's Info codes → assets, Ask's place); `npm run check:creator-dock` (maximize: Flow + inspector by button / Esc / Ctrl+Space, the main's unsaved edit,
scroll and widths survive, a Flow box restores + navigates, another panel stays maximized, closing ends it; dock at 1280 / 1600 / 1920 + Ask, sidebar fold, dock rail by button / Enter /
drag, outline fold + drag, inspector region, Ask, phone) and `npm run check:tools-dock` (36: admin list detail in the
dock at 1280 / 1600 / 1920 + Ask, Info takes turns, rail, drag, Escape, catalog form, phone; embassy AppShell + Ask,
Ask's Divider, phone). `DOCK_SHOTS=<dir>` (+ `DOCK_PREFIX`) for screenshots.

## Decisions (owner 2026-10-06)

1. The sidebar does NOT fold without Ask – only an (unfolded) dock and Ask together fold it. (Prototyped and dropped:
   folding it whenever the content gets narrow hid the chapter tree while the Flow was open.)
2. No bottom region for now.
3. The right dock folds to a rail like the sidebar (fold button, drag narrow, the rail shows the panel's icon + title,
   a click unfolds; ✕ still closes). Not remembered: a panel opens unfolded.

## Later options

- A bottom region (Blender's timeline / VS Code's panel), e.g. for the script's or the chapter's audio – a row of the
  shell above the ActionBar on the same `dockRegion` (vertical Divider); not needed today.
- Ctrl+\ for the dock / the outline too (today only the sidebar has a key).
- The dock's fold remembered per panel kind, if folding turns out to be a habit rather than a moment.
