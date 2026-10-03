# COMPONENTS — structure analysis + proposed hierarchy (2026-10-03, proposal, not decided)

Owner 2026-10-03: "very large files in the components … editors are not splitting nicely logically … suggest a human
readable hierarchy … identify recurring components." Analysis by a read-only agent; nothing moved yet. Line numbers are
from commit 5ff43ad and drift.

**Summary**
1. Creator: 79 `.svelte` files, ~17.5k lines; 7 over 600. `chapter-editor.svelte` (1400) does ~12 jobs; `slice-canvas.svelte`
   has 490 lines of script, mostly pure geometry.
2. `creator/src/lib/components` is mostly flat (41 root files + 6 domain folders) and the folders leak
   (`credits/locale-map-field` used by characters + map; `map/slice-unlocks` used by the chapter editor).
3. Recurring UI copied many times: field grid `.grid.g3` defined 6× with 5 gaps; 4 drag-to-resize implementations;
   3 picker sheets with a copy-pasted `arrows()`; hand-built tabs in 4 places while `@scutoons/ui` `Tabs` is unused; 7 local
   `.chip` classes next to tools.css `.sc-chip`; 15 `confirm()` calls against the UI rule.
4. `meta-page.svelte` already does save / publish / conflict / problems / history / keyboard for 8 editors;
   `chapter-editor` re-implements it (copy-paste, already drifting). Unifying it is the biggest win and the biggest risk.
5. Proposal: domain folders + a `controls/` and `fields/` layer, editor state in `lib/editor/*.svelte.ts`, ~14 small
   independently mergeable steps, starting with a chapter-editor smoke check (none exists today).

## 1. Inventory

Creator components > 400 lines (script / css lines):

| file | lines | script | css | mixes |
|---|---|---|---|---|
| components/chapter-editor.svelte | **1400** | 747 | 317 | see below |
| components/slice-canvas.svelte | 736 | 490 | 150 | transform geometry, pointer drag, nudge API, render |
| components/characters/character-detail.svelte | 710 | 240 | 218 | identity, personas, looks, A-pose, artwork, wanted images |
| components/map/map-props.svelte | 706 | 264 | 230 | 3 forms (place / route / layer) in one |
| components/script/script-editor.svelte | 704 | 356 | 217 | own outline resize + fold, editor keys, meta, outline |
| components/cover/cover-editor.svelte | 629 | 257 | 219 | line list, drag/handle canvas, inspector, reader previews |
| components/map/map-builder.svelte | 643 | 326 | 180 | start view, selection↔URL, canvas edits, keyboard, validation |
| routes/[toonId]/+page.svelte | 591 | 335 | 131 | languages, locale files, season/chapter creation, own saveAll/publishAll |
| components/map/map-canvas.svelte | 522 | 248 | 177 | pan, route drawing, drag, render (fairly coherent) |
| routes/+layout.svelte | 508 | 205 | 208 | nav, keyboard, palette command building, top bar |
| components/assets/asset-picker-sheet.svelte | 490 | 228 | 162 | filters, card grid, upload, grid keyboard |
| components/vn-events.svelte | 470 | 231 | 88 | event list, a form per event type, trigger |
| components/characters/characters-editor.svelte | 460 | 252 | 111 | |
| components/meta-page.svelte | 418 | 219 | 111 | the shared editor frame (the good pattern) |
| components/slice-media.svelte | 404 | 203 | 77 | image + video slots, player flags, placement, frame ratio |

300–400: `characters/cast-editor` 390, `character-picker-sheet` 376, `expression-grid` 345, `season-editor` 312,
`credits/entry-list` 303. `.ts` modules are fine (largest `lib/editor/doc.svelte.ts` 426).

Other apps: **viewer** already uses domain folders (`components/{account,avatar,camera,chapter,home,items,toon}`); worst:
`contexts/inventory.svelte.ts` 817, `camera/camera-overlay` 648, `toon/map/map-overlay` 593, `items/claim-sheet` 567
(`viewer/dist/` has untracked build copies – exclude). **admin** colocates components with routes; largest
`claims/[id]/+page` 448. **embassy** `orgs/[id]/+page` 479. **shop** small. **shared**: `ui/app-shell` 455,
`reader/slice-video` 480.

