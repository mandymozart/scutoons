# BACKLOG — everything open after the beta (one list)

One list for the open work (owner 2026-10-06: "we need to merge cutover post items and minor bugs"): the former
CUTOVER.md Phase 10 ("after the beta", items keep their numbers 10.x so commits and docs that cite "CUTOVER 10.x" still
find them here) and the known bugs that were in MEMORY.md. CUTOVER.md stays the runbook of the cutover itself.

New entries: the next free number (10.59, …), owner + date, then **O –** for steps the owner runs on production. Tick
an item ([x]) when it is done and move it to "Done" at the bottom.

## Day run 2026-10-07 (owner away: "go automatically") – the plan, all decided with the owner

Rules as for night runs: small commits on `main`, **never push**, the dev DB is free, `npm run check` + the step's
browser checks before each commit, each agent commits only its own paths. Two lanes with separate files:
**lane V** (viewer / server / content schema) and **lane C** (creator / admin / shared ui). Decisions below are the
owner's (2026-10-07, drafts reviewed in this session's browser). Results go into "Day run result" at the end of this
section.

Lane V, in order: 10.42 → 10.48 (E.5) → 10.41. Lane C, in order: 10.51 → 10.52 → 10.53 + 10.43 → 10.40 → 10.39 → 10.44
(E.1) → 10.45 (E.2) → 10.47 (E.4) → 10.46 (E.3 b/c) → 10.49 (E.6, after lane V) → 10.50 (mobile). Added later: 10.54 (layers 2) before 10.47.

- [x] 10.41 **Season URLs + season labels** – `/<toon>/chapter/N` stays THE address forever (printed QR codes);
      `/<toon>/season/S/chapter/K` and `/<toon>/season/S` (toon page at that season) redirect to it, resolved from the
      season's `sequence`. Chapter 0 = "Prologue" (name stays), not counted: `season/1/chapter/2` = `chapter/2`,
      `season/2/chapter/1` = `chapter/6` when season 1 holds 0–5. A later season's trailer is just its first chapter.
      Labels: "Season 2 · Chapter 1" (one-season toons: "Chapter N"; no #6 shown) in the tab title (new, the chapter
      page has none), the coming-soon / gate heading, the next bar / box choice, the toon page's chapter list grouped
      by season. **Positions frozen (option A):** the creator's validator errors when a listed (not draft) chapter is
      moved / removed / something inserted before it in its season; appending is always fine. Optional: an API
      lookup season+position → number.
- [x] 10.42 **A season's `sequence` holds chapters only** – data: every entry is `"type":"chapter"` (dev dump). A
      migration `2026-10-07-sequence-no-type.mysql.sql` (SELECT first: entries whose type isn't chapter → 0; then
      remove `type` from every sequence entry of `season` rows in entity_versions / content_versions as present),
      `SequenceItemDoc` without `type`, no type checks left (Library.php :712 / :756, template.ts :77, the creator's
      sequence editor, make-sandbox.ts). Songs in credits + scripts stay (owner). **O –** run the migration with the
      deploy (the SELECTs in its header first; dev DB: 49 versions, 374 entries, all chapters – applied 2026-10-07).
- [x] 10.43 **Slice editor: one layer stack** – text boxes AND items in ONE drag-reorderable list ("Layers" tab
      replaces "Text boxes" + "Items"; Audio / Events / Slice stay), the order = what's on top (top row = front), stored
      (analyse first; a stored order both the creator canvas and the reader's slice honour; absent = today's order).
      Rows single-line: grip ⋮⋮, small icon preview (item image / text-box glyph), name, a feather icon for "pick up"
      (`download`); no `#n`, no hint text. Uses 10.53. Draft: this session (toptoy ch1 blotter-deal slice 1).
- [x] 10.54 **Layers 2: sound areas and area events join the stack** (owner 2026-10-07, corrected twice – final:
      events started by click / hover / tap that are NOT "Whole slice", and samples with a trigger area, depend on the
      hierarchy) – each such object is a row of the one Layers list (10.43) and takes the same `layer` order: higher =
      in front = gets the tap / hover first, in the creator's canvas AND the reader's hit testing. They STAY listed in
      their own tabs too (Events / Audio – edited there as today); the Layers list shows them as well so their place in
      the hierarchy is visible and reorderable (owner: "that is the important thing"). Whole-slice events and other
      starts (in view, …) and samples without an area: only their own tab. Row icon: the sound / event feather icon + name / type. Absent `layer` = today's order (analyse the
      reader's current stacking of event areas vs sample areas vs items vs text boxes first; keep it when unlayered).
- [x] 10.44 **E.1 notifications** – as UI-CONCEPTS E.1; owner: toasts bottom-right (the shared stack), the
      language status as a chip next to Publish (phone: `de ▾`), only conflict / autosave / converted stay as bars.
- [x] 10.45 **E.2 crumbs** – as E.2; owner: the "…" menu (not a scroll); the logo is the root crumb, "CREATOR" goes;
      the real season. On phones the crumbs get their own row (where the file-status row was).
- [x] 10.46 **E.3 b/c** – the dock keeps its panel across navigation; Ask reads the crumbs.
- [x] 10.47 **E.4 slice editor room** – owner-approved draft: the toolbar = two **square** vertical groups left of the
      canvas, **View** (grid/snap with its size – hold / chevron for 5/10/15/30; zoom) and **Content** (text, item,
      sound, event); zoom hides behind its toolbar icon and opens the Flow's round buttons (zoom out, %, zoom in, fit,
      **1:1**); Ctrl/Cmd + wheel zooms; 50–300 %, remembered per device; the canvas fills (cap 2x).
