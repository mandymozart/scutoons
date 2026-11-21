Status: inventory 2026-11-20 for epic #626 (#654) – the map the rollout tickets work from; refresh a row when its area moves.

# Creator redesign – inventory

Target (RULES H9): one editor shell – pill toolbar, opened items as tabs Scene · Asset · Show, the stage full window,
floating panels defined once (Library, Inspector, Kanon, Problems, History, Flow, Reader …); desktop only, ≥ 744 CSS px.
Drafts: `workspace.html` here (approved 2026-11-20). `c/` = `creator/src/lib/components/`.

Facts that correct the epic's first guesses: there are no `items/` or `rigs/` routes (items: `catalog/items`; rigs only
at `/edit/rig:character.<id>`); `catalog/` only redirects; `scenes/` is read-only (holders, camera, shots, events are edited
in the storyboard and the chapter's VN events today).

## 1. Routes → new home

| Route | What | Main code | ~Lines | New home | Ticket |
|---|---|---|---|---|---|
| `(app)/+layout` | AppShell, AreaMenu, crumbs / PathMenu, ToonSwitch / ToonTabs, palette, PWA bars, text-language switch | page/area-menu, path-menu, toon/* | 1300 | the new shell (tabs replace crumbs, ToonTabs, AreaMenu) | shell |
| `(app)/`, `toons/` | dashboard (Continue, Toons, Not live yet, Waiting for you), toon list | page/home-bar, toon-list, review-list | 580 | Home | home |
| `[toonId]/` | seasons / chapters tree, publish tree, Flow dock | flow/flow-dock, status/* | 505 | Home (toon) + Library tree | home |
| `[toonId]/season/[n]`, `/toon`, `/credits` | season meta, toon settings, credits | toon/season-editor, toon-editor, credits/* | 1750 | Inspector of the toon / season node | home |
| ~~`[toonId]/cast`~~ | gone (#104): membership + role in the character's Links, the Library's cast facet; the season sheet's side with #665 | – | – | – | character |
| `characters/` (`?tab=wanted`) | registry cards; `character:<id>` = the character workspace (#104): picture as background, Tree, Inspector · Kanon, Moods · Vocabulary panels; wanted images | characters/* | 4500 | wanted → the central Wanted panel (#696) | character |
| `catalog/[kind]` | items, recipes, albums, products, editions, home-scenes, airlift-carriers | catalog/catalog-editor + `@scutoons/catalog` | 450+ | Asset: item; the other kinds Library + Inspector | item |
| `catalog/items/[id]/collage` | an item's score / collage stage | collage/* | 1700 | Asset: item (its stage) | item |
| `edit/[key]` (`?w=sound`) | Sound workspace: rig / module designer + deck stage | sound/*, rigs/*, `@scutoons/rig-designer` | 1150+ | Asset: rig · Asset: module (`/edit/<kind>:<id>` is already a tab address) | rig |
| `[toonId]/season/[n]/cover` | season cover with lettering | cover/* | 840 | Asset: cover (new kind) | home |
| `*/script`, `kanon/[subject]` | script / Kanon text, one editor, four routes | script/* | 910 | the Kanon / Text panel | text |
| `[toonId]/chapter/[n]` | chapter editor: slices, strip canvas, inspector, VN events, docks | chapter/chapter-editor (1852), slice/*, events/*, text/* | ~11k | Scene (the chapter as stage with events) – the biggest piece | chapter |
| `[toonId]/chapter/[n]/storyboard` | storyboard studio (§4) | storyboard/*, lib/editor/storyboard-*, lib/storyboard | ~11.8k | Scene: camera, shots | storyboard |
| `map/` | world map builder | map/map-builder (1389), map-canvas (1101) | 3650 | Scene: map; place props → Inspector | map |
| `scenes/[[id]]` | deck scenes list + read-only detail | scenes/* | 445 | the list → Library; a scene opens as a Scene tab | shell |
| `flow/` | universe flow graph | flow/* | 1475 | shared panel: Flow | shell |
| `guide/` | style guide + script rules | page/style-guide | 160 | Help panel | shell |
| `notifications/` | PWA push opt-in | `@scutoons/pwa` | 25 | Settings, or gone with the PWA | #pwa decide |
| `docs/` | public tutorials | lib/docs/* | 380 | stays (docs site) | – |
| `h/[code]` | the storyboard's handheld camera page (a phone, QR, no sign-in) | lib/handheld/*, ui HandheldStage | 250 | **stays a phone page** – a companion device, not creator UI | – |

**Edited twice today:** characters (`characters/` and `[toonId]/cast`); the character inspector (mounted by two layouts);
items (definition, collage stage, and the placement in the slice inspector); script text (one editor, four routes);
Flow (page + dock); moods (vocabulary + the sound workspace's mood chip – not a real duplicate).

**Already shared panels** (good first floating panels): Problems, History, Flow, Reader – mounted by the shell and taken
page to page. `meta-page` is the common frame of 11 editors (Save / Publish / History / Problems / PhoneBar): the main
thing to re-host.

## 2. shared/ui

| Piece | Users (cr/ad/em/vw) | Call |
|---|---|---|
| app-shell + shell-bottom, dock-content, dock-slot | 5/1/1/0 | keep for admin + embassy (embassy keeps its phone branch); the creator leaves it |
| dock.svelte.ts, dock-hold, dock-fold, dock-region, maximize-button, rail, fold-button, panel-width | creator only | replace by the floating-panel shell; divider / resizable may be reused |
| action-bar | 14/32/0/0 | admin keeps; the creator takes the pill toolbar |
| **phone-bar, bar-tabs** | creator only | **delete** (desktop bars render through phone-bar today – rebuild them first) |
| breadcrumbs | 1/0/0/0 | delete (tabs) |
| sheet | 6/8/0/2 | keep (admin, viewer); the creator's drawers become panels |
| prop-row, prop-select, scrub-*, linked-pair, color / paint fields | creator only | keep: the Inspector's vocabulary, restyle |
| segmented, tabs, badge, menu, section, panel, info-panel, command palette | mixed | keep + restyle; panel / section → floating-panel pieces |
| dialogs, toasts, icons, sortable, grid-keys, selection, tooltips, timeline, qr-code, handheld-stage | mixed | keep |
| data-table, pager, search-field, chart, bulk-*, split-view … | admin | untouched |
| field-reset, unit-prefs | no app users | delete |

## 3. Phone code to delete (#659)

Counts: creator 33 files with `@media`, 10 with `pointer: coarse` / `hover: none`; shared/ui 23 files (the 767/768
breakpoint in app-shell, action-bar, phone-bar, shell-bottom, divider, dock-fold, fold-button, maximize-button,
palette-trigger, tabs, toast-stack) – admin and embassy still use those, embassy's phone paths stay.

Creator phone-only: `lib/editor/sheet-drag.svelte.ts` (+ the copies in chapter-editor and map-builder), `lib/editor/tap-click.ts`,
`c/slice/phone-kinds.svelte`, `c/slice/long-press-menu.svelte`, clip-menu's long-press path, the PhoneBar / BarTabs blocks
(chapter-editor 114 refs, meta-page, map-builder, characters-editor, scene-browser, `[toonId]`, home-bar), every
`max-width: 767px` block. Review the 800–1279 px stacking (sound-workspace 900, cover 900, script 900, credits 1100,
map 1100, cast 800, moods 1023, chapter / score-stage 1279) against the 744 minimum.

Keep despite the name: `lib/editor/screens.ts` (emulates reader screens), the handheld camera (`lib/handheld/*`,
storyboard-handheld*, `h/[code]`).

## 4. The storyboard

`storyboard:<toon>:<chapter>`: shot list, main view (shot canvas / three.js 3D / Variants compare), six step panels
(Script, Shot / Figures, Rhythm, Generate, Handheld, Output), the timeline. Home: the Scene view – its shots and camera
are the Scene's; Generate, Variants, Output, Rhythm become shared panels; the timeline is a candidate shared with Show.

## 5. Legacy

Unimported: `c/characters/character-picker.svelte`, `c/page/up-link.svelte`, `c/status/live-chip.svelte`. Toon navigation
three times (ToonTabs, AreaMenu, PathMenu). `catalog/+page.ts` only redirects. Sound workspace still stacks below 900 px.
PWA (`lib/pwa.svelte.ts`, install / update bars, service worker, notifications route): keep or drop is a decision.
