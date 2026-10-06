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
| Info (help) | `shared/ui/src/action-bar.svelte` → `useDock`, `info-panel.svelte` | shell dock | 16rem, drags, DockFold + rail; takes turns with the page's panels | `:info`, open state `infoKey` |
| List + detail | `shared/ui/src/split-view.svelte` (admin catalog, codes) → `useDock` | shell dock | 24rem, drags, DockFold + rail, Escape / ✕ close; on phones covers the page | `:detail` |
| Ask | `shared/ask/src/ask-panel.svelte` via the shell's `ask` (creator, admin, embassy) | shell outermost right | `panelWidth` 24rem … ½, its own Divider; full screen < 768px | `:ask` |
| Right drawers | `shared/ui/src/sheet.svelte` (`side="right"`) | modal | Divider inside the drawer, normal … ½ window | `sc-sheet-width:<kind>` |

Gone with the migration: the creator's `usePageDock` / `nav.dock` (→ `useDock`), AppShell's `dock` / `dockSize` props,
the fixed Info panel (`--info-panel-top/-width`, its 1100px page margin), `--ask-inset`, `--ask-top`, `--ask-width`,
`html.sc-ask-open`, the embassy's own Ask margins, SplitView's sticky pane + its narrow-screen Sheet + `breakpoint`
(+ `--split-view-top/-bottom`), the script outline's own fold state / separator / px sizes, the Sheet's own drag edge
markup, Divider's `fixed` mode.

Checks: `npm run check:creator-dock` (maximize: Flow + inspector by button / Esc / Ctrl+Space, the main's unsaved edit,
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