- [x] 10.48 **E.5 end actions in the box** (viewer) – all end actions become choices of the VN message box, the
      same style as story choices: next chapter (with the 10 s countdown line + Cancel), replay, redeem a code, where to
      get a code, download the season; coming soon: the release date as the box text + "To the overview". The separate
      next bar goes (next-host's reserve stays). Checks: check-next-bar, chain-vn.
- [x] 10.49 **E.6** – strip preview for vertical groups; the viewer as a full dock panel (iframe of the draft preview
      at phone width, reload on each saved draft, postMessage "show group / slice", the viewer accepts it only from
      the creator's origin).
- [x] 10.50 **Creator mobile make-over** – phone walk (375 px) found: (a) the drawer's fold leaves a blank ~250 px
      panel over the page; (b) the drawer stays open after picking a group / slice; (c) ‹ slice ›, undo / redo,
      Problems, History hidden in More (+ a half black bar at More's top); (d) canvas ~150 px, handles ~8 px; (e) the
      toolbar duplicates the inspector tabs, the tabs wrap 2+2+1; (f) header: 6 controls, square + round mixed; (g)
      crumb only "Chapter 1"; (h) toon / chapter pages open with settings first. **Approved phone draft:** header ☰ ·
      logo · Ask · search (language switch + Flow at the top of the ☰ drawer); crumbs in their own row; canvas full
      width with the square tool groups at its left; inspector below as a sheet (grab handle, icon + count tabs); a
      slice bar above the action bar (‹ Slice n/m ›, undo, redo, ⚠ problems); action bar More · Save · `de ▾` ·
      Publish. Copy / paste on phones via a menu (10.39). Then audit every creator / admin / embassy page at 375 and
      768 (UI-CONCEPTS E.9) and apply the same patterns; touch targets ≥ 2.75rem.
- [x] 10.51 **Creator: names follow the text language** (owner screenshot: EN showed "Katzenpapst", "Krähen" in
      Featuring) – every shown name of a character / persona / item / recipe / album follows the header's text-language
      switch, fallback the source language. Today mixed: source language (featuring-field, cast-editor,
      character-detail, the chapter editor's cast notice), UI language (`itemName`, characters-editor, flow-pictures),
      text language (character refs, picker). One shared place for "the display locale".
- [x] 10.52 **UI rule: round vs square** (owner) – a standalone icon-only button is round (`sc-round`); grouped
      controls (segmented, toolbars, language switchers in every app) are square. Into HANDOVER + DESIGN.md; fix the
      outliers met on the way (the creator header's square search / language next to round buttons).
- [x] 10.53 **One reorderable-list mechanism** – a shared `Sortable` in @scutoons/ui: grip ⋮⋮, the lifted row,
      a primary-colour drop line; touch = press-and-hold on the grip (the list still scrolls); keyboard Alt+↑/↓; one
      reorder = one undo step. Replaces every chevron (`ReorderButtons`: supporters, credits sections / entries
      (authors), featuring, related items, list-field, moods, cover inspector, map routes; admin catalog editor / rows)
      and the four own drag codes (chapter tree, slice list, VN events, cast editor). First user: 10.43.
- [x] 10.57 **Several dock panels side by side + the footer while maximized** (owner 2026-10-07: "why is it either Flow
      or phone preview? Ask can also be active at the same time – that's what the panels refactor was for … jumping
      through the Flow would also directly show the final result in the preview"; "when in maximize mode the footer
      disappears – figure out what makes sense"; later the same day: the main may fold too, maximize = fold everything
      else, every rail's button at the height of Ask's header buttons). Rules: DOCKING.md "Several panels", "The main
      folds – by hand only", "Running out of room", "Maximize = fold everything else".
- **Content (owner, production):** catpope chapter 1's last caption says chapter 2 comes "1. November 2025", the
  release is 15 October 2026.
- 10.40: owner confirmed the multi-select look (all picked rows cyan when live); 10.39 on phones: yes, via a menu.

### Day run result

**Status (finished):** every day-run item is done – 10.39–10.54 and 10.57 (+ the content-test flake fix, the owed checks
after Docker: check:events 29/29 incl. a real reader-name bug, PHPUnit 299/299, check-next-bar 98/98 incl. the desktop
video autoplay regression from M.2). All on `main`, **not pushed**. **O –** with the deploy run
`2026-10-07-sequence-no-type.mysql.sql` (SELECT first; HANDOVER loose ends). Still owed: the admin's item-limit fields
checked with the admin running. Found, not changed: the Reader panel's frame takes the keyboard focus when it loads
(creator shortcuts go to the viewer until the page is clicked). UI-CONCEPTS E.9's 9 phone findings: deferred (owner).
**Decisions waiting** (owner answers 2026-10-07) – still open:
- 10.52 / E.4: the slice editor's zoom flyout – one SQUARE group like the Flow's zoom (a toolbar), or round as the E.4
  draft showed?
- 10.49: a `frame-ancestors` CSP for the viewer (framing limited to the creator; needs the static hosts' .htaccess –
  DOCKING.md "The Reader panel").

Answered: cross-toon pastes write `../<toon>/…` – **keep `../`** (10.39 text + ASSETS.md updated); the end-action box on
desktop – **good as it is** (full width, no arrow back); toasts – **good** (bottom-left); UI-CONCEPTS E.9's phone
findings – **ignore for now** (marked deferred there); the crumb "scene-13" on group key "scene-14" – **data, not a
bug**: catpope ch2's scene document `scene:catpope:2:scene-14` has carried `label: "scene-13"` since 2026-10-03 (renamed,
EXAMPLES.md), the crumb shows a group's label and the URL its key – the same group; `check:creator-chapter` – **runs
again**: catpope ch1 was published (no pending draft), 87/87 after two fixes in the check (catpope ch1's content has
an error – an audio sample with an empty id on group 1 slice 2 – so Publish asks "publish anyway?" first: the check
now prints the errors and goes on to the naming confirm; "+ event" counts one line more instead of assuming an empty
slice).


- 10.57 done (lane C): the dock holds several panels side by side (`shellDock.panels`, one per kind, fixed places detail · History · Problems · Flow · Reader · Info, recency `used`; one `useDock` per panel – chapter editor Flow / Reader / Problems / History, MetaPage Flow / Problems / History; Info no longer gives way); each panel its own header, `DockFold` (via `DockSlot`), ✕ and Divider (measured from its own right edge); out of room → narrower together to 12rem, then the least recently used folds to its rail (a squeezed rail's click brings it back); the main folds by hand only ("Fold the page" in the leftmost panel, Ctrl+Shift+\, its rail with the deepest crumb; a Problems "go to" unfolds it); maximize = everything else on its rail (sidebar, main, other panels, Ask), railed regions stay mounted (laid out at their last width, hidden – the Reader keeps its frame); **footer: the ActionBar stays** while maximized / the main folded – it vanished only because the page renders it inside the hidden main; it is the page's chrome and acts on the document the maximized panel drives. Rails' buttons at Ask's header buttons (Rail padding `--space-05`). Checks: svelte-check ui / creator / admin / embassy 0, tools-css ok, creator-dock 100/100, tools-dock 44/44, creator-flow-dock 19/19, creator-reader 36/36 (+ Flow · Reader · Ask at 1440 / 1920, a Flow box moves the Reader, squeeze + "Fold the page", maximize); shots `dock2-*`. Found: the Reader's frame takes the keyboard focus when it loads (creator shortcuts then go to the viewer) – not changed.
- 10.41 done (lane V): one helper `@scutoons/content` seasons.ts (`countedChapters` / `chapterAt` / `chapterPlace` / `chapterLabel`: position K = the K-th sequence entry with number ≠ 0, every stored entry counted; `frozenSequenceProblems`). Viewer routes `/<toon>/season/S/chapter/K` → `/<toon>/chapter/N` (query kept) and `/<toon>/season/S` → the toon page (`#season-S` with ≥ 2 seasons), unknown → not found; the toon layout loads every season (`seasons`), a chapter page reads with the season that lists it. Labels (`common.season_chapter` "Staffel 2 · Kapitel 1", one season "Kapitel N" = the position): new tab title "<toon> · <label>: <title>", coming-soon heading, the box's end actions, start button "S2 K1", the list's "#K"; the toon page's list grouped by season under its heading. Positions frozen: the season validator errors (Problems panel) when a chapter listed in the last PUBLISHED version moved / was removed / got something inserted before it – the creator's ContentDoc loads the published sequence (`publishedSequences`), meta-page passes it; 6 tests (seasons.test.ts). No stored format change. check-next-bar updated to today's content (ch2 coming soon → chapter 1's end; countdown via a rewritten toon answer; ch2 groups / sandbox via the test user's draft preview; `devpromo`; tab title + season addresses): 390-de 32/32; known left: 1440 guest autoplay of the prologue fails also without this change (3 cases per locale), 390-en cancel/countdown flaky once (passed on rerun). npm run check green, chain-vn 24/24, a fake 2-season toon checked headless (headings, #1/#2, labels, redirects).
- 10.51 done (lane C): `textLanguage.names` (creator text-language.svelte.ts) = the header switch as it stands on the page (the layout publishes the page's languages); `shownCharacterName` / `contentName` / `itemName` use it with the fallbacks source → en / de / id – featuring, cast notice, cast, characters page + detail, picker, ref fields, related items, item rows, the Flow. Universe pages use the switch over all toons' languages (source `de`); a page whose toon is still loading the remembered choice, else the source. Toon titles and map place names still follow the UI language. check:creator-cast (sandbox) 18/18, creator-dock 82/82, check:creator-chapter refused (catpope ch1 has a draft).
- 10.52 done (lane C): the rule in HANDOVER "UI rules" + DESIGN.md "Round vs square". Fixed: the palette trigger (compact = creator / admin / embassy, and every one under 768px) is a round search icon, "Ctrl K" moved into its tooltip; the Flow's zoom / fit = one square `sc-seg` (a toolbar, were 3 round); the viewer menu's languages = one square joined frame, the chosen one inverted (were circles). Already right: the creator's text-language switch, the shop's languages, the ActionBar's undo / redo. Left open: the frameless `CloseButton` (HANDOVER open decision), slice toolbar (10.47), reorder chevrons (10.53). check:tools-css ok, svelte-check creator / admin / embassy / viewer 0 errors, creator-dock 82/82, check:tools-ui only the known slot / text-wrap / batches findings; creator-cover / creator-script checks now answer the note dialog with `button.primary`.
- 10.48 done (lane V): the end actions are choices of the VN message box (`vn.setEnd(snippet)`, message.svelte renders it after the story's choices; the box is on every chapter page now, VN or not) – next chapter / next group / authored end (+ the 10 s countdown as an ink line along the choice's bottom, a small "Abbrechen" under it), ↺ replay, sign in, redeem, where to get a code, the music download; coming soon / season end / members: their text is the box's text. The fixed bar is gone; `.next-host` reserves the box's height + 2rem; the characters stand on the box (ground from the box's height, `endHeight` gone). Checks: viewer svelte-check 0 errors, chain-vn 24 / 24 (new: the character stands on the end box as on a message), check-next-bar 390-de the box cases pass (its other failures are content / stale: catpope ch2 is now coming soon, `next_bar.replay_video` no longer exists, the sandbox toon isn't in the library), events 7 failures – the same 7 without this change.
- 10.42 done (lane V): `SequenceItem` / `SequenceItemDoc` / `LoadedSequenceItem` without `type` (schemas regenerated), no type checks left (Library, FlowIndex, template, validate – a leftover `type` is an error now –, the creator's toon page + visibility, the viewer's sequence navigation, make-sandbox, fixtures); the library's chapter summary has no `type` either. Migration `2026-10-07-sequence-no-type.mysql.sql` run on the dev DB (49 versions); **O –** run it with the deploy (HANDOVER loose ends). Checks: npm run check, PHPUnit 296 / 296, phpstan clean.
- 10.53 done (lane C): `sortable` (action) + `SortGrip` in @scutoons/ui (DESIGN.md "Reorderable lists"; DataTable `onmove`): grip ⋮⋮, lifted row, primary drop line, touch hold 300 ms, auto-scroll, Esc, Alt+↑/↓ + aria-live, `onmove(from, to)` = one edit. Migrated: cast, credits sections + entries, supporters, featuring, related items, list-field, moods, cover lines, map waypoints, chapter tree groups, slice list (file drops kept), VN events (selection moves together, `moveMany(list, picked, from, to)`), admin catalog order + catalog rows + homepage featured; `ReorderButtons` (ui + creator) removed. check:creator-cast 20/20 (drags the grip), creator-dock 82/82, tools-dock 38/38, creator-reach 21/21, creator-cover 43/43, svelte-check ui / creator / admin 0; check:creator-chapter has a new reorder part + SCUTOONS_CHAPTER_TOON / SCUTOONS_CHAPTER but refused (catpope ch1 draft; no other chapter with inline slices is clean) – the same steps ran headless without saving on catpope ch1/ch2 + sandbox ch2 VN (all ok).
- 10.43 done (lane C): stored as `layer?: number` on each `dialog[]` / `items[]` entry (higher = on top; `@scutoons/content` layers.ts `sliceLayers` / `setSliceLayers`, validation: whole number ≥ 0, schemas regenerated); none = the old order (items under text boxes, an entry without `layer` above the numbered ones) – no migration (dev DB: no `layer` key anywhere), arrays + ids untouched (showTextBox refs, pickup refs). Reader: one `SliceLayers` (dialogs + `SliceItem`s in one container, `SliceItems` gone) in the viewer's `Slice` and the creator canvas (its boxes take the pointer in the same order). Creator: one "Layers" tab (text + item tools open it), rows grip · icon / T tile · name · `download` for pickups, Sortable + Alt+↑/↓ = one `doc.edit`. Docs: ITEMS.md "Placed items", DESIGN.md Part 2. Checks: npm run check, PHPUnit 296/296, creator-dock 82/82, chain-vn 24/24, events the known 7, tools-css ok; check:creator-chapter (new layers part) refused (catpope / toptoy chapters have drafts) – the steps ran headless without saving on toptoy ch1 blotter-deal slice 1 (all ok).
- 10.44 done (lane C, E.1): transient info = toasts bottom-right above the ActionBar (creator `lib/toasts.ts` + the layout's shared ToastStack; `DocSession.notify`: Saved / Published / Nothing to save · publish / review submitted, errors stay; event ids assigned 8 s; "added to the cast" with **Undo** – the shared toast's new `action: { label, run }`); unsaved = the Save button's dot (bar gone; phones "Save"); the not-live languages = one chip next to Publish (`status/live-chip.svelte`, "de en · draft ▾", phones "de en ▾", opens the file-status list upwards) on the chapter editor and every meta page – the file-status row is gone (the script page keeps its rules / cursor line); only conflict / autosave / converted stay as bars (DocNotices). DESIGN.md "Notifications". Checks: svelte-check creator / admin / embassy / ui 0, tools-css ok, creator-dock 82/82, tools-dock 38/38, creator-cast (sandbox) 20/20, creator-script 28/28, creator-cover 43/43, creator-expr 36/36 (the dirty / notice probes now read the Save dot / the toasts); creator-chapter (new E.1 part + a Saved-toast check) refused (catpope / toptoy / sandbox chapters have drafts) – its steps ran headless without saving on catpope ch1: all E.1 ok (one known content miss: the layers part needs a 2nd box on that slice). M.5 + M.13 ticked.
- 10.40 done (lane C, E.8 steps 1–6): tokens `--select-live-*`; tools.css `.sc-selected` + `[data-sc-region][data-live] .sc-selected` = the only selection CSS (inverted; live = cyan tint, 2px inset primary frame, #005a70; table rows framed by their cells); `@scutoons/ui` `region` action + `liveRegion` store (`initial` / `modal`, innermost wins). Regions: chapter editor tree / slice list / slice canvas + inspector (one) / VN events, characters, people, map (nav + map), cover (canvas + Lines), every DataTable; modal: Sheet, palette, comboboxes, expression builder. Keys act on the live region only – **fixed: Delete / arrows in the slice list or the tree no longer remove / nudge the canvas box**; the canvas shows handles + tag only while live. Hover never inverts (menu, people rows, map nav, `.sc-round`, palette trigger); focus = 3px ring everywhere (wins over `all: unset`, toggle 3px); the cover box: focus ≠ selection (Enter / Space select); the batches' running row = cyan tint. Nav "you are here" unchanged (inverted). Checks: svelte-check ui / creator / admin / embassy 0, tools-css ok, creator-dock 86/86 (+ live region), tools-dock 40/40 (+ admin live row, embassy ≤ 1 live), creator-cast (sandbox) 20/20, creator-cover 43/43, tools-ui only the known findings; creator-chapter (new selection part) refused (catpope / toptoy drafts, no clean chapter with ≥ 2 inline slices) – the steps ran headless without saving on catpope ch1 slice 3 (28/28: canvas / list / tree live, Delete + arrows + Esc only on the live canvas, multi-select all cyan, 1440 + 390); shots `sel-*`.
- 10.45 done (lane C, E.2 + M.14): `Breadcrumbs` in @scutoons/ui (`items` {href, label}[], `label`, `moreLabel`, `root` snippet = the first item's content, `keep` = 2, `testid`): every crumb its full text; too wide → the crumbs after the root collapse from the left into one square "…" button with a menu (Enter / Space / ↓ open, ↑ ↓ Home End, Esc back to "…", Tab / click outside close); even the kept two too wide (a long group name on a tablet) → the trail wraps, nothing cut. Measured by a ResizeObserver on the nav + an invisible copy. The creator's trail = SCUTOONS (logo, root) › Toon › Season › Chapter › Group › Slice, "CREATOR" gone; the season = the season whose sequence lists the chapter (`CreatorNav.loadSeasons` / `chapterSeason`, @scutoons/content `chapterPlace`; read again on each chapter opened; none lists it → no season crumb). Under 768px (where the sidebar is the ☰ drawer): header ☰ · logo · Ask · search, the crumbs in their own row (without the root), the text language + the page's Flow at the top of the drawer. The account email shows from 1600px (the full trail needs the room at 1440). Admin / embassy have no crumbs – nothing to adopt. Checks: svelte-check creator / ui 0, check:tools-css ok, creator-dock 82/82, tools-dock 38/38, creator-reach 21/21, creator-cover 43/43, check:tools-ui only the known slot / "New character" / viewer findings; a headless E.2 run (trail, collapse, menu keys, navigation, Alt+↑, phone row, drawer) all ok; shots `e2-*`.
- 10.46 done (lane C, E.3 b/c + M.6): dock panels across pages – `useDock` panels with `kind` + `content` (component + props, `@scutoons/ui` `DockContent`) are mounted by the shell; the page leaving HOLDS the panel, the next page docking the same kind takes the instance over (no remount, fold kept, new props), nobody → it closes after that navigation (`DockHold` in each creator route's `{#await languages}` keeps it while the page loads); pages open their own when `shellDock.has(kind)`. Kinds: `flow` – ONE creator `FlowDock` (header + toon / season / chapter body) toon → season → chapter → chapter (a box, the sidebar, a crumb), each new level rebuilt at the kept zoom (`flow-controls` first view at the kept zoom), group → group keeps graph + view; the boxes `goto` without `?flow=1` (deep link stays); `problems` (`ProblemsDock`, chapter editor + meta pages); `info` (every ActionBar, admin / embassy too). History + the admin list detail stay page-bound; leaving a page ends a maximize. Ask (c): already read `nav.crumbs` (the Breadcrumbs' source, real season) – now also sends the deepest crumb's address as the agent's `ref`. Docs: DOCKING.md "Panels across pages", UI-CONCEPTS E.3 status, DESIGN.md Flow panel. Checks: svelte-check creator / ui / admin / embassy 0, tools-css ok, creator-dock 86/86, tools-dock 40/40, creator-reach 21/21, new `check:creator-flow-dock` 19/19 (the same FlowDock element + mount counter toon → season → chapter → chapter, zoom 0.69 kept, group → group the same graph + zoom, a box navigates, cast closes the held Flow, Problems chapter → season, admin Info codes → assets, Ask's context + ref). **Owner: try toon → season → chapter with the Flow open.**
- 10.54 done (lane C): analysed first – the reader drew sample areas under the stack (z 1), items + text boxes in it, event areas above (z 2), but nothing occluded: event / sample areas were pointer-none rect tests on the `.slice` (every area under the pointer ran, an item's tap bubbled into the areas below). Now `@scutoons/content` layers.ts: `sliceLayers` also answers event areas (click / hover with a rect, the events of one `triggerKey` = one area, `events: [...]`) and sample areas; `layer` on the event entry (`VNSliceEvent.layer`, one number per area) and on `Sample` (validation whole ≥ 0, schemas + server copies regenerated); `hasSliceLayers`. No `layer` = exactly the old order (sample areas, items, text boxes, event areas) AND the old hit testing – no migration (dev DB: no `layer` anywhere). A slice with layers: the viewer draws its areas inside `SliceLayers` (`area` snippet; Click / Hover trigger `own`, SampleTrigger `onplay`) – the browser's hit testing = the stack, only the front-most area / tappable item reacts. Creator: the canvas sorts every box by the stack (pointer order), on the Layers tab the stack's areas take the pointer; Layers rows for areas (Feather zap / music, "joinScene +5 · Tap", "id · Tap"), a click selects on the canvas (the tab stays, `sc-selected`), a square Events / Audio button under the list opens the area's tab; edited in their tabs as before. Docs: ITEMS.md "Placed items" (Layers 2), DESIGN.md Part 2. Checks: npm run check (content 7 layer tests; the flow linear-timing test flaked under load, tokens' qr copy out of date – neither mine), PHPUnit 296/296, creator-dock 86/86, chain-vn 24/24, a headless reader run on a sandbox ch9 draft (removed again): unlayered = every area / item under the pointer (2 3 1 1), layered = only the front-most (2 1 0 1); the creator steps headless without saving on toptoy ch1 blotter-deal slice 1 + an unsaved sample area + text box (14/14: one list of all four kinds, canvas pointer order = the list, Alt+↓/↑ move an area with its 6 events, Events button); check:creator-chapter's layers part updated (areas in the list, canvas pointer order) – refused as before (drafts). Shots `l2-layers-1440`, `l2-layers-390`, `l2-reader-390`.
- 10.39 done (lane C, E.7 steps 2–8; side fix first): **side fix** – the creator ActionBar fits at every width ≥ 375 (the live chip drops its state word below 1600, its dot below 1024; below 1280 "Save", the slice count without its word, the note in More; the note shrinks first) – measured headless 1600 / 1440 / 1280 / 1024 / 768 / 390 / 375 (e506991f + the repair 0cf4594e: a partial staging had misplaced the CSS). **Copy / paste** – one system: `lib/editor/clipboard.svelte.ts` (DOM copy / cut / paste events, JSON with a marker, the LIVE region's handler; Ctrl+Shift+V = settings only; fields / page text keep the browser's keys) + `clip.ts`; handlers: slice canvas + inspector (text box, placed item, sound area, event area), VN events, slices, groups (Shift+V per UI-CONCEPTS E.7's table); fresh ids (boxes, events via assignEventIds, placements random), one doc.edit per paste (a sound into a separate scene's chapter library = a 2nd undo step there), Ctrl+X = cut. Across toons: references `../<toon>/…` (ASSETS.md "Paths in content"; one resolver `toonAsset`, every `${toon}/${path}` join in reader + creator + media slots + rootPath replaced). Placement ids `<item>-<7 random>`. Item limits `placements` / `max_world` (catalog + admin, PUT 422 `item_limit`, `GET /api/catalog/items/<id>/placements`, grant / craft 409 `taken`, creator Problems error + paste refusal). Menu: right-click / long-press = Copy · Cut · Paste · Paste settings; phones: the same in More. SELECTs (dev DB): stored paths outside the toon folder: 272 values, all `../` (catpope's files used by toptoy / yellowmouse / sandbox), 0 leading `/`; pickup keys `picked:<image>#<id>`: 6 placements, 0 collisions. **O –** nothing to run on production (the limits are JSON fields of catalog_items.data; no stored format change). Checks: svelte-check creator / admin / viewer / embassy 0 errors, phpstan clean, unit tests (creator 37 incl. clip.test, content incl. paths.test – its one chapterFlow failure came with lane V's uncommitted 10.54 work), tools-css, creator-dock 86, tools-dock 40, creator-cast 20, chain-vn 24, events the known 7; headless never saving: boxes 16/16, items + event areas 8/8, VN events 12/12, slices + groups across toons 15/15, the bar fit 12/12. **Owed (Docker / MariaDB hung since ~14:00):** PHPUnit (new ItemLimitsTest, 3 tests), the browser checks for steps 6–8, the headless runs of the limits, the new-placement id and the menu (shots `cp-menu-*` / `cp-phone-*` not taken); shots so far `cp-bar-*`, `cp-box-pasted-1440`, `cp-item-pasted-1440`, `cp-vn-pasted-1440`, `cp-paste-across-toons-1440`, `cp-group-pasted-1440`.
- 10.47 done (lane C, E.4 + M.4): the toolbar row is gone – `slice-toolbar.svelte` is a strip LEFT of the canvas, two square vertical groups (`sc-seg is-column`, `Segmented vertical`): **View** = the grid button (pressed = snap, the size in its corner; hold / right-click / the corner number / → open 5 · 10 · 15 · 30, a pick switches snap on), the zoom button, and with `{…}` texts an eye button (its flyout holds the sample-reader select – `canvas-sample` moved into it); **Content** = text · item · sound · event. The zoom flyout = zoom out · % · zoom in · fit · 1:1 as ONE SQUARE group like the Flow's since 10.52 (the draft showed round ones – decision (3) answered with square for consistency; owner: say if round). Zoom = `ChapterView.zoom` (null = fit, else 0.5–3 × the slice's 1x width; `scutoons_creator_zoom`, try/catch); the canvas is laid out at that width (no transform – boxes, handles, snapping stay in %), the area scrolls; fit = the area's height capped at 2x (was 1x), phones the full width; Ctrl/Cmd + wheel over the area zooms around the pointer. Phones: the same strip at the canvas's left, buttons ≥ 2.75rem. check:creator-chapter: snap probes read the corner + a new E.4 part (strip left of the canvas, sizes via corner / right-click, 1:1 = the slice width, zoom in, Ctrl+wheel, fit, Esc); creator-expr-check opens the eye flyout first. Checks: svelte-check creator 0, tools-css ok, creator-dock 86/86, tools-dock 40/40 (admin + embassy dev servers were down – started them), tools-ui: the slice editor clean at 1440 + 390 (the known other findings); check:creator-chapter refused (catpope ch1 draft) – the steps ran headless without saving on toptoy ch1 blotter-deal slice 1 at 1440 + 390: 45/45. Shots `e4-zoom-flyout-1440/390`, `e4-snap-sizes-1440/390`.
- 10.49 done, **removed 2026-10-08** (owner: the Reader in the right dock shows it) (lane C, E.6 + M.9 / M.10): **strip preview for vertical groups** – `strip-preview.svelte` `vertical`: the same screen (phone 390:844 / desktop 16:9), the slices stacked at the reading column's width (phone: the screen's; desktop: 500 / 1440 of it, centred), scrolled inside the screen, no wheel remap; mounted for every vertical group (horizontal swiper groups: none). **The Reader as a dock panel** – kind `reader` (`lib/components/reader/reader-dock.svelte`; the top bar's round `smartphone` button left of the Flow's; `sc-panel-width:reader` 24rem): the viewer's draft preview (`?draft=1`) in a frame, a phone 390 × 844 (or a 1440 × 810 desktop: a square phone / desktop group in its header, `sc-reader-screen`) scaled to the panel; group → group / slice / chapter → chapter (another toon too) keep the instance and the frame – the place goes by `postMessage` (`@scutoons/content` reader-link.ts: "ready" viewer → creator, "show {toon, chapter, group, slice}" creator → viewer, each sent to the other's origin only); reloaded after every stored draft of its toon (`draftSaves`: every `ContentDoc.save` + Ask's saves, debounced) or a language / screen change. The viewer (`lib/creator-link.svelte.ts`, root layout) accepts "show" only framed, from `window.parent`, from `CREATOR_URL`'s origin, with its draft preview on, well formed – then navigates (another chapter; another chain via its `?label=`) and scrolls (a strip's later slice into its travel). Session: the shared SSO cookie (one site) – no token in a URL or a message (DOCKING.md "The Reader panel", PLATFORM.md "Framed apps"). Checks: svelte-check creator 0 / viewer 0 errors, content tests 225 (+3 reader-link), creator 37, viewer 21, tools-css ok, creator-dock 86/86, creator-flow-dock 19/19, tools-dock 40/40 (admin + embassy started for it, stopped after), chain-vn 24/24, new `check:creator-reader` 23/23 (vertical preview phone + desktop; the panel opens and says ready; the frame reads toptoy ch1's draft via the cookie; group → group / slice / chapter → chapter (sandbox 2) the same panel + frame, no reload, the reader scrolled to the slice; a Save on sandbox ch2 reloads it – its version deleted again; a "show" from the CDN's origin ignored, no "ready" to it); check:creator-chapter refused (catpope ch1 draft) – nothing of its steps changed. Shots `e6-reader-toptoy-ch1`, `e6-strip-vertical(-desktop)`. Open: a `frame-ancestors` CSP for the viewer (limit framing to the creator) – a separate decision.
- 10.50 done (lane C, E.9): **bugs** (9f88fba7) – (a) a phone's drawer never shows the rail, its fold closes it; (b) a pick in the drawer (Meta, Audio, an opening group, a slice – `data-drawer-pick`) closes it; (c) the slice bar above the ActionBar on phones (‹ Slice n/m ›, undo, redo, ⚠ count → Problems; round), More = a sheet on the bar with the page dimmed (the "black bar" was the inspector's tab row peeking above the panel), toasts above the slice bar / under the More scrim; (d) touch: grips + frameless icon buttons ≥ `--control-height` wide, canvas handles 1rem + a 2.75rem target, crumbs / logo full height, the list field's + / × square; (e) inspector tabs one row (icon + count), the inspector a sheet below the canvas (45 %, grab handle: drag / tap 45 ↔ 80 % / ↑ ↓); (g) the tree's group name wraps instead of being cut (all widths); (h) toon page: chapters first, languages after (all widths); chapter page on phones: the groups first (meta), a group's slice thumbnails (a tap opens the slice editor). **Audit** (UI-CONCEPTS E.9, 375 × 812 + 768 × 1024, touch, every creator / admin / embassy page): creator 19 findings – 13 fixed, 5 owner decides, 1 known; admin 7 – 5 fixed, 2 owner decides; embassy 4 – 2 fixed, 2 owner decides; fixed: page wider than the screen (admin homepage 532 / 1205, batch 869, creator moods 912, wanted 840, embassy every page 1014 at 768), the catalog's bar end under More, the map's zoom one square group, touch targets (markers, sort links, logos, Toggle) – e1a6dadd, 24841820, 5a00d309. **Owner decides** (E.9): Flow box links on touch, the cast rows on phones (file has your pending edit), standalone text links full height?, slice file names in the drawer, "Save draft" vs "Save" on the toon page, tables as cards on phones?, catalog kind tabs as a select?, embassy week row / CSV file input. Checks: svelte-check creator / admin / embassy / ui 0, tools-css ok, creator-dock 86/86, tools-dock 40/40, creator-flow-dock 19/19, creator-reader 23/23, creator-cast (sandbox) 20/20 (after the bug commit; a later rerun failed only in the dock / maximize / Reader parts while another session's uncommitted dock work – dock-slot.svelte, `fold_page` – was in the tree), check:tools-ui (1440 + 390 touch) only the known slot / "New character" / batches / viewer findings; creator-chapter refused (catpope ch1 draft) – its steps ran headless with every API write blocked: 79 ok, the 3 save steps + the known layers content miss failed. Shots: scratchpad `shots/mobile/` – `m-before-*` / `m-after-*` (chapter editor slice, drawer, More, slice bar, groups, slices at 375), `before-*` / `final-*` every page at 375 + 768.

- content test flake (chapterFlow "lone divert", 1/222): no culprit commit – passes at every commit 0400e515…bb75826e; `assertLinear` (n = 400, ~5 ms) failed on a > 30 ms stall of a busy machine; test/linear.ts now interleaves n / 2n and retries up to 4× (a quadratic still fails every attempt).
- owed checks after the Docker hang (2026-10-07 afternoon): `check:events` 29/29 (4 runs) – its "known 7" were real: a member's askText name (scope reader) went to the DEVICE when the session check took > 3 s (a busy, single-threaded dev API), the account stayed empty, the next page asked again → `readerName.set` waits for the session (05610724); the check now waits for the progress push / the account name instead of fixed sleeps, leaves the page before its restore and sweeps a dead run's leftovers (the hung run's draft 899 of chapter:sandbox:9, its evc-* runs + chapter 9 progress, a cucumber + 2 grant refs – removed by hand first). PHPUnit 299/299 after fixing ItemLimitsTest (it stored the limits as JSON strings – CAST; the code was right), phpstan clean. 10.39 steps 6–8 headless 24/24 (nothing saved; the catalog limit set + removed by SQL): API PUT 422 `item_limit` (nothing stored), at the limit no Problems error / over it one ("Gurke: 7 placements, the catalog allows 6 (this chapter, Chapter 2 (1), Group 14 (1))"), a paste and the item picker refused with a toast, a new placement `camera-<7 random>` (Ctrl+Z removes it), the right-click menu (4 entries, Copy, Esc), phones: the four in More + the long-press menu inside the window (a synthetic contextmenu – CDP touch holds give none); the menu's entries now read from the left. Shots `cp-menu-1440`, `cp-phone-390`, `cp-phone-menu-390`, `cp-limit-paste-1440`, `cp-limit-problems-1440`. Found: `GET /api/items/` is cached 5 min (public, max-age=300) – a limit changed in the admin reaches an open creator only after that (the server's 422 stays the authority). `npm run check` green after regenerating catalog.schema.json (10.39's fields were missing) and syncing the qr tokens copy (10.40's `--select-live-*`) (ba9c943a); chain-vn 24/24 (+ removes a dead run's draft first), creator-dock 86/86, creator-flow-dock 19/19, check-next-bar 98/98 – its failing 1440 prologue autoplay (3 per locale) was a regression of 9cc2aca2 (M.2's `.slice { overflow-x: clip }` cut the full-viewport player to the 500px column, so half of it was never "in view"): a viewport-video slice no longer clips (bc0787b3). Headless profiles: `scripts/dev/chrome-profile.mjs` `tempProfile()` (removed on any exit, stale ones swept) in creator-dock / creator-flow-dock / tools-dock (they never removed theirs) + events; 29 leaked profiles deleted (06d6f702).
- 10.39 decision done (owner 2026-10-07: a unique item whose owner's account is deleted "is back up for grabs. The code still applies."): there is no account deletion feature – an account goes by `DELETE FROM users` (phpMyAdmin); the foreign keys (ON DELETE CASCADE) already removed its inventory, so `max_world` freed up by itself, but `qr.claimed_by` (no FK) kept pointing at the gone account and `qr.email` kept its address → the code was dead for good (409 already_claimed). New trigger `users_release_codes` (BEFORE DELETE ON users): every code the account owned is released (`claimed_by` NULL, `email` '' – as an admin release), its open ownership claims are withdrawn; the migration also repairs accounts deleted earlier. Rule: a code is all-or-nothing with one owner – all its items went with the account, so the next claimer gets the whole code (nothing exists twice); every code is released, not only those with a `max_world` item (the items are gone either way; a dead code kept the deleted email) – owner may say otherwise. Guests' local inventories untouched (import still skips `taken`). ITEMS.md "Item limits", server docs/API.md. PHPUnit `ItemLimitsTest` + 1 test (claim → delete → a 3rd account claims the same code, gets the unique + the non-unique items, others refused as before). Dev DB: migrated (the prod seed has no accounts / codes: 0 rows to repair). **O –** run `2026-10-07-account-deletion-frees-codes.mysql.sql` on production (the two SELECTs first; trigger with delimiter `$$`; any time, no code depends on it – HANDOVER loose ends).

## Night run result (2026-10-07 morning) – read this first

The night's commits are on `main`, **not pushed** (`git log origin/main..main`). `npm run check` green, PHPUnit 295 / 295; the
browser checks run for each step (creator-dock 82, creator-chapter 60, creator-reach 21, events 32, chain-vn 24).

**Done:** 10.38 items are not variables (steps 1–3 + 5: `inventory.<item>`, the warnings, `giveItem` / a choice's `give`,
the migration run on the dev DB, the old variable→item bridge gone) · M.1 intersect area invisible · M.7 "Coming soon"
stamp · M.8 sideways scroll on strips · M.11 / M.12 creator list padding + dividers · M.16 empty video slot portrait ·
10.20 asset picker icon + sizes · 10.11 admin "Server errors" (+ qr writes its own log) · 10.19 `console setup` ·
M.3 part 1 (characters without a box stand lower) · M.6 the Flow's jump · B.1 / B.3 closed (no longer true) ·
10.13 / 10.18 / 10.22 closed (owner / already done) · `agents/UI-CONCEPTS.md` (E.1–E.6 concepts).

**Please look at (couldn't be watched at night – the browser pane was hidden):** M.3 (characters with the box gone, on
the phone), M.6 (the Flow: group → group, slice → slice), M.8 (← / → on a strip, smooth).

**Decisions waiting for you** (details at each item): 10.38 step 4 `takeItem` rules · M.1 hide click / hover areas too?
· M.2 where exactly does the page jump? · M.15 re-export ch1 slices at 3x + the two code proposals · UI-CONCEPTS E.1–E.6
(which first; E.3 b/c, E.5 b need drafts) · the "Morning: for the owner" list below (production steps incl. **10.38's
migration with the next deploy**, 10.1, check-testo, check-next-bar).

## Priorities (owner 2026-10-06 evening: "prioritise now and then set a plan to work through it alone through the night"; – then: "prioritize based on technical implementation and when refactors would be needed")

**Night run 2026-10-06 → 07 (agent alone).** Rules: commit / merge on `main` in small steps, **never push** (a push
deploys – owner: "just merge to main, but do not push to production"), nothing on production; the dev DB is free to
change (owner: "you can do anything to the dev database" – its catpope / sandbox stories are the test case); `npm run
check` + the step's browser checks before each commit. A step that needs an owner decision stops there – the question
goes into "Morning: for the owner". **Bigger UI is drafted, not built** (HANDOVER UI rule: design drafts before bigger
UI) – M.3, M.4, M.5/M.13, M.6/M.14, M.10 get a written concept tonight, the owner picks in the morning.

How the list is ordered – by what the code needs, not by size:
- **A. Finish what is open in the code** – 10.38 (items vs variables) is half built; its data step changes stored
  content, so it goes first while the rest is quiet.
- **B. Quick, independent fixes** – one component each, no shared code touched: M.11, M.12, M.1, M.7, M.16, 10.20.
- **C. Bugs that need finding first** – M.2 (page jumps on effects), M.8 (horizontal scroll input), B.1–B.3 (old,
  re-check), M.15 (slice quality: a report where it is lost, fix only if it's one clear place).
- **D. Self-contained features** – 10.11 (server errors page), `tokens-switch-knob` (1 commit).
- **E. Refactors – concept first, in this order (each one builds on the one before)** – the concepts: **agents/UI-CONCEPTS.md** (2026-10-07):
  1. **M.5 + M.13 One notification model** (shared/ui: transient toasts for info – "Published" fades by itself –, a
     persistent status only where something needs doing, never pushing the layout). Every app's pages use the
     banners today, so this is a shared/ui refactor touching creator, admin, embassy; it frees the room M.4 / M.14 need.
  2. **M.14 Header + breadcrumbs** (AppShell header: the creator title gives way, crumbs SCUTOONS > Toon > Season >
     Chapter > Group > Slice with a "…" for the left ones). Needed before M.6, because the crumbs ARE the hierarchy.
  3. **M.6 Navigation hierarchy** (sidebar + main = the hierarchy; Flow / Ask take their context from it, entry points
     only) – fixes the Flow's jumps as a consequence; builds on the dock model (DOCKING.md) and M.14.
  4. **M.4 Slice editor room** (vertical tool strip, a bigger canvas, zoom like the Flow, Snap as one small button) –
     the slice inspector region (DOCKING step 3) stays; gains from M.5/M.13 (no status bar) and M.14 (smaller header).
  5. **M.3 VN message box + end actions** (reader: avatar / portrait anchored to the page bottom without a box, the
     end actions inside the message box) – the viewer's VN overlay layout; drafts on the real page first.
  6. **M.9** strip preview for vertical groups (after M.4 – the same editor), **M.10** the viewer as a dock panel
     (exploration, after M.6 – it is a third context consumer like Flow / Ask).
- **F. Last** – 10.19 (setup step for a fresh environment; owner: priority last).

**Tonight, in order:** 10.38 step 5 (+ step 4 as a proposal) → M.11 → M.12 → M.1 → M.7 → M.16 → 10.20 → M.2 → M.8 →
B.1–B.3 → M.15 (report) → 10.11 → `tokens-switch-knob` → concepts E.1–E.6 (written into this file) → 10.19.

**Morning: for the owner** (production steps, decisions – the night run doesn't touch them):
- Production steps still open (which of these already ran?): 10.1c `check:fonts` on production, 10.3 steps 6–8,
  10.5 qr deploy with `PROXY_FULL_API=0`, 10.24 code-case migration, 10.32–10.35 migrations, 10.37 `content:check`,
  10.38's migration (when step 5 is in).
- Decisions: 10.12 (link or drop credits / download), 10.14 (claim page – details), 10.21 (picker drafts), 10.25 (map
  places ↔ story), 10.26 (delete-a-chapter rules), 10.38 step 4 (`takeItem` rules), the E. concepts.
- With the owner, not alone: 10.17 atomic deploys (only testable by deploying), 10.23 asset jobs through the API,
  10.2 / 10.4 asset layer, 10.7 / 10.9 cleanups, 10.8 the release.
- 10.1 ("fix the beta's issues on `platform`") – `platform` is abandoned, everything is on `main`: close it?
- `check:testo` (scripts/check-testo.mjs) fails from its first step: its guest half picks up the testo in the sandbox, but
  since 10.30 (the production seed) the sandbox toon is creators-only – a guest gets "Not found". Not from tonight's
  work (the member path works: the testo lands in the inventory, checked 2026-10-07). Make the sandbox public in dev,
  run the guest half on a published toon, or drop it?
- `check-next-bar.mjs` fails 7 of its checks with and without tonight's changes (checked 2026-10-07): it expects chapter 2
  to be readable (it is coming soon in the seed) and a replay label that renders "↺ undefined" – update the check
  (and look at that label).
- `tokens-switch-knob` deleted (2026-10-07): main already had the same tokens-check fix; the branch's commit was d9f838f8.

## Open

- [ ] 10.1 Fix the beta's issues on `platform` → `main` (deploys automatically).
- [ ] 10.1c Fonts (IBM Plex Mono, Bangers + OFL licences) to `scutoons/fonts/` on the CDN. Branch `10.1c-cdn-fonts`
      (2026-10-05): staged in `media/scutoons/fonts/` (18 files); the apps and qr's landing page load `@font-face` from
      `<CDN>/scutoons/fonts/` with their local copy as fallback (ASSETS.md "Fonts").
      Merged into platform 2026-10-05 (owner: "merge it now", after 10.3). Uploaded by hand to
      `scutoons/fonts/` and registered (admin Rescan: 18 new assets) 2026-10-05; cdn.scutoons.com answers 200 + ACAO \*.
      Tick after `check:fonts` on production shows 200s from the CDN.
- [ ] 10.2 Asset layer: merge `assets` (migration `2026-10-02-assets` in phpMyAdmin, `ASSET_STORAGE=bunny` + `BUNNY_*`
      into `production`/targets, `assets:scan`) – ASSETS.md.
- [ ] 10.3 cdn.scutoons.com → bunny _(code ready on branch `10.3-cdn-domain`, 2026-10-05; steps 1–5 done + verified 2026-10-05, GitHub `CDN_PUBLIC_URL` changed; open: 6 deploy, 7 optional, 8 verify, local `.env`)_:
      custom hostname on pull zone 6731366 + CNAME → `scutoons.b-cdn.net` + SSL; then every CDN URL in `deploy/targets.json`
      (api `CDN_URL` + `CDN_PUBLIC_URL`, qr `CDN_URL`, every app's `VITE_CDN_URL`) = `https://cdn.scutoons.com` + redeploy.
      The legacy system is gone, so nothing live uses cdn.scutoons.com: the apps stay on `scutoons.b-cdn.net` until the
      redeploy, which comes only after cdn.scutoons.com is verified on bunny → **no downtime for the apps**. Only old deep
      links to cdn.scutoons.com change source (up to the 2 h KAS TTL, plus a few minutes without a certificate until step 4).
  - **State before (read-only, 2026-10-05):** cdn.scutoons.com = KAS A record `85.13.151.120`, TTL 7200
    (ns5/ns6.kasserver.com), no AAAA; it 404s every new-layout path. `scutoons.b-cdn.net` on samples (trailer
    `catpope/s1/s1-trailer-loop.720p.mp4`, slice `catpope/s1/c2/s1c2-slice-14.webp`,
    `scutoons/models/mobilenet_v2_100_224/model.json`) with Origin `https://scutoons.com` + Range → 206, ACAO `*`,
    content-range sizes = the files. `assets:check-storage` (main checkout, read-only): zone 866 files / 787.3 MB, pull
    zone hostnames `scutoons.b-cdn.net` only; vs the dev copy `media/`: 686 same, 180 only on bunny + 39 different size
    (= production uploads since the init – bunny is ahead), 56 only local (dev-only: catpope/characters SVGs,
    merch/bandcamp, a few items/slices). **No `assets:sync` before the switch:** the switch does not change which bytes
    are served, and a local → bunny `--overwrite` would replace the 39 newer production files. The decisive check is
    production admin → Assets → Missing (nothing new). Old FTP CDN paths that still answer there but not on bunny
    (moved/deleted by the relayout, `logs/cutover/relayout-report.json`): 12 – `catpope/audio/CatPope-Awakening-Music.zip`
    (now `catpope/s1/…`), `catpope/images/s1s{1..6}-thumbnail.jpg`, `catpope/images/collectables/cucumber.png`,
    4 × `catpope/images/vectors/*.svg`; they 404 after the switch (none is a printed URL – PLAN.md RULE; optional: a
    bunny Edge Rule redirect for the album zip if that link was shared).
  - **Owner steps, in this order (the agent verifies each read-only):** 1. Read-only check SQL in phpMyAdmin (absolute media URLs stored in the DB):
    `sql
       SELECT 'entity_versions' t, SUM(json LIKE '%scutoons.b-cdn.net%') bcdn, SUM(json LIKE '%cdn.scutoons.com/%') cdn FROM entity_versions
       UNION ALL SELECT 'content_versions', SUM(json LIKE '%scutoons.b-cdn.net%'), SUM(json LIKE '%cdn.scutoons.com/%') FROM content_versions
       UNION ALL SELECT 'toons', SUM(data LIKE '%scutoons.b-cdn.net%'), SUM(data LIKE '%cdn.scutoons.com/%') FROM toons
       UNION ALL SELECT 'settings', SUM(value LIKE '%scutoons.b-cdn.net%'), SUM(value LIKE '%cdn.scutoons.com/%') FROM settings
       UNION ALL SELECT 'catalog_items', SUM(data LIKE '%scutoons.b-cdn.net%'), SUM(data LIKE '%cdn.scutoons.com/%') FROM catalog_items
       UNION ALL SELECT 'catalog_albums', SUM(data LIKE '%scutoons.b-cdn.net%'), SUM(data LIKE '%cdn.scutoons.com/%') FROM catalog_albums
       UNION ALL SELECT 'catalog_products', SUM(data LIKE '%scutoons.b-cdn.net%'), SUM(data LIKE '%cdn.scutoons.com/%') FROM catalog_products;
       `
    (the catalog lines need `2026-10-04-catalog`; drop them if it has not run.) `bcdn` > 0 → the migration
    `2026-10-05-cdn-domain` (step 7) rewrites them – both names serve the same files, so it is cleanup; nothing breaks
    before. `cdn` > 0 → absolute URLs that today load the OLD FTP CDN: the agent checks those paths on bunny first
    (`SELECT id, entity_key, status FROM entity_versions WHERE json LIKE '%cdn.scutoons.com/%'`). 2. bunny dashboard → Pull Zones → `scutoons` (6731366) → General → Hostnames → add `cdn.scutoons.com` (harmless
    before DNS). "Force SSL" not yet. 3. KAS → Tools → DNS-Einstellungen → `scutoons.com`: the row `cdn` type A `85.13.151.120` → edit to type `CNAME`,
    Prio 0, Data **`scutoons.b-cdn.net.`** (trailing dot). If KAS refuses to edit the subdomain's automatic record:
    delete the `cdn` A row and add a new entry with those values. **Leave the KAS subdomain cdn.scutoons.com and its
    folder `/cdn.scutoons.com/` as they are** (backup until 10.7). Agent: `nslookup -type=CNAME cdn.scutoons.com
ns5.kasserver.com` → `scutoons.b-cdn.net`. 4. bunny → Hostnames → `cdn.scutoons.com` → "Verify & Activate SSL" (free certificate; needs step 3 visible to bunny –
    retry after a few minutes if it fails). Then "Force SSL" on for that hostname. 5. Agent: `https://cdn.scutoons.com/…` on the three samples with Origin + Range → 206, ACAO `*`, same sizes,
    `server: BunnyCDN-…`; `assets:check-storage` lists both hostnames on the pull zone. 6. Merge + deploy (`targets.json` is in every target's `watch`, so all apps rebuild): `git checkout main` ·
    `git merge --ff-only platform` · `git push origin main` · `git checkout platform`. 7. phpMyAdmin: `server/migrations/mysql/2026-10-05-cdn-domain.mysql.sql` (idempotent; a no-op when step 1 found
    nothing), then admin → Assets → Rescan. 8. Agent: the live apps (scutoons.com reader, qr., creator., admin., shop.) load media from `cdn.scutoons.com`
    (network requests, own browser tab); `npm run deploy:status`. 9. Tidy (not read by the deploy): GitHub environment `production` `CDN_PUBLIC_URL` → `https://cdn.scutoons.com` or
    delete it (`targets.json` fixes the value); the local root `.env` `CDN_PUBLIC_URL=https://cdn.scutoons.com` (only
    the local `assets:check-storage` HEAD uses it).
  - **Rollback:** before step 6 – KAS: the `cdn` row back to type A `85.13.151.120` (the old FTP CDN answers again; the
    apps never left `scutoons.b-cdn.net`). After step 6 – revert the `targets.json` commit and redeploy: the apps go back
    to `scutoons.b-cdn.net`, which keeps working whatever the DNS says. The migration needs no rollback (both names serve
    the same zone; reverse = the same REPLACE the other way round).
  - **Done 2026-10-05:** step 1 – no absolute media URL in the DB (all 0; content_versions empty), so step 7 is a no-op.
    Steps 2–4 by the owner; agent check: ns5.kasserver.com + 8.8.8.8 answer CNAME `scutoons.b-cdn.net`; pull zone
    hostnames `scutoons.b-cdn.net, cdn.scutoons.com`; certificate CN cdn.scutoons.com (Let's Encrypt, to 2027-01-03),
    http → 301 https; the three samples through cdn.scutoons.com (bunny edge) → 206, ACAO `*`, sizes = the files.
  - Afterwards: **10.9 is unblocked** (nothing uses the old FTP CDN any more); the folder `/cdn.scutoons.com/` stays as a
    backup until 10.7 and is the later backup target (ASSETS.md "Backup", `FtpStorage`).
- [ ] 10.4 Folder restructure: `assets:layout` re-run + owner review, batch relink (to build), the model path code change,
      then prune (ASSETS.md "Migration plan").
- [ ] 10.5 _(test hosts + T1's origin removed 2026-10-05)_ Remove the test hosts `next.` / `qr-next.` (and T1's origin); later `PROXY_FULL_API=0` when qr's log shows
      no `/api/*` traffic; ~~remove `generate/` + `GENERATE_PASSWORD` after admin parity~~ **done 2026-10-09** (owner:
      "alter Generator kann entfernt werden"): the SPA, its 13 static-token endpoints (incl. `keys.php` with the private
      key), `GenerateToken`, `GENERATE_PASSWORD` and qr's `/generate/` redirect are gone; `get-signature.php` + the
      signature files stay public (`Legacy/SignatureController`). **Owner:** delete the GitHub secret `GENERATE_PASSWORD`
      and `api/public/generate/` on the host if it is still there.
      _`PROXY_FULL_API=0` (branch `10.5-qr-proxy-off`, 2026-10-05):_ `deploy/targets.json` qr "0" and the default in
      `qr/src/Config.php` ("1" = rollback, then deploy qr). Still proxied: `/api/validate/`, `/api/keys/keys.php`,
      `/api/certificates/*`, `generate/get-signature.php` + signature files; the rest of `/api/*` → 404 JSON. Evidence: the
      old viewer (built from this repo's old `main`, `viewer/build`) never had a service worker – no `service-worker.*`,
      `serviceWorker`, workbox or PWA plugin anywhere in git history (only `site.webmanifest`, no SW), the old qr repo neither;
      the new viewer registers none either, so nothing can keep serving the old app. Every live app bundle (viewer,
      creator, admin, shop, embassy at 82c325d; all chunks crawled) calls `https://api.scutoons.com/api`, none
      `qr.scutoons.com/api`; their prod fallbacks now point there too. Mails link only `?login=` / `/account`; the Ask
      service needs `SCUTOONS_API`. Neither the qr app nor the API logs proxied requests – traffic only in all-inkl's
      stats (owner check before the deploy). `generate/` + `GENERATE_PASSWORD` stay (after admin parity, 10.18).
- [ ] 10.7 After one release without problems: delete the old folders (keep backups); the old DB stays as an archive.
- [ ] 10.9 KAS FTP cleanup (after the switch + the CDN move – unblocked once 10.3 is live): delete the users `cdn-sync` (f017c2d3; its repo tooling is gone since 2026-10-04), github actions
      `f017b642` and `f017b651`; remove their old secrets from GitHub (old qr repo, repo-level FTP secrets of the old viewer
      workflow). The main user stays, used only by hand.
- [ ] 10.12 Viewer: `/catpope/credits` and `/catpope/download` work but no page links to them (owner 2026-10-02) –
      **owner decision: link them (toon page / season slot / menu) or drop the routes.** Also: the QR camera never decoded on the owner's webcam (blurry
      image) – check focus / resolution constraints and a sharper decode path once the phone test (Phase 9) is in.
- [ ] 10.14 Admin claim page rework (owner 2026-10-02, during 6.8: "the ui in admin for claim needs rework") – details
      from the owner; first guess: the two side-by-side note fields + the decision buttons hidden in the bottom bar.
      Also: the viewer's claim form promises the current owner never reads the statement – true for owners, but an owner who
      is also an admin recipient gets it in `ownership_claim_admin`; reword if the owner wants.
- [ ] 10.16b Admin claim page: no raw data (owner 2026-10-02) – part of 10.14.
- [ ] 10.17 Deploys are not atomic: FTP replaces files one by one, so for a few seconds the API runs half old / half new
      code (2026-10-02 23:49: "No entry or class found for Transport" during a deploy). Upload to a fresh folder and switch
      (rename / symlink), or a maintenance flag during the upload.
- [ ] 10.21 Asset picker rework (owner 2026-10-03: "we do have to fix the ui of the picker and its relation to the page
      where we pick media – there is redundance and confusion"): drafts first (2–3 layouts, owner picks). Known: the choice box
      opens above the grid, the upload is a heading-looking fold-out, the page's own path fields repeat what the picker shows;
      includes 10.20.
- [ ] 10.23 Asset jobs on production (owner 2026-10-03: "Optimize" stays queued): `assets:jobs` reads the queue from its
      own DB – locally the dev DB, and production's MariaDB is not reachable from outside. The runner on the owner's machine
      (ffmpeg) should take jobs through the API (admin token): list queued → download the source from bunny → transcode →
      upload → mark done. Job #1 (video-renditions, 2026-10-02 22:45) can be dropped.
- [ ] 10.24 Codes case-sensitive + strict session sql_mode (owner 2026-10-03, branch `db-case`): **O – run
      `server/migrations/mysql/2026-10-03-code-case.mysql.sql` in phpMyAdmin** (whole file at once; before or after the API
      deploy, both orders are safe; idempotent). Every code lookup is exact, also `/api/certificates/` + the verify page
      (no case-insensitive fallback; certificates printed before show UPPER CASE – typed that way they are unknown now, the
      QR / signature file still verify); new certificates print the exact code. The API sets a strict session `sql_mode`
      (`DB_SQL_MODE`, production's server mode is not strict). Optional check afterwards: the information_schema SQL in
      server/docs/DATABASE.md "Matching production" (all utf8mb4; the code columns utf8mb4_bin).
- [ ] 10.25 Map places ↔ story: where a place lives (owner 2026-10-03, "lets investigate … isn't group the better place?" –
      **decide first, after the open UI fixes**). Findings 2026-10-03:
  - **Unlock today:** on the place (world map `place.unlock` = toon, chapter, `slice` = the slice's image path, `at`).
    The viewer unlocks it when the reader _sees_ that slice (`world.svelte.ts` `sliceSeen` → `placesForSlice`); a place
    whose chapter has no slices yet unlocks from the chapter position `at` / a finished chapter (`unlocksFromProgress`).
    The position never unlocks a slice place (VN choices skip groups). The slice inspector's "Unlocks place" box is the same
    setting from the slice's side – its wording confuses ("the slice represents the place").
  - **Jump today:** VN `jumpTo` = chapter + optional group label (`/catpope/chapter/2?label=scene-15`), the next bar too –
    groups are the jump unit. The map card links only `place.chapter` (opens the chapter at its start, not at the place).
  - **Idea:** the place on the **group** ("this scene is: Cathedral"): unlocks when the reader enters the group, the card
    jumps to `chapter/N?label=<group>`, one setting shown from both sides (map builder: "Chapter 2 · scene-15"); chapter +
    position stays the fallback for chapters without groups (3–5).
  - **Doesn't line up yet:** chapter 1 is one group of 95 slices with two places (apartment slice 2, apartment-blocks
    slice 72); chapter 2 `scene-20` holds church-square (slice 20) + cathedral (21); `scene-17` has none. Group labels are
    not the screenplay scene numbers (places say scene 13–17, groups are scene-14 … scene-20).
  - **Blocker:** splitting groups breaks the reader's infinite scroll (owner 2026-10-03) – fix that first, or keep groups
    as they are and let a place point at a group + an optional slice (unlock precision) instead of splitting.
  - **Open decisions:** split vs. group + slice; rename groups to their places (labels are in jump links – only with OK).
- [ ] 10.26 Delete a chapter safely (owner 2026-10-03: "how could i delete a chapter? … super dangerous"). Today nothing
      can be deleted: the API has no delete for entities (versions are append-only), so no chapter is lost. Numbers are given
      automatically (next free, never reused) and are the chapter's identity (entity `chapter:<toon>:<n>`, URL, release order;
      shown big in the chapter's Meta since f345743). Proposed rules, enforced by the **server**, the creator only mirrors them:
      only a chapter that was **never published** in any language (status draft) and is the **last of its season**; a typed
      confirmation (the chapter number, in-page dialog); delete = removed from the season's sequence + the entity marked deleted,
      its versions kept (admin can restore; purging is a separate admin step). A published chapter is never deleted – at most
      hidden (draft / coming soon), so links and reading progress keep working.
- [ ] 10.29 **Catalog into the database** (owner 2026-10-04: "lets just do it properly … we have to do them all now"): items, recipes,
      albums, products, editions, featured, permissions / code types, toons (admin-created) → tables + history; slots / stats → code;
      basic admin UI with the asset picker and bulk imports. Plan and status: agents/CATALOG.md. Part 1 (tables, loaders,
      admin API) built 2026-10-04 on branch `catalog/db` _(production migration has run – owner check 2026-10-05: 11 tables, old settings folded)_: **O – run `server/migrations/mysql/2026-10-04-catalog.mysql.sql` in
      phpMyAdmin** (whole file at once; before or after the deploy; idempotent) – steps + check query in CATALOG.md "Status".
- [ ] 10.28 **SCUTOONS MCP server** – an agent with the rights of the signed-in creator (owner 2026-10-03: "very important
      … i want to also connect other services that help me create more sophisticated assets"; first step right after the
      release, branch `mcp`). Why MCP and not only an "Ask" panel: the API can do much more than the UI, any MCP client can use
      it (Claude Code / Desktop today, claude.ai connector + the creator's own panel later), and other MCP servers (asset
      generation, image / audio tools) combine with it in the same client – the agent makes an asset there and puts it into a
      slice here.
  - **Package** `tools/mcp` (`@scutoons/mcp`, TypeScript, official MCP TypeScript SDK; versions checked when building).
    Calls the existing PHP API with the creator's token (`Authorization: Bearer`, like the creator) → the API's role
    checks apply unchanged: the agent can do exactly what that creator can.
  - **Tools** (step 1): `list_toons`, `list_chapters` / groups / points (`scriptLabels`), `read_entity` (draft or published,
    one or all languages, with version ids), `save_draft` (content + base version → 409 on a newer edit; runs
    `@scutoons/content` `validateFile` first and returns the problems), `validate`, `list_versions` / `restore_version`,
    `search_assets` (+ upload later), characters / cast / moods, map places. **Publish** is a separate tool, off by default
    (env flag), one explicit confirmation per call. Saves only ever create new draft versions (append-only, restorable).
  - **Resources**: the format rules the agent reads first – VN events (label / divert / if / setVariable / choice actions /
    anyOf, `vn-script.ts`), text presets, slice sizes and media variants, entity keys, the one-click and flow rules.
  - **Transport**: stdio first (local: Claude Code / Desktop against local or production API, token from `.env` /
    `npm run dev:test-user` locally). Remote (Streamable HTTP + sign-in, claude.ai connector, the creator's "Ask" panel via
    the API's MCP connector) is step 2 – the shared hosting (FTP deploy) can't run a long-lived Node server: a small Node
    host or a PHP implementation, owner decision then.
  - **Checks**: a script like `check:stroll` that drives the tools against the local stack (read → patch → validate → save
    draft → restore), never against production.
  - **Example task**: "set up the chapter 2 stroll" (today `scripts/dev/stroll-chapter2.mjs`) as one request.
  - **Step 1 built (2026-10-03/04, branch `mcp`)**: `tools/mcp` (`@scutoons/mcp`, MCP TypeScript SDK v2
    `@modelcontextprotocol/server` 2.3 – the split packages that replace `@modelcontextprotocol/sdk`; zod 4). Runs from
    the repo with Node's type stripping (`bin/scutoons-mcp.mjs` → `src/main.ts`, nothing built). Config: `SCUTOONS_API`
    (default local), `SCUTOONS_TOKEN` (locally falls back to `logs/dev/test-user.json`, never for a remote API),
    `SCUTOONS_MCP_PUBLISH=1` (publish tool, off by default). Tools: `list_toons`, `list_chapters` (groups: label, scene
    entity, slices, `scriptLabels` points), `list_entities` (every stored document of a toon / the platform),
    `search_content` (full text in a toon's documents, the screenplay per line), `read_entity` (draft / published /
    version id, one or every locale, `path` / `lines` for a part), `read_characters` (cast season 1 like the creator,
    looks, moods), `read_map_places`, `search_assets`, `upload_asset` (a new file from a URL or local file → the asset
    manager, 409 when the path is taken), `validate` (validateFile with the chapter editor's context + validateLocales;
    validateCast for cast), `save_draft` (`ops` set / insert / remove on the `base` version, or whole `content`; errors
    refuse the save unless `allow_errors`; message gets "(agent)"; 409 explained), `list_versions`, `restore_version`,
    `submit_for_review`, `publish_entity` (only with the env flag, `confirm: true` per call). Resources
    `scutoons://rules/{workflow,entity-keys,vn-events,flow,text-presets,slices}` generated at start from the source.
    `npm run check:mcp` (`scripts/dev/mcp-check.mjs`, 31 checks, local only, restores what it changes). README: Claude
    Code / Desktop, local + production. Not validated yet: season status of a chapter and placed items against the
    catalog (the creator passes both).
  - **Step 2 decisions (owner 2026-10-04)**:
    - **Order:** the creator's "Ask" panel is the way for creators; the API token (step 1, stdio) stays for Claude Code /
      Desktop; no claude.ai connector for now (no OAuth needed – revisit only if creators want claude.ai).
    - **Host: bunny.net Magic Containers** (EU, the same bunny account as the CDN): the Ask service in a Docker container.
      Billed by usage (~$0.02 / core-hour + RAM, account minimum $1/month) – check the real idle cost in the 14-day trial.
      Rejected: Cloudflare Workers (US; free plan 10 ms CPU per request is tight for validating a big chapter; needs
      bundling), bunny Edge Scripting (Deno, a port), Koyeb / Render / Hetzner / Scaleway (a second provider).
      Also considered and set aside (owner 2026-10-04: "we stick to the node server on bunny"): the agent loop in the PHP
      API (tools in the browser, or a PHP port of the story rules). Separate idea, not decided: the API checking every
      save against the generated JSON Schemas (`shared/content/schema`).
    - **Auth: long-lived personal access tokens** for Claude Code / Desktop (owner's choice; OAuth was recommended as
      safer). To build: a creator page "Tokens" – create (shown once, only the hash stored), name, last used, revoke; never
      the browser session; the API accepts them as Bearer like a session. Expiry optional.
    - **"Ask" panel: the owner pays** (SCUTOONS, personal business); model Claude Sonnet 5.5. Every AI call is logged:
      creator, service, model, tokens, cost → an admin overview by creator and service per day / month. **No limits in
      SCUTOONS** (creators need to get the job done; the owner controls spending on the Anthropic account side).
    - **Asset upload: yes** (`upload_asset`; remote: from a URL only).
    - **Publish: keep the tool** (drafts get reviewed; the chat history shows what an agent did). Today off by default with
      `confirm: true` per call, and the API lets every creator publish. Review levels (who may publish without a review)
      are a separate task in the API, for agents and the UI alike.
    - **Chat history: yes** (owner 2026-10-04: "I need to see what I did before if something stops or breaks").
  - **Step 2 built (2026-10-04, branch `mcp`)**:
    - **Ask service** `tools/mcp/src/ask.ts` (`npm run dev:ask`, port 9002 in dev-ports; `npm run dev` starts it when the
      repo-root `.env` has `ANTHROPIC_API_KEY`): `POST /api/ask`, Server-Sent Events. The creator's session (Bearer or the
      `scutoons_sid` cookie) checked against `/me/` (role creator); the MCP tools in-process (InMemoryTransport) with that
      session; Claude Sonnet 5.5 (server-side fallback `default`, effort medium, cached system prompt with the format
      rules); context per message: page, the header's text language, the app language. Every API call → JSON line in
      `logs/ask/usage.jsonl` (creator, model, tokens, cost in USD). A loop guard of 60 rounds (not a spending limit).
    - **Chat history** in the API: table `agent_chats` (migration `2026-10-04-agent-chats`, **to run on production**),
      `/api/agent-chats` (list / get / put / delete, role creator, own chats only, 503 without the table). The service
      saves after every round and loads a chat to go on with it.
    - **Creator panel** (`creator/src/lib/components/ask/ask-panel.svelte`): round header button; docked right, the page
      makes room, drags wider; full screen on phones. Streams the answer, each tool call a step, saved drafts as links;
      stop, new chat, earlier chats (clock button), the open chat comes back after a reload. `VITE_ASK_URL` for
      production (empty = "not available").
    - **Deploy prepared (2026-10-04, branch `ask-deploy`)**: `tools/mcp/Dockerfile` (+ `Dockerfile.dockerignore`, build
      from the repo root; node:22-alpine, runs the TS sources, port 8080, health `/api/ask/health`, SIGTERM; built and run
      locally); `.github/workflows/ask-image.yml` pushes `ghcr.io/mandymozart/scutoons-ask:latest` on changes to main.
      Usage log in the API: table `agent_usage` (migration `2026-10-04-agent-usage`), `POST /api/agent-usage` (the
      creator's session + the shared `AGENT_SERVICE_KEY` / `SCUTOONS_SERVICE_KEY`; user from the session), admin
      `GET /api/admin/agent-usage?from&to&group=day|month` (per creator / service / model); the service falls back to its
      file. Deploy: `AGENT_SERVICE_KEY` into the API's config.php (targets.json), `VITE_ASK_URL` into the creator build.
      Verified locally: a question → 2 calls, $0.04, one row per call in the database, the overview sums them.
      **Owner steps** (tools/mcp/README.md "Production"): the two migrations in phpMyAdmin; the secret `AGENT_SERVICE_KEY`;
      bunny Magic Containers app (image from ghcr.io, port 8080, env `ANTHROPIC_API_KEY`, `SCUTOONS_API`, `ASK_ORIGINS`,
      `SCUTOONS_SERVICE_KEY`); DNS `ask.scutoons.com`; the variable `VITE_ASK_URL`.
    - **Live (2026-10-04):** the three migrations (reviews, agent-chats, agent-usage) ran on production; secret
      `AGENT_SERVICE_KEY`; bunny Magic Containers app `scutoons-ask` (id `ZBYjE7aB2VZvKZ6`, container `scutoons-ask`, region
      AT, CDN endpoint → `https://ask.scutoons.com`, env `ANTHROPIC_API_KEY` (its own production key), `SCUTOONS_API`,
      `ASK_ORIGINS`, `SCUTOONS_SERVICE_KEY`; registry: a GitHub token with `read:packages`); creator built with
      `VITE_ASK_URL=https://ask.scutoons.com/api/ask`. The answer streams through bunny (owner checked). **Auto-deploy:**
      `ask-image.yml` → bunny's action `BunnyWay/actions/container-update-image` with the environment variables
      `BUNNY_ASK_APP_ID` / `BUNNY_ASK_CONTAINER` and the secret `BUNNY_API_KEY` (verified: bunny switched to the pushed
      commit's image, rolling update, health 200 throughout).
    - **Open:** ~~an admin page for the usage overview~~ (Dashboard, 2026-10-05); the build's commit in `/api/ask/health` (to see
      which image runs); personal access tokens.
    - **Shared in three apps (2026-10-04, branch `ask-shared`; owner: "make Ask a shared integration in embassy,
      creator, admin")**: the panel is `@scutoons/ask` (shared/ask: `AskPanel` + `ask` / `askAbout` / `pagePanel`; the
      app passes its name, chats API, labels, context, links and an optional result snippet; `panelWidth` moved to
      `@scutoons/ui`). The request names its app; the service picks role, prompt and tools per app (`APPS` in ask.ts):
      creator → role creator, content tools (unchanged); embassy → liaison / ambassador / admin, `src/embassy.ts`
      (agents/EMBASSY.md "Ask + mail"); admin → role admin, the content tools that only read + `agent_usage`. Chats
      and usage rows carry `app` (migration `2026-10-04-ask-embassy-mail`); every message carries today's date.
      `ASK_ORIGINS` (bunny env) must list creator, embassy and admin (the code default does, the env wins); `VITE_ASK_URL` goes into the embassy and admin builds too.
    - **Admin agent (2026-10-05, branch `ask-admin`; owner: the admin's Ask could not answer "signed codes for Cat Pope –
      Awakening", "add an event", "replace item images" and guessed wrong):** step 1 – read tools over the admin API in
      `tools/mcp/src/admin.ts`: `dashboard`, `count_codes` (per edition; toon / edition / status / type), `list_codes`,
      `read_code` (+ timeline + ownership history), `list_events` (the activity log), `list_users` / `read_user`,
      `list_claims` / `read_claim`, `email_log`, `read_catalog` (any kind, `q`, `fields`), `catalog_history`,
      `read_calendar`, `asset_usages`, `read_settings`, `agent_usage`; the admin prompt (ask.ts `ADMIN_SYSTEM`) carries a
      map of the admin's pages, what the agent can't do, and the ambiguous words (event, edition, item image, season →
      the toon's editions). Step 2 – `propose_changes` (never writes): a **changes card** in the panel
      (`admin/src/lib/components/changes-card.svelte`, the mail-draft pattern; before → after, image thumbnails); Apply
      makes the changes in the admin app with the admin's session: catalog entries of items / recipes / albums / products /
      editions / toons (fields by dot path, one PUT per entry with note "Ask: …" → catalog history + undo; refused when
      the entry changed since the agent read it), calendar entries (add / update → ONE new draft of `events`; the Calendar
      page now offers Publish for a newer draft), an asset file replaced under its path (`PUT /assets/replace`; the new
      file from another asset path or picked in the card). Not offered: revoking / transferring codes, claims decisions,
      deletes, roles, asset cleanup, new catalog entries. **Dashboard → AI usage**: this month per app (tiles) and per
      person, cost per day in the dashboard's period (bars per app) – `admin.agentUsage` in `@scutoons/api`. No migration
      (uses `agent_usage` from 2026-10-04). Verified locally: the three questions, a catalog change applied through the
      card and undone in the History. Not verified live: an asset replace (the local asset registry was empty).
  - **Review levels (owner 2026-10-04; follows "Publish: keep the tool")** – who may publish without a review:
    - **Decided:** not a role – a relation per toon, set by an admin, like GitHub reviewers. `reviewer` (per toon or `*`)
      gets a mail on every submit and publishes or sends back; `needs_review` (per account per toon or `*`) = may save
      drafts and submit, not publish / unpublish. No rule = publishes directly (as before; no backfill). Reviewed levels:
      every toon-level entity (toon page + settings, season, chapter + scenes, cast, credits, **script** too); platform-wide
      ones (characters, moods, people, roles, events, map, asset-requests) publish directly for everyone. A submit is
      pinned to its version (the reviewer publishes what they read; newer drafts stay drafts); a newer submit of the same
      file annuls the open one. No reviewer assigned → `409 no_reviewer` "ask an admin to add a reviewer". Mails: review
      requested (reviewers), published / sent back (author) – one per person per action, listing the files.
    - **Built (API, branch `claude/vigilant-chandrasekhar-67b979`):** migration `2026-10-04-reviews` (**production: run it**;
      two new tables `content_review_rules`, `content_reviews`), `Domain\Content\Reviews`, `GET|POST /api/reviews`,
      `POST /api/reviews/publish|return`, `403 review_required` on `/api/entities/<key>/publish|unpublish`, admin
      `/api/admin/review-rules` + `/api/admin/users/{id}/review-rules/{toon}/{rule}`, mails `review_requested` /
      `review_published` / `review_returned` (de + en), setting `CREATOR_URL`. server/docs/API.md "Reviews", PHPUnit `ReviewsTest`.
    - **Built (UI, drafts approved by the owner 2026-10-04):** admin user page → section "Reviews" (the account's rules,
      add toon / All toons + Reviewer / Needs review, remove). Creator: for `needs_review` accounts every page's Publish
      button is "Submit for review" (the publish dialog + an optional note; `lib/editor/reviews.svelte.ts`), the history
      panel hides publish / unpublish for them and marks versions "in review"; the dashboard shows "To review" (one row per
      submit: author, time, languages, note, "newer drafts" hint, Send back / Publish with a note to the author) and "Your
      submissions" (in review / sent back with the reviewer's note) – both only when not empty. Mails name the toon
      ("Cat Pope", not the id) and link to the dashboard (owner: no further action needed than the list).
    - **Agents (2026-10-04, branch `ask-deploy`):** the MCP tool `submit_for_review` (`POST /api/reviews`, items pinned to a
      version + note); `publish_entity` answers `403 review_required` for `needs_review` accounts – the agent submits instead.
- [ ] 10.31 **Naked A-pose removed** (owner 2026-10-05: "Remove the naked A-pose variant. Keep the safe-for-work switch in
      the account.") _(branch `apose/no-naked`)_: the only standard A-pose look is `swim`; the API and the reader ignore a
      leftover `naked` look (or one still marked `sensitive: true`), the creator's validation warns about it; no cover rule
      any more (legs may be empty – the swim body has the pants painted in); `AVATAR_DEFAULT_UNLOCKS` = `{"catpope":["swim"]}`.
      "Hide sensitive content" stays in the account (it will blur / mute sensitive slices). **O – production steps, in order:**
  1. Deploy (API + viewer + creator).
  2. phpMyAdmin → d0487143 → SQL: run `server/migrations/mysql/2026-10-05-no-naked.mysql.sql` (idempotent: naked bodies →
     swim, naked unlock rows deleted – the character stays unlocked –, a stored settings override cleaned). If the
     installation's `config.php` / environment overrides `AVATAR_DEFAULT_UNLOCKS`, drop `"naked"` there too (harmless
     otherwise: the API ignores it).
  3. Delete the naked A-pose looks in the creator (registry: each character → A-pose → × on "naked"; the Problems panel
     lists them as "no longer used"), publish the registry, then delete the CDN files `characters/*/…_apose-naked.webp`.
- [ ] 10.32 **VN event `showOnomatopeia` → `showTextBox`** (owner 2026-10-06, branch `chapter-neutral`): the event
      reveals any text box; the misspelt old name is gone without an alias (types, validation, schemas, viewer, creator).
      The creator's picker is one select "choose or create new" (DESIGN.md "Onomatopoeia = a text box"). Dev DB done
      2026-10-06 (31 rows in entity_versions, all compact `"type":"showOnomatopeia"` in old catpope chapter 2 / scene-14
      versions; 0 left). **O – production steps, together (old code can't read the new type, new code not the old):**
  1. phpMyAdmin → d0487143 → SQL, check first: `SELECT COUNT(*) FROM entity_versions WHERE json LIKE '%"showOnomatopeia"%';`
  2. Deploy (API + viewer + creator), then right away run `server/migrations/mysql/2026-10-06-show-text-box.mysql.sql`
     (idempotent; entity_versions + content_versions); the SELECT of step 1 answers 0 afterwards.
- [ ] 10.33 **Expressions in texts + the clock syntax** (owner 2026-10-06, branch `chapter-neutral`, EXPRESSIONS.md step 2):
      texts use ink's inline syntax; the old `{name:clock}` is gone (no alias) – it is `{clock(name)}` now, validation
      calls the old form an error and the API refuses to save it (422). Dev DB (= the production dump) checked 2026-10-06:
      0 occurrences in entity_versions / content_versions; the migration is for production's newer test content.
      **O – production steps, together with the deploy of step 2:**
  1. phpMyAdmin → d0487143 → SQL, check first:
     `SELECT COUNT(*) FROM entity_versions WHERE json REGEXP '\\{[A-Za-z_][A-Za-z0-9_]*:clock\\}';` (and the same on
     `content_versions`).
  2. Deploy (API + viewer + creator), then run `server/migrations/mysql/2026-10-06-clock-syntax.mysql.sql` (idempotent,
     `REGEXP_REPLACE` → `{clock(name)}`; tested on MariaDB 10.11.14); the SELECTs of step 1 answer 0 afterwards.
- [ ] 10.34 **Events: repeat rules, ids, askText, the reader's name** (owner 2026-10-06, branch `chapter-neutral`,
      EXPRESSIONS.md step 3): a trigger's / placed item's boolean `once` is the repeat rule `repeat` now (no alias;
      every time = none – the new default; what `once` meant = `{"mode":"view"}`); events get hidden ids (the creator
      assigns them – stored ones get theirs with the next save, a bar says so); `users.display_name` for the reader's
      name. Dev DB done 2026-10-06 (81 versions changed: 166 `"once":true` triggers, 55 triggers and 4 tap-event items
      without once → view; 0 left; the result checked against a JSON-walking transform of all 457 versions).
      **O – production steps, together with the deploy (old code reads no `repeat`, new code no `once`):**
  1. phpMyAdmin → d0487143 → SQL, check first: `SELECT COUNT(*) FROM entity_versions WHERE json LIKE '%"once":%';` (and
     the two REGEXP SELECTs in the migration's header; the same on `content_versions`).
  2. Deploy (API + viewer + creator), then run `server/migrations/mysql/2026-10-06-repeat.mysql.sql` (idempotent, PCRE
     JSON walk – < 1 s on the dump) and `server/migrations/mysql/2026-10-06-display-name.mysql.sql` (`ADD COLUMN IF NOT
     EXISTS`); the SELECTs of step 1 answer 0 afterwards.
  3. Event ids: nothing to run – a chapter / scene opened and saved in the creator stores them; until then validation
     warns ("N events have no id yet") and the reader keeps such events' "once per page view" in memory.
- [ ] 10.35 **Jump targets: string values + jumps into another toon at a label** (owner 2026-10-06, branch
      `chapter-neutral`, DATA.md "Jumps into another toon at a label"): `JumpToTarget.toon` is a new optional field (no
      migration; old code ignores it – such a jump would land in the CURRENT toon's chapter of that number – so deploy
      viewer + creator + API together before saving such jumps). `content:check` found the old chapter 2 stroll's targets with a NUMBER value (`"value":2`
      – the type says string): 11 versions, all `scene:catpope:2:scene-14` (+ de / en), 3 targets each.
      **O – production steps:**
  1. phpMyAdmin → d0487143 → SQL, check first:
     `SELECT COUNT(*) FROM entity_versions WHERE json REGEXP '"type":"(chapter|season|toon|product|xr)","value":-?[0-9]';`
     (and the same on `content_versions`).
  2. Run `server/migrations/mysql/2026-10-06-target-value.mysql.sql` (idempotent, `"value":2` → `"value":"2"`, any
     time – old and new code read both); the SELECTs answer 0 afterwards.
  3. **Owner decision**: the published `scene:catpope:2:scene-14` (201, + de 127 / en 128) still has an event of the
     unknown type `end` (`{"type":"end","params":{"jumpTo":…scene-20}}` – the reader skips it, `content:check` reports
     it). Publishing the scene's newest version (no `end`) or deleting the event clears it; which one is yours.
- [ ] 10.36 **One save game across all toons** (owner 2026-10-06, branch `chapter-neutral`, DATA.md "The save game"):
      story variables, the history and every toon's checkpoints move from the per-toon game states (`progress` rows
      `toon_id = <toon>`) into the member's save game (`toon_id = 'scutoons'`, which holds the map). The new viewer reads
      and writes only `/api/progress/scutoons/` (PUT merges: map / checkpoints / history as sets, variables the newer
      write's); old per-toon calls keep working for older viewer builds (their rows are not read by the new viewer).
      Guests: the viewer takes the old `scutoons_game_<toon>` keys in once (same rule) and removes them.
      Variables have a domain (owner 2026-10-06, `@scutoons/content` variables.ts): a toon's old variables keep it –
      `x` of catpope's state becomes `catpope/x` (written `x` in catpope, `catpope.x` elsewhere); `global.x` is shared.
      **O – production steps (with the deploy of viewer + API):**
  1. phpMyAdmin → d0487143 → SQL, check first (the SELECT is in the migration's header): the members with per-toon
     states (`SELECT user_id, COUNT(*) … WHERE toon_id <> 'scutoons' GROUP BY user_id`) – each toon's variables go in
     as `<toon>/x`, so there are no clashes between toons.
  2. Run `server/migrations/mysql/2026-10-06-global-save-game.mysql.sql` (idempotent: a second run changes nothing; the
     per-toon rows stay as they were – rollback = the old viewer). Afterwards the header's last SELECT answers 0.
  3. Deploy viewer + API together right after (an old viewer keeps writing per-toon rows the new one doesn't read; a
     member who reads in between with an old build: their per-toon progress after the migration isn't merged – run the
     migration again after the deploy, it folds them in by the same rule).
      _(2026-10-06: the migration ran on production in its final form – the save games hold `catpope/…` keys (3 members
      with a catpope state: one empty, two with item-named story variables `cucumber`, `broken_record_player`,
      `repair_coupon` → `catpope/…`, correct: the items themselves are in the inventory). main pushed 908b97c1.)_
- [ ] 10.37 **No dead ends: the save guard** (owner 2026-10-06, EXPRESSIONS.md "Dead-end safety"): no data change, no
      migration. With the API deploy, `PUT /api/entities` refuses (422) chapters / scenes that can get a reader stuck – a
      question whose choices can all be hidden, a divert / jump into nothing (`FlowCheck`); `content:check` reports the
      same. **O:** after the deploy run `php bin/console content:check` on production: a document it lists can't be saved
      unchanged any more – fix it in the creator (the Problems panel names the place). Known from the dump: only the old
      `scene:catpope:2:scene-14` versions (its `scene-20` jump – the 10.35 decision).
- [ ] 10.8 Public release. Set one version everywhere (root + every app + api/qr → `1.0.0`; today root 0.0.0, frontends 0.1.0, api/qr 1.0.0) and tag the commit `v1.0.0`; optional tag `v1.0.0-beta.1` on the beta commit (owner 2026-10-03).
- [ ] 10.38 **Items are not variables** (owner 2026-10-06: "it is just a naming conflict"; "with script we can not
      create items"): an item exists only in the catalog (admin) and lives in the reader's inventory (global, per
      account); a story variable is the script's own number / text (`x` = this toon's, `global.x`, `<toon>.x` –
      variables.ts). Today they mix: `set` / `add` of a variable named like a catalog item also grants the item (the VN
      item bridge, `grantStoryItem` in viewer variable-actions.ts; a slice pickup grants and then `add`s the variable),
      so `catpope/cucumber` is a story count that drifts from the real inventory (eating, crafting). Steps:
  1. ✅ _(2026-10-06, 6b7a46f0)_ Read: `inventory.<item>` = how many the reader has (shorthand for
     `inventory.count("<item>")`; the analyser checks it against the catalog – `unknown-item`; viewer, creator sample
     readers, the PHP analyser). A namespace is never a variable's domain (`inventory.x = 1` is refused).
  2. ✅ _(dadb1e2d)_ Warn: a story variable named like a catalog item – setVariable / choice set + add / askText ("give
     it with giveItem"), a condition ("what the reader has is inventory.<item>"), a bare item name in an expression
     (`item-as-variable` → `inventory.<item>`).
  3. ✅ _(dadb1e2d)_ Give: VN event `giveItem` { item, qty 1 … 10 } and a choice's action `give` { item, qty } – types,
     validation, schemas, the Flow box ("gives cucumber ×2"), the viewer (`giveStoryItem`: source story, once per place),
     the creator (+ giveItem, "give an item", the GiveItemField with the catalog picker).
  4. **Take – decided (owner 2026-10-07):** VN event `takeItem` { item, qty, repeat } and a choice's `take`.
     - What: an item with the catalog flag **"a story may take it"** (admin, off by default) – also another toon's items;
       **physical items never** (only claims move those).
     - Repeats: the trigger / pickup intervals (`repeat`: every time – the default –, once, per page view, every N
       minutes / hours / days), per event.
     - Too few: all or nothing – nothing is taken; the script guards it (`if inventory.x >= n`). In a choice's action
       list a failed take stops the rest (a trade never gives for free); the story goes on. A take can lead into a
       detour (`divert`).
     - Test content: toptoy ch1 (the mama story) – 5 cucumbers to pick up on one slice, Top Toy trades a blotter (`lsd`)
       for 5 cucumbers, swallowing it diverts into a short psychedelic detour.
     - ✅ _(2026-10-07, 53859498 + this commit)_ built: the engine (see the commit), the catalog flag set for `cucumber` /
       `lsd` in the dev DB, toptoy ch1 draft 819 (dev DB): group `blotter-deal` (s1c2-slice-53: 5 cucumber pickups, a tap on
       Top Toy offers "Trade (5 cucumbers)" if `inventory.cucumber >= 5` → take 5 + give a blotter, "Swallow the blotter"
       if `inventory.lsd >= 1` → take it + jump to `lsd-trip`, "No thanks"), `mama-hypercity`, `lsd-trip` (a short
       psychedelic talk, then back to mama-hypercity). End to end headless as the test user: all 7 steps pass. Found on
       the way: the follow-up bar's 5 s countdown went on while the reader picked things up – a tap on the page stops it now.
       **O – production:** set "a story may take it" on the items in admin, and the content via the creator (the dev
       draft isn't on production).
  5. ✅ _(this commit)_ Data: the dev DB analysed (latest versions: 4 choice adds in scene:sandbox:1/2/5 – cucumber ×3,
     testo; older versions catpope's repair_coupon / broken_record_player; no setVariable / condition on an item) →
     `server/migrations/mysql/2026-10-07-give-item.mysql.sql` (choice `add` / setVariable `add` of an item-named
     variable → `give` / `giveItem`, history included; PHPUnit `testTheGiveItemMigration`) – run on the dev DB: 52
     versions. The bridge is gone (`set` / `add` never grant; a slice pickup only grants – no story variable, and it no
     longer counts as a set variable in the facts / FlowIndex). make-sandbox.ts writes `give`. Checked in the browser:
     sandbox chapter 2's testo ("Einsammeln") lands in the inventory, source story, no `sandbox/testo` variable.
      **O – production:** run `2026-10-07-give-item.mysql.sql` in phpMyAdmin **together with** the viewer + creator
      + API deploy (an old viewer doesn't know `give` – a choice would give nothing; the new one doesn't grant on
      `add`). Check first: the header's SELECT; afterwards it answers nothing.
- [x] 10.39 **Creator: copy / cut / paste** (owner 2026-10-07: "as granular as possible", to save editing time; the main
      case: from one toon or hidden draft to another, inside the same system). Concept: UI-CONCEPTS.md E.7. Decided:
      - **One system only.** No sandbox → production transfer (owner: "too much overhead") – no media in the clipboard,
        no upload on paste.
      - **System clipboard, no API change:** the DOM copy / cut / paste events (no permission prompt), JSON with a
        marker; a paste goes into the open document (`doc.edit` = one undo step) and is saved by the normal PUT.
        Cmd/Ctrl+C / X / V; Shift+V = the settings only onto the selection (a text box's look, an event's params …).
        Ctrl+X today only deletes (chapter-editor.svelte onKeydown) – it becomes copy + delete.
      - **The live region shows (10.40, UI-CONCEPTS E.8):** the region clicked / focused last is live; its selection is
        light cyan with a primary-colour border and dark blue text, selections elsewhere stay inverted; the shortcuts
        act on the live selection only.
      - **Ids:** fresh on paste (events via assignEventIds, text boxes via `taken`, `showTextBox` follows its box).
        Placement ids become **random** (`<item>-<7 random>`, like event ids) for new placements and pastes – today
        `<item>-<n>` is unique per slice only, so the same image reused on another slice of the toon with the same
        item shares the pickup memory (`picked:<slice image>#<id>`, slice-items.ts). Existing ids stay (readers'
        keys); a one-off SELECT for collisions that already exist.
      - **Media: cross-toon references, no copies** (owner: duplicates "create a mess"). A paste into another toon
        writes `../<source toon>/…` (owner 2026-10-07: keep `../`); one helper `toonAsset(toon, path)` in reader + creator replaces every
        `` `${toon}/${path}` `` (slice image, variants, video, audio, key visual) – today only the VN character views
        read a leading `/`, so a picked slice image from another toon is broken (an existing bug, fixed by this).
        Stored `slice.image` values are never rewritten (the pickup key contains them) – a SELECT first for `/` / `../`.
      - **Sounds:** the clipboard carries the used `audio` library entries; a paste adds the missing ones.
      - **Item limits, combined (owner):** placements across all toons (`placements: n`) AND owners in the world
        (the planned `unique` / `max_world`, ITEMS.md). **API extension:** the catalog fields (admin), a count on save
        (PUT /api/entities → 422 over the limit), `GET /api/catalog/items/<id>/placements` (where else it is placed),
        the grant check across all accounts. The creator: a Problems error + a paste refuses with a toast.
      Steps: 1. the live region = 10.40 (first) · 2. clipboard + text boxes (+ Shift+V) · 3. items, sound + event areas · 4. VN
      events · 5. slices + groups, `toonAsset` + the path SELECT · 6. random placement ids · 7. item limits (API) ·
      8. optional: a right-click menu.
      _(Built 2026-10-07, all steps – Day run result. Decided on the way: a paste into another toon writes `../<toon>/…`
      (the asset picker's form; all 272 out-of-folder values in the dev data are `../`, none `/`) – `toonAsset` reads
      both. SELECTs: 0 pickup-key collisions (6 placements). No migration: the limits are catalog JSON fields.
      **Owed:** PHPUnit ItemLimitsTest + browser checks of steps 6–8 (Docker hung).)_
- [x] 10.40 **One selection model, app-wide** (owner 2026-10-07: "we need an all app wide concept, done thoroughly";
      concept + inventory + drafts: UI-CONCEPTS.md E.8). Inverted today means six things (nav, selection, listbox
      highlight, toggle on, hover, progress) and "selected" is drawn four ways. Decided (owner 2026-10-07, drafts on the
      chapter page): selected = inverted (brutalist, unchanged); **live selection** (selected AND in the live region –
      what keys act on) = `--surface-tint-cyan` background, 2px inset border in `--primary-color`, text #005a70 (new
      tokens `--select-live-*`); nav "you are here" stays inverted (live look only when it is also the live
      selection); toggles / tabs / segmented stay inverted; hover never inverted; focus = 3px ink everywhere (fix the
      `all: unset` gaps, toggle 2px, menu). One `region` action + `liveRegion` store in @scutoons/ui, one class
      `sc-selected`, the only selection CSS in tools.css; keys (Delete, arrows, Esc, copy / paste) act on the live
      region only – fixes Delete in the slice list deleting the canvas box. Steps 1–6 in E.8 (tokens + tools.css →
      @scutoons/ui → creator chapter editor → other creator pages → admin / embassy → checks). Picked = the selected
      look, all cyan when live (owner confirmed 2026-10-07). Built 2026-10-07 (UI-CONCEPTS E.8 status).
- [ ] M.3 **Reader: avatar / portrait without a message box** – with no box of any kind showing they sit lower,
      anchored to the page bottom (not to the top of the box); when the end actions slide in they push avatar and
      portrait up like a box does. Rethink: the end actions INSIDE the message box (it is already the place for
      anything the reader does), one consistent style that gets readers used to the VN style. _(concept first – E.5)_
      _(2026-10-07, part 1 built – the ground: the box reports its height only while it shows, the follow-up bar reports
      its own; the characters stand on the shown box, else on the bar, else 1 rem above the page bottom (vn/characters.svelte,
      message.svelte, next-bar.svelte). chain-vn 24/24; not watched with the box gone (the pane was hidden at night) –
      **owner: a look on the phone**. Part 2 (the end actions inside the box) = UI-CONCEPTS.md E.5, drafts first.)_
- [x] M.4 **Creator: the slice editor needs more room** (owner screenshot 2) – a bigger canvas by default, zoom like
      the Flow graph, the tool icons (text, item, sound, effect) as a vertical strip on the left, Snap + its "30 px"
      as one small button (options on click), the slice takes the freed space. _(concept first – E.4)_
- [x] M.5 **Top messages need a new concept** (screenshot 2, the "Published ×" banner) – they take a lot of room,
      appear often and make the layout jump; informative ones ("Published") fade by themselves; they show without
      pushing the layout. _(with M.13 – E.1)_
- [x] M.6 **Navigation hierarchy (concept)** – bug: the Flow view jumps at slice / group / chapter / season / toon
      level. Problem: two navigations compete (left of the edit view, and the Flow on the right; either jumps to the
      other), so it's never clear how the edit view relates to the Flow's level. Idea: the left menu and the main
      panel are linked and ARE the hierarchy; Ask and Flow only take their context from it and otherwise stay
      independent – entry points, not a second navigation. _(with M.14 – E.3)_
      _(2026-10-07, the jump fixed (UI-CONCEPTS.md E.3 a): the chapter Flow was rebuilt on every selection
      ({#key level:current:slice} → SvelteFlow + dagre + the first view at zoom 1 again). Now only a level change rebuilds
      it; a new group / slice on the same level updates the graph in place, the zoom stays and the view pans only when
      the current box is out of sight (flow-panel.svelte, flow-controls.svelte). creator-dock 82, creator-reach 21; the
      feel not watched (the pane was hidden) – **owner: try group → group, slice → slice**. The concept (sidebar + main =
      the hierarchy, the dock keeps the panel across pages) = E.3 b/c.)_
      _(2026-10-07, E.3 b/c built (BACKLOG 10.46): the Flow is one dock panel toon → season → chapter (zoom kept), Ask
      reads the crumbs. **Owner: try toon → season → chapter with the Flow open, and group → group.**)_
- [x] M.9 **Creator: strip preview for vertical groups** – the same strip preview horizontal groups have. _(after M.4)_
- [x] M.10 **Explore: the viewer as a dock panel** (like Flow and Ask) – up to date with every saved draft; selecting
      an element in the other views jumps the viewer there. Exploration first, not a fix. _(after M.6)_
- [x] M.13 **Creator: the published / unpublished status bar takes room** (e.g. from the slice editor). _(with M.5 –
      E.1)_
- [x] M.14 **Header + breadcrumbs take too much room** (the creator title next to the SCUTOONS logo, plus the crumbs).
      Too many crumbs → the left ones collapse into a clickable "…". The crumbs are the main way back up – they must
      stay usable (a problem today). The hierarchy: SCUTOONS > Toon > Season > Chapter > Group > Slice (owner: the
      season level too). The creator title is what gives way (owner: "the creator title can go"). _(with M.6 – E.2)_
- [ ] M.15 **Investigate: some slices look poor even at 2x and high quality** – where is it lost: the upload /
      resize pipeline, compression settings, which size variant the reader serves, or CSS scaling.
      **Findings 2026-10-07** (the dev DB = the production seed, its asset registry, the code):
      1. **The files aren't there (the main loss).** No slice has a 3x file; catpope ch1 (95 slices) has no 2x either
         (only ch2's scenes 14 / 17 / 18 have 2x). The registry has one 390 px "original" per ch1 slice (s1c1-slice-1:
         9.5 KB) – nothing larger exists anywhere. The reader falls back down (pickQualityVariant): "High" on a 3x phone
         gets the 390 px file, upscaled 3×. **Fix = content:** re-export from the masters at 1170 px (3x) and drop them
         on "Import slices" (it makes 1x / 2x / 3x from a 3x file, replacing the slice's image, texts stay).
      2. **The level ignores the screen.** low / medium / high = 1x / 2x / 3x by bandwidth only; a 2x laptop showing
         the 500 px reading column needs ~1000 px – the 2x file (780 px) is upscaled 1.28× even on "Medium". Proposal:
         Automatic picks at least the screen's density (devicePixelRatio × the column's width) when the bandwidth allows.
      3. **The encoder.** The browser's WebP (canvas.toBlob, quality 0.88, media-upload.ts) always subsamples colour
         4:2:0 – crisp coloured line art (red / cyan on black) smears a little. Proposal: quality 0.92, or the server
         encodes with libwebp `-sharp_yuv` (the asset jobs) – compare on one slice first.
      4. CSS: `.slice img { width: 100% }` in a ≤ 500 px column – no extra scaling beyond 1. / 2.
      **Owner:** do 1. for ch1 (and ch2's 1x-only scenes 15 / 16); decide 2. and 3. (small code changes).
- [ ] 10.58 **Newsletter** (owner 2026-10-07: mails to every member and to the old QR form's addresses, bundled so the
      SMTP isn't overloaded; first one: the invitation to the next chapter release, week of 2026-10-12) – built on branch
      `newsletter` (agents/NEWSLETTER.md): contacts (old QR form + event lists, language per contact), audiences,
      batches through the SMTP account with an hourly limit (runtime setting, 200), bounces / unsubscribe (link +
      one-click), admin Emails → Newsletters / Contacts / Unsubscribed. **O –** the going-live steps in NEWSLETTER.md
      (migration `2026-10-07-newsletter.mysql.sql`, secret `NEWSLETTER_CRON_KEY`, KAS cron every 5 min, SPF / DKIM /
      DMARC check, contact languages, event lists).
- [ ] 10.56 **Reader FX: feedback cues** (owner 2026-10-07; started the same day: "free to go") – hints, pickups,
      reveals and impacts as one set of cues (motion + sound + haptic) inside the visible viewport; no external MIDI /
      devices. Plan, rules, phases and open decisions: agents/FX.md. The live performance mode that builds on it:
      agents/LIVE.md (target February 2027, explored in its own session). Phases 1 + 2 built (pickup collect / refuse,
      nudge, shape hit-test, contour, account volume + vibration; `npm run check:fx`), 3 (entrances pop / slam / fade /
      typeOn + a sound per reveal) and 4 (the `fx` event: shake / flash / punch / vignette). Open: an item's own pickup
      sound (catalog `ItemDef.sound`, a migration) – owner decision on the sound design first (FX.md open decisions).

## Done

- [x] B.2 Scene files use `{"type":"end"}` → "Unknown event type" _(MEMORY.md, 2026-09-30)_.
      _(2026-10-07, re-checked: still in the PUBLISHED versions of scene:catpope:2:scene-14 (neutral 201, de 127, en 128)
      and older drafts – stored with spaces (`"type": "end"`, an end with a jump to scene-20), so a first search for the
      compact form missed it; content:check and the new `setup` command report them. The same case as 10.37's known one
      – decided with 10.35 (the scene-20 jump).)_
      _(closed 2026-10-07, owner: "only configuration – a test scene"; the owner fixes it in the local DB.)_
- [x] M.2 **Reader: the page jumps when animations fire** – e.g. when a showTextBox effect fires, the page jumps oddly.
      _(2026-10-07, not reproduced: the effects are transforms inside the absolutely placed box (no layout change);
      scrolling catpope ch2 at 390 px recorded no layout shift and no scroll jump; a slice's showTextBox click
      trigger only plays inside its group's run. **Owner: where exactly (chapter / slice / phone or desktop)?**
      Candidates once it is seen: the box's text fitting (dialog.svelte measures and resizes), the wait overlay
      (an inset-0 button over the VN area), the pulse's scale(1.2) at a slice edge.)_
      _(found + fixed 2026-10-07 with the owner's repro (catpope ch1 slice 31, the "GRRRR" showTextBox): the box
      reaches past the slice's right edge (x 37 % + 70 %, rotated), the reader didn't clip a slice sideways, so the page
      became 421–424 px wide on a 390 px screen while it shivered – a phone moves / re-zooms such a page. Now `.slice`
      clips sideways (`overflow-x: clip`, shared/reader slice.svelte – up / down a box may still overlap), the page stays
      390 px (measured headless at 390 × 844); the creator warns about a box past a side (the only one in the content).)_
- [x] 10.19 _(priority last – owner 2026-10-06)_ Setup / installer for a fresh environment (owner 2026-10-03): one step that runs the migrations, the content
      seed and the asset scan in order and checks each (the cutover did them by hand – 6.9 found the empty asset registry
      only through the creator's picker). Content keeps plain CDN paths; the registry is the index for the tools.
      _(locally largely `npm run dev:db` since 10.30: migrate + prod seed incl. the asset tables; no content seed any more)_
      _(done 2026-10-07: `php bin/console setup` (locally `npm run dev:db -- console setup`) – schema (pending
      migrations), content (stored versions + catalog), assets (registry), content:check, settings (CDN_URL, MAIL_FROM,
      LOG_PATH writable); report only, `--apply` runs migrate / assets:scan, `--apply --seed` also db:prod-seed.
      On the dev DB: everything ok but the check – the old scene-14 versions (B.2 / 10.37).)_
- [x] 10.11 Server errors in admin (owner 2026-10-02, after reading `/web/api/logs/error.log` by FTP to find the dashboard
      500): an admin-only API endpoint that reads the newest lines of `LOG_PATH` (newest first, size-capped, no secrets –
      messages only), and an admin page "Server errors" showing them (filter by day / text). Same for the qr app's log.
      _(done 2026-10-07: GET /api/admin/server-errors (ServerLog: the last 512 KB, entries with their stack trace,
      newest first, masked: e-mail addresses, tokens / keys / hashes, passwords, header values; the server's folder cut
      from paths; PHPUnit ServerLogTest) + admin → System → "Server errors" (API / QR app, day, search, Reload; help).
      The qr app writes its own qr/logs/error.log now (qr/public/index.php; logs/ is 404 in qr/.htaccess); the API reads
      it via QR_LOG_PATH = ../qr/logs/error.log. **O – after the API + qr deploy:** open the page, check "QR app" finds
      the log (api/ and qr/ are siblings on all-inkl – else set QR_LOG_PATH in the API's .env).)_
- [x] 10.20 Asset picker: the selected tile turns black but its kind icon stays black (blank white box) – invert the icon
      too (owner screenshot 2026-10-03). The tile's size sums every file incl. the master (roar-little-kitty 491 MB) – show the
      default rendition's size or label the total.
      _(done 2026-10-07 (c6c4a939): the selected card's picture box inverts; sizes = the asset's own file, "(all N files …)" for the total)_
- [x] M.8 **Reader: horizontal scroll keeps native left / right** – down → right and up → left already; left / right
      input (trackpad swipe, horizontal wheel, arrow keys) must keep working on top.
      _(done 2026-10-07 (750332ec): sideways wheel / swipe / Shift + wheel scroll the page the same distance; ← / → step frames while pinned)_
- [x] M.16 **Creator: the "Add video" picker starts empty** – like the default empty slice; today its placeholder is
      landscape (suggests videos are landscape).
      _(done 2026-10-07 (3fb7a5f0): an empty image / video slot is the default slice, 390 × 625)_
- [x] M.12 **Creator: lighter dividers between a group's slices** – a lighter grey; the selected slice stands out more.
      _(done 2026-10-07 (b3c4051c): --border-muted)_
- [x] M.11 **Creator: "Import slices"** needs the padding of "Add group".
      _(done 2026-10-07 (b3c4051c))_
- [x] M.7 **Reader: "Coming soon" on the chapter key visual** needs a nicer style.
      _(done 2026-10-07 (3fb7a5f0): a tilted comic stamp in the lettering font, ink block – on the gate and over the teaser)_
- [x] M.1 **Reader: the intersect area shows** – the orange dashed outline where a sticker overlaps a speech bubble must
      not appear in the reader (owner 2026-10-06 "minor bugs" list).
      _(done 2026-10-07 (3fb7a5f0): the intersect area is invisible – click / hover / audio areas keep their look (a tap hint?) – owner: hide those too?)_
      _(2026-10-07, owner: "hide click outlines for any interaction element … a toggle in the account: show hints"):
      click / hover / sound areas are drawn only with "Show hints" (account page, off by default; users.show_hints,
      PUT /me/ { show_hints }, migration 2026-10-07-show-hints – **O: with the next API deploy**); guests: off, no toggle
      (like "Hide sensitive"); the creator keeps showing every area. Checked headless: off → transparent, on → outlined.)_
- [x] B.3 Ch3–5 `keyvisual` is a string, the type expects `{image, video?}` (ch2 fixed 2026-09-30) _(MEMORY.md – not
      re-checked)_.
      _(closed 2026-10-07: every chapter's keyvisual is an object; chapters 3–5 have none)_
- [x] B.1 Ch1 audio samples are at `slice.audio.samples` but code reads `slice.audio.audio.samples` → ch1 samples never
      fire _(from MEMORY.md "Known bugs spotted", 2026-09-30 – not re-checked since)_.
      _(closed 2026-10-07: the data has `slice.audio.audio.samples` (catpope ch1 slices 6, 46, 64, 91) – the path the reader reads)_
- [x] 10.18 Batches + code generation on real data (owner 2026-10-03): admin → Batches (and the generate side) still
      read the static lists (editions / items / batch manifests from files); populate them from the database (catalog,
      editions, codes) instead.
      _(done with 10.29 – editions come from catalog_editions (Editions.php), batches are run manifests by design; the last "editions.json" error text fixed 2026-10-06 (owner: "should be done already"))_
- [x] 10.22 Creator chapter tree: "Add group" + "Add separate scene" (cut off, owner 2026-10-03: "weird") – the second is a
      storage detail (a ref group = its own scene entity, from the old file layout). One "Add group"; the editor picks the
      storage. Scene keys (`chapter2/scene-14`) are internal IDs only, never shown as names (owner 2026-10-03: "legacy from
      files – in the db we don't care about ids that way"); the label is the name (done for the tree / crumbs: 3109fd8).
      _(done – the chapter tree has one "Add group" (checked 2026-10-06; owner: "is existing"))_
- [x] 10.13 Visibility rule (owner 2026-10-02): `draft` is seen by creators only, `coming-soon` is the only non-published
      status the public sees. Production's public API follows it (season 1 lists chapters 0, 1, 2 = coming soon). Check
      the signed-in creator's viewer list: it showed chapters 4 and 5 as "coming soon" while the creator marks them draft.
      Later: drafts only for creators who are authors of that toon.
      _(closed 2026-10-06, owner: "not relevant anymore")_
- [x] 10.1b _(done 2026-10-02. Hint: setting secrets with `gh` gave 404 until gh was re-authorised – the fine-grained
      token did not cover the `scutoons` repo.)_ Right after the assets merge: the 7 bunny keys (`BUNNY_STORAGE_ZONE`, `BUNNY_STORAGE_ENDPOINT`,
      `BUNNY_STORAGE_PASSWORD`, `BUNNY_STORAGE_READONLY_PASSWORD`, `BUNNY_PULLZONE_ID`, `BUNNY_API_KEY`, `CDN_PUBLIC_URL`)
      as GitHub `production` secrets for the live asset manager (the merge adds them to `targets.json`); verify in admin →
      Assets → Storage (set / missing per key, no values).
- [x] 10.6 Embassy go-live (EMBASSY.md "To go live"): subdomain + SSL, roles, import (`order` done 2026-10-04). _(done, owner 2026-10-05)_
- [x] 10.10 Local MariaDB in Docker instead of SQLite for dev + tests (owner 2026-10-02, after the admin dashboard 500 on
      DB-T that SQLite never showed): same server version as all-inkl, the mysql migrations, CI runs the suite on it too.
      _(done 2026-10-03, branch `dev-mariadb`; server/docs/DATABASE.md "Local MariaDB")_ `docker/compose.yml` (MariaDB
      10.11, port 13306, volume, healthcheck; Adminer 13380 behind a profile); `npm run dev:db` = container + databases +
      migrate + the `*.mysql.sql` content seed + promo `devpromo` + switch + API restart + test user; `npm run dev:db sqlite`
      back (SQLite stays the default); `npm run dev:db test` / `phpunit -c phpunit.mysql.xml` = the unit suite on MariaDB;
      CI job `test-mariadb` in server.yml. Found on MariaDB: two signups at the same moment deadlocked in the basic dress
      (fixed, test); `qr.code` was case-insensitive in production (utf8mb4_general_ci) → 10.24; a flaky test fixture.
      Production's values (owner, phpMyAdmin 2026-10-03: 10.11.14, sql_mode NO_ENGINE_SUBSTITUTION, latin1 server
      defaults) are the container's defaults since branch `db-case`.
      _(10.30: SQLite, the switch, `dev:db sqlite`, phpunit.mysql.xml and the content seed are gone – MariaDB only, prod seed)_
- [x] 10.15 Content source of truth is the DB now (owner 2026-10-02): local dev from `export-content` of production
      instead of the stale `shared/content/seed`; the seed becomes a starter set for empty databases (or goes).
      _(covered by 10.30: the seed is gone, local dev loads production's export)_
- [x] 10.16 Mail Reply-To (owner 2026-10-02: a mistake, fixed before the beta): `MAIL_REPLY_TO` = hello@scutoons.com
      (Reply-To header; the texts say "reply to this email", mail comes from no-reply@).
- [x] 10.27 VN story logic – **built 2026-10-03 for the release** (owner: "we have to build it now. it is part of the release").
      Concept: jumpTo = navigation (groups are pages, by design); the story's own flow lives in the group's script. Done:
      one runner (`viewer/src/lib/visual-novel/runner.ts`, group script + slice / item triggers); `label` (named point) +
      `divert` (event, choice action, a slice event diverting into its group's script); `if` ("Only if") on any event;
      `setVariable` (set / add); choice actions as a list; `jumpTo.target.anyOf` (a random group, never the current one);
      `{var:clock}`; validation (points unique, diverts hit a point, VN flow per group); creator forms. Chapter 2 stroll:
      `scripts/dev/stroll-chapter2.mjs` (local), `npm run check:stroll` (headless play-through, 15 checks). Still open from the
      research: `once` choices + fallback, visit counts. Original notes:
- [ ] (history) 10.27 VN story logic, ink's ideas in our JSON events (owner 2026-10-03, **after 10.25**; research into inkle's ink).
      **Owner decision 2026-10-03: adopt some of the concepts below – which ones is still open (pick before starting).**
      Not ink itself: ink is a text script (our authors use the creator's forms), its localisation is one file per language
      or line-ID tooling (breaks our per-field de source / en override), and knots would be a second address system next to
      group labels. Today: variables are written (`set`/`add`, `{var}` in text, item grants) but nothing reads them – no
      conditions; a choice has one action; the only branch is leaving the group (a choice without jump continues with the same
      lines). Steps, all shared (structural) fields so translation and 10.25 stay untouched:
  1. One runner: `toon/interactions/visual-novel.svelte` and `visual-novel/event-processor.ts` both run events (the
     README calls the second deprecated, slice triggers + placed items still use it) – one shared step function first.
  2. `addChoice.params.if: { variable, op: '=='|'!='|'>='|'<', value }` – schema, `checkEvent` in validate.ts, an
     "Only if" fold under Advanced in `vn-events.svelte`, the runner skips the choice when false. Then on `showMessage`.
  3. `once` choices (+ a fallback choice) and `onSelect` as a list (set a flag _and_ jump).
  4. In-group labels + `jumpTo {type:'label'}` (branch for a few lines, rejoin – ink's gathers; guard against loops).
  5. Visit counts per group / label (`seen:scene-15`) next to the variables.
- [x] 10.30 Production export as the local seed (owner 2026-10-05: production is the source of truth for content and
      catalog; "i want no sqlite also anymore") _(branch `10.30-prod-seed`)_:
  - **Prod seed:** phpMyAdmin → d0487143 → Export → Custom → SQL, data only, tables entity_versions, toons, catalog_meta,
    catalog_items, catalog_recipes, catalog_albums, catalog_editions, catalog_products, catalog_featured,
    catalog_permissions, catalog_code_types, assets, asset_versions, asset_renditions, asset_usages → `server/db/prod-seed.sql`
    (gitignored – may hold creator ids; a full dump works too: `npm run dev:db -- seed --from=<dump.sql>` keeps those 15
    tables). `php bin/console db:prod-seed [--from=] [--force]` empties the 15 tables and loads the rows in one
    transaction; same file again (SHA-256 in setting `dev_prod_seed`) = no-op.
  - **MariaDB only:** SQLite removed everywhere (migrations/sqlite, `DB_DRIVER` / `DB_SQLITE_PATH`, `--db=` options,
    `npm run dev:db sqlite`, the db.json switch, phpunit.mysql.xml – phpunit.xml.dist targets MariaDB `TEST_DB_*`,
    default 127.0.0.1:13306 `scutoons_test`). `npm run dev:db` = container → databases → migrate → prod seed → sandbox
    toon (make-sandbox.ts from the local DB, written to logs/dev/sandbox) → devpromo → API → test user; subcommands `up`,
    `seed`, `sandbox`, `media [--apply]` (missing media from bunny.net, never deletes / overwrites), `status`, `stop`,
    `reset`, `test`, `console`, `sql`, `adminer`, `version`; `SCUTOONS_DB_NAME=scutoons_<x>` per worktree.
  - **No file sources:** catalog JSON → test fixture `server/tests/fixtures/catalog/` (CatalogSeed test-only;
    `catalog:seed`, `catalog:export-sql`, file fallback, featured default list, `/admin/catalog/respawn` removed);
    shared/content/seed + legacy + the one-off migrate-*.ts / slice-sizes.ts deleted (TS tests: fixture
    `server/tests/fixtures/content/`). Contract suites + `composer test:contract` removed (`composer test` = PHPUnit);
    CI = one MariaDB job (PHPUnit + PHPStan).