### chapter-editor.svelte by line range

| lines | job |
|---|---|
| 62–66 | two `panelWidth` (side panel, inspector) |
| 67–166 | loads 7 docs (chapter, season, scene refs, cast, registry, moods, world map), cast auto-add/undo, legacy-text conversion, images |
| 170–207 | UI state: section / group / slice / selection / tab, snap + grid prefs |
| 238–366 | selection logic + URL↔selection sync |
| 368–436 | problems + `gotoProblem` |
| 438–573 | create / remove canvas objects |
| 575–613 | group add / move / remove (`confirm()`) |
| 615–678 | save / publish / conflict – copy of meta-page.svelte:128–177 |
| 679–700 | preview link, `beforeNavigate` + `confirm()` |
| 702–741 | global keyboard |
| 751–795 | sidebar tree |
| 800–845 | slice prev/next + ActionBar |
| 848–895 | file status + notice bars – copy of meta-page.svelte:251–282 (CSS 1136–1160 = meta-page 333–360) |
| 897–968 | section switch, slice toolbar, SliceCanvas |
| 970–1066 | slice inspector: resize edge, hand-built tablist (no `role="tab"`), 5 tab bodies |
| 1068–1080 | problems / history side panel |
| 1083–1400 | 317 lines CSS |

Drift between the two save/keyboard copies: `isField` checks `isContentEditable` in chapter-editor but not meta-page;
chapter-editor undoes the document even with the cursor in a field, meta-page skips undo in fields; publish messages differ.

### Natural splits elsewhere
- **vn-events**: row chrome (type select + up/down/remove) + one form per type (message, choice, jump, onomatopoeia,
  join/leave) + trigger.
- **slice-canvas**: pure geometry (`deltas`, `transformedDeltas`, `rotateTo`, `scaleTo`) → `lib/editor/canvas-geometry.ts`
  (unit-testable); pointer state machine; render.
- **map-props**: place / route / layer props + shared `map-toons-chips` (same chip row twice).
- **character-detail**: header, identity, personas+looks tabs, A-pose, artwork, wanted → one section component each.
- **cover-editor**: `cover-lines`, `cover-lettering-canvas`, `cover-inspector`, `cover-reader-previews`.
- **script-editor**: `script-outline` (fold + resize), `script-text` (input + keys), `script-meta`.
- **routes/[toonId]/+page**: `toon-languages-section`, `season-chapters-section`; save/publish → shared session; route
  becomes a thin wrapper like the others (28–30 lines).
- **routes/+layout**: palette command building → `lib/palette.ts`.

## 2. Creator today
- Routes are thin (28–30 lines, load languages + mount `*-editor`) except `routes/[toonId]/+page.svelte` (591) and
  `chapter/[n]/preview/+page.svelte` (258).
- Components mostly flat; folders `assets/` (1), `characters/` (11), `cover/` (2), `credits/` (8), `map/` (4), `script/` (2).
- Misplaced: `credits/locale-map-field` (generic), `map/slice-unlocks` (chapter editor only), `item-picker-sheet` at root
  while the other picker sheets are in folders.
- Mixed imports (`'../meta-page.svelte'` vs `$lib/components/...`).
- `lib/editor/*.ts` (15 files, ~1700 lines) is good but thin; selection / keyboard / save orchestration still in
  components; `lib/panel-width.svelte.ts`, `lib/nav.svelte.ts` at `lib/` root.
- Suffixes implicit / inconsistent: `-panel` means both main-column sections and side panels; `image-picker` is the old
  path-field picker (still in `shared-field.svelte`), against "media fields are slots".
- shared/ui exports `Tabs`, `Field`, `Panel`, `SplitView`, `Toggle`, `Menu`, `IconButton`, `Button`, `SearchField` – the
  creator uses none of them (only `Sheet`, `Badge`, `Section`, `CloseButton`, `ActionBar`, `Icon`).

