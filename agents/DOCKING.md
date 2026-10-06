# DOCKING – one layout model for the tool shell (creator, admin, embassy)

Owner 2026-10-06: "these areas should all behave a bit more like in Blender or here in the Claude Code desktop app, so we
don't have to deal with their sizing and collapsing individually. Closing them is a different story." Status: proposal
+ prototype on branch `ui/docking-areas` (waiting for the owner's review). Closing panels is unchanged.

## The model

```
 ┌──────────────────────────── top bar (crumbs · page actions · Flow · Ask) ─────────────────────────────┐
 ├──────────┬╫──────────────────────────────────────────┬╫──────────────────────┬╫────────────────────────┤
 │ LEFT     │╫ MAIN                                     │╫ RIGHT DOCK           │╫ ASK                   │
 │ sidebar  │╫ the page (may split itself:              │╫ ONE page panel:      │╫ app-wide, outermost   │
 │ (tree,   │╫  outline | text, canvas | inspector)     │╫ Problems / History / │╫ (shell level, fixed;  │
 │ children)│╫                                          │╫ Flow / page's own    │╫ the frame ends at it) │
 │ folds to │╫ keeps ≥ minContent (24rem) when a dock   │╫ closes (×), never    │╫ closes (×)            │
 │ a rail   │╫ is dragged wider                         │╫ folds                │╫                        │
 ├──────────┴╫──────────────────────────────────────────┴╫──────────────────────┴╫────────────────────────┤
 └──────────────────────────────── bottom ActionBar (page actions) ──────────────────────────────────────┘
   ╫ = a Divider (the only drag edge)                   < 768px: sidebar = drawer, dock + Ask cover the page
```

- **Regions tile, never overlap** (from 768px): every open region pushes its neighbour; nothing is positioned over the
  page. The shell (`@scutoons/ui` `AppShell`) owns the grid: `sidebar | content | dock`; Ask sits outside it and the
  frame ends at Ask (`html.sc-ask-open .sc-shell-frame { margin-right }`). Pages never position panels themselves.
- **Panels live in regions**: a page hands its open panel to the right dock (`usePageDock(() => ({ panel, size }) | null)`,
  creator `nav.svelte.ts`); one at a time – Problems, History, Flow, a page's own dock take turns (as before).
- **ONE `Divider`** (`@scutoons/ui` divider.svelte) for every drag edge: `role="separator"`, 0.75rem hit zone on the
  2px border, 3px ink line on hover / focus, pointer drag, ← / → 16 px (Shift 64), Home or double-click = normal width,
  `html.sc-resizing` during a drag. It drives any size object (`resizable`, `dockRegion`, `panelWidth`).
- **ONE fold-to-rail** (`dockRegion`, dock-region.svelte.ts): only left regions fold (sidebar today, the script outline
  next). Fold button (chevrons) at its top right, Ctrl+\, or drag the edge below 60 % of its normal width (snap, like
  Blender / VS Code); a drag that starts on the rail only opens it. Folded is remembered; folded "for the page's sake"
  (`foldWhen`) is not.
- **Sizes remembered per panel kind** (localStorage, per browser – owner rule: no server state for chrome):
  `sc-sidebar-width:<app>`, `sc-sidebar-folded:<app>`, `sc-panel-width:chapter` (Problems/History in the chapter,
  16rem), `:meta` (meta pages, 24rem), `:flow` (every level, 28rem), `:slice-inspector` (22rem), `:ask` (24rem),
  `sc-script-outline-*`. Normal width = the minimum; max = half the window (sidebar a third).
- **Running out of room** – who gives way, in order:
  1. the sidebar folds to its rail when a dock AND Ask are open (owner rule, `foldWhen`; unfolding by hand wins);
  2. a dock dragged wider stops where the content would get narrower than `minContent` (24rem) next to the sidebar
     (`panelWidth.room`, set by the shell; a smaller window: the same in CSS);
  3. only when even its normal width leaves the content less than half of that, the dock gets narrower than normal;
  4. inside the main region the page's own rules (the slice inspector moves below the canvas under 1280px);
  5. Ask never shrinks by itself (the person sized it).
- **Phones (< 768px)**: no dividers; the sidebar is the burger drawer, the dock and Ask cover the page (closed by ×).

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

## Inventory (2026-10-06, before the prototype)

| Area | Where | Size / fold / keys | Remembered |
| --- | --- | --- | --- |
| Left sidebar + rail | `shared/ui/src/app-shell.svelte` (creator + admin layouts) | `resizable` right edge, normal `--tool-sidebar-width` 12rem (admin 14rem) … ⅓ window; folds (chevrons, drag < 60 %, Ctrl+\\); `foldWhen` | `sc-sidebar-width/-folded:<app>` |
| Chapter tree | in the sidebar (`usePageNav({ sidebar })`) | = sidebar | = sidebar |
| Script outline | `script/script-editor.svelte:57-80, 348` | own `resizable` + own fold logic (folded strip "Outline") | `sc-script-outline-*` |
| Slice inspector | `chapter/chapter-editor.svelte:73, 749, 824, 949` | `panelWidth` 22rem … ½; under 1280px below the canvas | `sc-panel-width:slice-inspector` |
| Problems / History (chapter) | `chapter-editor.svelte` | `panelWidth` 16rem; was `position: fixed` + page `margin-right` + own `.panel-resize` | `sc-panel-width:chapter` |
| Flow (chapter) | same panel, `panel === 'flow'` | `panelWidth` 28rem | `sc-panel-width:flow` |
| Problems / History / dock (meta pages, season Flow) | `page/meta-page.svelte` | `panelWidth` 24rem / 28rem; fixed + margin + own resize | `:meta`, `:flow` |
| Flow (toon page) | `flow/flow-dock.svelte` + `[toonId]/+page.svelte` | third copy of fixed + margin + resize | `:flow` |
| Ask | `shared/ask/src/ask-panel.svelte` (creator, admin, embassy) | `panelWidth` 24rem, fixed, pushes the shell frame, sets `--ask-inset` | `sc-panel-width:ask` |
| Info (help) | `shared/ui/src/info-panel.svelte` via ActionBar | fixed at `--ask-inset`, fixed `--info-panel-width`, pushes content ≥ 1100px; not resizable | open state `infoKey` |
| Right drawers | `shared/ui/src/sheet.svelte:45` (`side="right"`; pickers, admin history / problems / assets) | modal `<dialog>`, own resize edge | `sc-sheet-width` (one for all) |
| List + detail | `shared/ui/src/split-view.svelte` (admin catalog, codes) | fixed `minmax(22rem, 34%)` from 1100px | – |

Duplicated before: the fixed side-panel CSS + `margin-right: var(--panel-width)` + `.panel-resize` markup/CSS 4× (chapter
editor, meta page, flow-dock, Ask), separator markup 6× (+ AppShell, inspector, Sheet, outline), fold logic 2×
(AppShell, script outline), `pagePanel.open` flag set by 3 pages for the sidebar rule.

## Prototype (branch `ui/docking-areas`)

- `@scutoons/ui`: `Divider` (divider.svelte), `dockRegion` (dock-region.svelte.ts: resize + fold-to-rail + `foldWhen`),
  `resizable` sets `html.sc-resizing` itself, `panelWidth` gains `width`, `normal`, `room`.
- `AppShell`: the sidebar uses `dockRegion` + `Divider`; new `dock` region (`dock`, `dockSize`, `dockLabel`,
  `minContent`) replaces the unused `aside`.
- Creator: `usePageDock` + `flowWidth` (nav.svelte.ts); the chapter editor, MetaPage and the toon page dock their
  panel in the shell (fixed CSS, margins and three resize handles gone; `flow-dock.svelte` deleted); the slice
  inspector's edge is a `Divider`; `pagePanel` is gone (the layout folds the sidebar on `ask.open && nav.dock`).
- Ask: its edge is a `Divider` (`fixed`).
- Check: `npm run check:creator-dock` (32 checks, `DOCK_SHOTS=<dir>` for screenshots).

## Migration plan for the rest

1. **Script outline** → `dockRegion` + `Divider` (its fold logic and key are the sidebar's; rail label "Outline").
2. **Info panel** → a page panel in the right dock (takes turns with Problems / History / Flow; one remembered width)
   instead of its own fixed box at `--ask-inset`; `--info-panel-width` / the 1100px margin go.
3. **Slice inspector** → a region of the chapter editor's main area with the same rules (min, ½, below the canvas
   under 1280px) – already on `Divider`.
4. **Sheet `side="right"`** → its edge a `Divider`; stays modal (pickers are choices, not docked panels).
5. **SplitView** (admin) → its detail pane a right dock in the admin's AppShell (resizable, remembered).
6. **Embassy** → AppShell like admin / creator (today Ask pushes it via `--ask-inset`), then `--ask-inset` goes.

## Open decisions (owner)

- Fold the sidebar also when room runs out without Ask (e.g. Flow at 1280px leaves the content 13rem)? Prototyped and
  taken out again: it hides the chapter tree while the Flow is open (check:creator-reach clicks the tree).
- A bottom region (Blender's timeline / VS Code's panel) for the script's or the chapter's audio? Not needed today.
- Should the right dock fold to a rail too (a strip with the panel's name) instead of only closing?
