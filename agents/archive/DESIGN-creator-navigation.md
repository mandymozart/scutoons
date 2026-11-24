# DESIGN – "Creator navigation" (moved from agents/DESIGN.md on 2026-11-24, #901; frozen)

The creator's frame from 2026-09-30 (draft D 2026-10-20): the top bar with the path, the area menu, the page lists, the
AppShell and its docks. The creator left it on 2026-11-21 for the workspace host (#751, RULES H5); how it is now:
DESIGN.md "Creator navigation".

## Creator navigation (owner 2026-09-30; draft D 2026-10-20, research/CREATOR-NAV.md §2a, #92)

The creator is a hierarchy: **toon → season → chapter → group (scene) → slice**, next to the platform-wide areas
(characters, the map, the catalog, the Flow). Editing deeper must never change the frame around you.

- **One top bar, the same on every page:** the reader's square mark (a full-height cell, as wide as the rail) = home
  (the dashboard); the **path** (`Toons / Cat Pope / Season 1 / Chapter 2 / scene-14 / Slice 3` – the area first; no
  underlines, the levels above muted, `/` between: `Breadcrumbs` `plain`); the page's end (`crumbEnd`); the text
  language, the page's tools (Flow …), search, notifications. The bar's icon buttons are borderless squares (hover = the
  nav outline, on = inverted). Every crumb is a link with its full text – never a per-crumb ellipsis; when the path
  doesn't fit, the crumbs collapse from the left into ONE "…" (UI-CONCEPTS E.2); the last two never collapse. The
  season crumb is the chapter's real season (the season whose sequence lists it).
- **The menu by areas on the left** (`area-menu.svelte`), always there: Dashboard, Toons with every toon flat under it
  (never deeper), Characters (Moods, Wanted images are its tabs), Map, Scenes (#490), Catalog, Flow; the place you are in inverted
  (inside a toon: the toon); the account (→ Notifications) and the fold button at its foot. It folds **by hand** (the
  button, Ctrl + \, remembered) to an icon rail of the areas (unfold at its foot), and to the rail while a region is
  maximized. A page's own list (the chapter tree, a season's chapters, the map's lists – `usePageNav({ sidebar })`)
  sits beside the menu in the same region, its right edge the one `Divider`; its sticky head names the place (Chapter 2,
  Map) with a quiet « that folds the list by hand to a slim rail (remembered, `sc-creator-list-folded`); it hides while
  a region is maximized. The map's lists (Layers, Places, Routes, Regions) are all folded at first, then stay as the
  member left them (`sc-map-nav-open`, owner 2026-10-20).
- **A toon's pages are tabs** (`toon-tabs.svelte`): Stories (the toon page; a season's page belongs to it) · Cast ·
  Script · Credits · Settings. The chapter editor and the cover editor have their own frames.
- **Phones** (< 768px, like the reader): one row – mark (home) · the place's name as a small text button (in the chapter
  editor the group = the scene, not the slice – the editor scrolls through a group's slices; `path-menu.svelte`) that
  drops the whole path down, one level per row, indented (owner 2026-10-20); on a toon's own pages the toon picker
  instead – avatar · name · a small chevron, no frame; another toon opens the same tab (`toon-switch.svelte`, owner
  2026-10-21) · the page's end (the strip editor's
  ⚠ · Save · Publish · •••) · on the dashboard search + notifications · ☰ on the right. ☰ slides the menu in from the
  right (`AppShell drawerSide="right"`): the areas, the page's own list, "Recent" (this browser's last places,
  `recent.svelte.ts`); at its foot search, notifications, the text language and the page's tools.
- **Dashboard** (home, `routes/+page.svelte`; desktop three columns, a phone one): Continue (the last place, from
  Recent – opens it, a chapter at its slice) · Toons · Not live yet (every file with a draft newer than live, the
  newest first – the list Publish offers) · Waiting for you (the open reviews). An empty section shows nothing.
  **Toons** (`/toons`): every toon as one compact row (avatar · name · state). Both: the PhoneBar (Publish everything ·
  Reader).
- **Content** edits the selected item. **Bottom ActionBar** is the same component everywhere: Info (left-most),
  "up" (the parent level), page status / filters, secondary actions, primary action right-most.
- **URLs follow the hierarchy.** The existing routes map cleanly and stay: `/<toon>`, `/<toon>/season/<n>`,
  `/<toon>/chapter/<n>` (a chapter belongs to its season; the breadcrumb shows it), `/<toon>/cast|credits|toon`,
  `/characters?tab=wanted&id=`. Inside the chapter the selection is in the query:
  `/<toon>/chapter/2?group=scene-14&slice=3` (`group` = the group's id, else its number;
  `slice` 1-based; `?section=audio`). Deep links, crumbs and back/forward all work through the URL.
- **Keyboard:** Alt+ArrowUp = one level up (the parent crumb); Alt+ArrowDown = the first child in the page's list.
- **Names, not files:** the UI never shows storage keys (`chapter2.json` …): one mapping `entityLabel()` in
  `@scutoons/ui` → "Chapter 2", "Season 1", "Scene 14", "Toon settings", "Characters", "Moods", "Wanted images".
  **No version numbers anywhere** (owner 2026-09-30; the ids are store-wide row ids, kept in the API only): state chips
  read "de · Published", "Draft · not published" (newer unpublished changes), "Not published" (never published), with
  the last change's date in the title; the History panel lists versions by date, author and note (restore / publish
  stay); the admin content overview shows the same state words + "N versions · changed <date>".
- **Toon page** (owner 2026-09-30; 2026-10-12; 2026-10-20): no title row – the toon's settings are its Settings tab;
  "Season 1: <title>" links to the season editor; no "Chapters"
  heading; "+ New chapter" is the list's last row; the languages are one row of chips (the source filled, translations
  with ★ make-source and ×, an "+ Add" select); phones: the PhoneBar (Save · Publish); a chapter row (its title link covers the row) opens
  the chapter – no separate "Edit season" / "Toon settings" / "Open" buttons. **Statuses in lists are read-only badges**
  (published / draft / coming soon per language, `status-badges.svelte`: toon page, season editor, season sidebar);
  the status is changed only on the chapter page (Meta).
- **Help lives in Info** (`@scutoons/ui` InfoPanel via the ActionBar's "Info" toggle, closed by default, remembered),
  not above the fields. Info is a panel of the shell's right dock (agents/DOCKING.md): it sits next to Problems /
  History / Flow / Reader / a list's detail (outermost – 10.57), drags wider and folds to a rail like them. Buttons and links carry no decorative arrows; icon-only buttons (reorder, remove, prev/next
  slice) have an accessible label.

Implementation: `creator/src/lib/nav.svelte.ts` (`CreatorNav` context: route crumbs from the root layout, `extra`
crumbs + `sidebar` (the page's own list) + `fill` registered by a page with `usePageNav`), root layout = `AppShell`
with the top bar and the sidebar region (`area-menu.svelte` + the page's list), `toon-tabs.svelte`, `recent.svelte.ts`,
`up-link.svelte` in every ActionBar.