## 3. Recurring UI

| pattern | where | proposal |
|---|---|---|
| Field grid `.grid.g3/.g4` | CSS in dialog-inspector, group-panel, slice-audio, vn-events, toon-editor, character-detail (6 defs, 5 gaps; "g3" = 2, 3 or auto-fill cols) | one tools.css `.sc-fields[data-cols]` + one gap token |
| Segmented / presets `.seg`, `.presets` | chapter-editor, cover-editor, dialog-inspector, vn-events, text-preset-picker, placement-picker, admin emails `.segments` | `Segmented.svelte` in @scutoons/ui + `.sc-seg` |
| "Advanced" folds | dialog-inspector, vn-events (re-styles summary although tools.css does), group-panel, script-editor, apose-editor | tools.css only, drop local summary CSS |
| Drag-to-resize edge | app-shell (sidebar), sheet, `panel-width.svelte.ts`, script-editor outline (own impl.) | one `resizable()` + `ResizeEdge.svelte` in @scutoons/ui |
| Tabs | chapter-editor (no `role="tab"`), cast-editor, character-detail, character-picker-sheet (same `.tab` CSS 3×) | @scutoons/ui `Tabs` (+ icon/count snippet) |
| Picker sheets | asset (490), character (376), item (138): same Sheet + `.filters` + `.cards`; `arrows()` copied verbatim | `PickerSheet.svelte` + `gridArrows` in `lib/editor/grid-keys.ts` |
| Comboboxes | character-picker, person-picker (same listbox logic), role-picker similar | `controls/combobox.svelte` |
| Status chips | tools.css `.sc-chip.is-*` exists; file-status `.badge.*`, status-badges `.st-*`, 7 local `.chip` | `.sc-chip` everywhere, one `StatusChip` |
| Reorder rows | chapter-editor, featuring-field, list-field, related-items-field, supporters-field, vn-events, cover-editor `moveLine`, map-props `moveWaypoint` | `controls/reorder-buttons.svelte` + `moveAt()` |
| Document session | meta-page vs chapter-editor vs routes/[toonId]/+page | `lib/editor/doc-session.svelte.ts` |
| Notice bars | meta-page = chapter-editor (markup + CSS) | `page/doc-notices.svelte` |
| Per-locale fields | `locale-text` (path) vs `locale-map-field` (value); `prop-field` vs `shared-field` | two families in `fields/`; check merging prop/shared-field |
| Media | media-slot, media-variants-slot, image-picker (old) | slots in `media/`, retire image-picker |
| `confirm()` | 15 calls (audio-panel, chapter-editor ×2, character-detail ×2, moods-panel, people-panel, sections-editor, history-panel, map-builder, map-props ×3, meta-page, slice-list) | `confirmDialog` (async – own step) |

## 4. Proposal

### 4.1 Target tree (creator/src/lib)

```
components/
  page/        editor-page (← meta-page), doc-notices NEW, side-panel NEW, problems-panel, history-panel, page-help,
               style-guide, up-link, nav-sidebar
  status/      file-status, status-badges, chapter-badges (all on .sc-chip)
  controls/    reorder-buttons NEW, combobox NEW, picker-sheet NEW
  fields/      prop-field, shared-field, list-field, locale-text, locale-map-field, text-original, featuring-field,
               related-items-field, supporters-field
  media/       media-slot, media-variants-slot, placement-picker, asset-picker-sheet
  chapter/     chapter-editor (shell < 300), chapter-tree NEW, meta-panel, audio-panel, group-panel, slice-list, slice-import
  slice/       slice-canvas, slice-toolbar NEW, slice-inspector NEW, slice-media, slice-audio, slice-unlocks
  text/        dialog-inspector, text-preset-picker, text-language-switch
  items/       item-inspector, item-picker-sheet
  events/      vn-events (list + row), event-message, event-choice, event-jump, event-onomatopoeia, event-cast, event-trigger
  map/         map-builder, map-canvas, place-props, route-props, layer-props, map-toons-chips
  characters/  characters-editor, character-detail (shell) + identity / personas / apose-section / artwork, cast-editor,
               expression-grid, apose-editor, moods-panel, wanted-images, asset-dialog, character-picker(-sheet), character-ref-field
  credits/     credits-editor, entry-list, people-panel, roles-panel, sections-editor, person-picker, role-picker
  cover/       cover-page, cover-editor (shell), cover-lines, cover-lettering-canvas, cover-inspector, cover-previews
  script/      script-editor (shell), script-outline, script-text, script-meta, script-rules
  toon/        toon-editor, season-editor, toon-languages-section, season-chapters-section (from the route)
editor/        doc, doc-session NEW, chapter-state NEW, editor-keys NEW, canvas-geometry NEW, grid-keys NEW,
               panel-width (from lib/), … existing
```

To @scutoons/ui: `Segmented`, `ResizeEdge` + `resizable()` (replaces app-shell / sheet copies), `Tabs` with icon/count.
To tools.css: `.sc-fields[data-cols]`, `.sc-seg`, `.sc-bar.*`, `.sc-chip` everywhere.

### 4.2 Rules (proposed)
1. One component = one job (if the header comment needs "and", split).
2. ≤ 300 lines per component, ≤ 150 lines of script; > 400 needs a reason in the header.
3. Non-markup state / logic (selection, save, key maps, geometry) in `lib/editor/*.ts|.svelte.ts`; pure functions stay testable.
4. No local copies of tools.css vocabulary (`.grid/.g3`, `.seg`, `.chip`, `.badge`, `.tab`, `summary`) – add to tools.css
   (could be enforced by `check:tools-css`).
5. Suffixes: `-editor` page, `-panel` main-column section, `-inspector` edits the selection, `-field` one bound value,
   `-picker` inline combobox, `-picker-sheet` Sheet, `-slot` media, `-canvas` drawing surface, `-section` part of an editor.
6. Imports via `$lib/...`, never `../`; used by two domains → `fields/`, `controls/` or @scutoons/ui.

### 4.3 Steps (smallest risk first; each: `npm run check` + `check:tools-css` + the named check)
0. **Safety net**: `scripts/dev/creator-chapter-check.mjs` (none covers the chapter editor) – tree, tabs, tools, add text,
   snap, prev/next, undo/redo, nudge / Delete / Escape, save draft.
1. **Folder moves only** (`git mv` + `$lib` imports).
2. **Field grid** → `.sc-fields` with a density by level (decided, see 4.4; visual change, show first).
3. **Chips** → `.sc-chip`.
4. **Folds**: drop local summary / `.advanced` CSS.
5. **ReorderButtons** + `moveAt`; **gridArrows** in asset + character pickers.
6. **Segmented** in @scutoons/ui (keep testids).
7. **PickerSheet** base (check: `check:creator-cast`).
8. **vn-events → one form per type** (check: `scripts/vn/vn-check.mjs` + step 0).
9. **map-props split** (`scripts/check-map.mjs`); then character-detail, cover-editor, script-editor sections.
10. **canvas-geometry.ts** + unit tests (creator needs vitest).
11. **doc-session + doc-notices + editor-keys**, first in meta-page, then chapter-editor + toon route.
    Owner decisions: see 4.4 (undo in a field, publish wording).
12. **chapter-editor split**: state → `chapter-state.svelte.ts`, then `chapter-tree`, `slice-toolbar`, `slice-inspector`.
    ⚠ selection / tool / tab are written by canvas, tree, inspector and keyboard – one state object; `canvas?.nudge` must
    stay reachable from the shell's keyboard; URL sync effects run once in the shell; keep `{#key}` remount.
13. **Resize unification** (`resizable()` in app-shell, sheet, panelWidth, script outline).
14. **Behaviour changes, each alone**: 15 `confirm()` → `confirmDialog` (`beforeNavigate` can't await: cancel, then
    `goto`); hand-built tabs → `Tabs` (`role="tab"`, arrow keys, testids); retire `image-picker`.

Done (branch `components`):
- Step 0 ✓ dc43ddc – `npm run check:creator-chapter` (44 checks, local stack only; `SCUTOONS_API` / `SCUTOONS_CREATOR` for
  side ports). Found: Audio → Meta in the tree leaves `?section=audio` in the URL (logged as a note, not asserted).
- Step 2 ✓ (the commit adding this line) – tools.css `.sc-fields[data-cols][data-density]` in the 6 places; local CSS gone
  except column overrides (toon-editor 2 cols < 900px, vn-events cast row `2fr 1fr 1fr auto`, character-detail
  `--fields-*-min`); `check:tools-css` flags new `.gN` copies / gaps on `.sc-fields`. 4c86e8a: the widths as tokens
  (the tokens check had gone red). 0942160: content schemas regenerated (`npm run check` was red before the branch).
- Fixes: 1fb604f URL ↔ selection compares with the address bar (replaceState leaves `page.url` behind) – chapter tree,
  characters, map builder; checked. Field grids the analysis missed (the commit after 1fb604f): audio-panel, meta-panel,
  season-editor (page), item-inspector, cover lettering inspector, script meta (panel), admin respawn-editor +
  asset-detail, embassy forms (page); `check:tools-css` also flags a bare `.grid/.two/.pair/.form-grid/.meta-grid` field
  grid (`grid-ok` exempts thumbnail grids).
- Check fixes (red at the branch start): check:creator-cast, check-map builder (in-page note / publish dialogs),
  check:creator-cover (text-language switch).
- Step 1 ✓ 39439d4 – folders `page/ status/ fields/ media/ chapter/ slice/ text/ items/ events/ toon/` (4.1, names kept:
  meta-page stays meta-page until step 11); `panel-width` → `lib/editor/`; all imports `$lib/…`.
- Step 3 ✓ (the commit adding this line) – status chips on tools.css `.sc-chip.is-*` (+ `.is-dot` colour square,
  `.is-tag` outlined name, `.is-quiet`): file-status, persona tags, renditions, locale-text source/missing/long, cast
  star; interactive chips `.sc-chip-control` (+ `.has-image`, `.is-warning`, `.is-filled`/`.is-draft`, `.is-error`):
  featuring, related items, credit roles; marked notes (image requests, "will be added") `.sc-mark`. The unused
  `status-badges` component is gone (`statusWord` → `lib/editor/status-word.ts`). Not chips, left alone: the toggle
  buttons named `.chip` (cast personas, map toon filters, role picker) – controls; @scutoons/ui `Badge` stays the shared
  component (admin; no uppercase for ids). `check:tools-css` flags a local `--badge-*` filled rule.

### 4.4 Owner decisions (2026-10-03)
1. **Undo inside a field:** the meta-page behaviour everywhere. With the cursor in a text field (input, textarea,
   contentEditable), Ctrl+Z / Ctrl+Y belong to the browser and undo the typing in that field. Outside a field (after a
   click out or Escape), they undo / redo the last change to the edited file (chapter, scene, toon settings, character …).
   The chapter editor stops undoing the file from inside a field.
2. **Field-grid spacing follows the hierarchy level**, on the token scale only (row gap / column gap):
   - page form (toon-editor, character-detail): `--space-075` / `--space-1`
   - inspector panel (dialog-inspector, group-panel): `--space-05` / `--space-075`
   - dense rows (vn-events, slice-audio): `--space-025` / `--space-05`
   One `.sc-fields` with a density attribute (e.g. `data-density="page|panel|dense"`) + `data-cols`; no local gaps.
3. **Publish wording depends on context:** one shared confirm dialog (same structure and buttons everywhere), whose text
   names exactly what is published and in which language, e.g. "Publish chapter 2 (de) – readers see it immediately." /
   "Publish the toon settings for catpope (en)." No browser `confirm()`.

Result: chapter-editor ~1400 → ~300 lines, largest creator component ~400, ~600 lines of duplicated CSS / logic gone, every
recurring control in one named place.
