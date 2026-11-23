<!-- status: planning notes for lap 2 (epic #626), collected 2026-11-24 in the planning session – nothing decided until the plan (lap2/PLAN.md) is agreed; no tickets from these notes -->
# Lap 2 – planning notes

The owner's words (dictation typos fixed), then what the review found. Input for the plan, not decisions yet.

## The owner, 2026-11-24
1. **The dashboard does not replace the menu.** *"The dashboard app switcher style overlay does not replace the menu on the right. The idea is to have that overlay come alive with Alt Tab or Ctrl K or Command K as you have now. And then it doesn't need the search tool, since we got a search feature by itself."*
   → the menu button opens a menu again; the overlay is the app switcher only (Alt Tab / Ctrl K / Cmd K), without the launcher field; search is its own feature.
2. **The edit-language switcher** *"lives in the menu and it can move to the top bar if a person is for example craft translator."*
   → in the menu by default; in the bar for the translator craft (a craft shapes the bar).
3. **Map** *"is broken"* / *"missing"*: the page has 0 px width; the workspace row hides Map without a working "more" menu.
4. **Views** *"are not individually activatable so I can not design workspaces."*
5. **One look:** *"library and other places use different color schema. We have to make this really uniform also between deck editor and creator."*
6. **Scenes** *"still have nothing to edit but publish. So perhaps we need a design for this page. Also there needs to be the link to the deck … we assume it is installed on the same system so it can launch and it will need to launch with that scene on. Same goes for rigs."*
7. **Catalog and Library:** *"let's redesign also our concept of catalog and library and how they are intertwined."*
8. **Wording:** *"many pages there are not following the translation dogma"* (DESIGN.md "Creator wording").
10. **The deck too:** *"implementation means also to implement it in the deck."* → every lap-2 item has its creator and its deck side; the plan names both (the deck side is the deck orchestrator's).
11. **Design from what is there:** *"please design the views based on what is there atm, so we don't fuck up things, and stick to our design more closely. There was plenty of interpretation going on last night."* → the pictures start from screenshots of today's creator and deck, change only what a note asks, and mark each change; where a note leaves something open, the picture asks instead of choosing.
12. **The deck's dashboard follows the creator's** (2026-11-23, on #830): *"the dashboard is weird. i said it should use the same design dogma as the creators"* · *"lap two needs design first"*. #830 stays on its branch (49009f380) until the plan. Open: whether the deck can show Needs review / Not published.
9. **How to work:** *"plan first carefully before creating tickets"* · *"you work much better when a lap has a specific plan and designs to follow."*

## The review of lap 1 (2026-11-24, code read against the 64 points)
- Built: ~50 of 64, mostly with tests. Partly: Generate canvas (stub), own workspaces (no rename / duplicate). Missing: the layout model in the Rust core (point 22), keyboard tiling, the pad's Y; the deck (#830) not built.
- Map 0 px: `meta-page.svelte` `.content{margin:0 auto}` inside the flex `.fill` page – one line (`margin:0; width:100%` on `.fill .content`); check flow / storyboard too.
- Workspace row: the "more workspaces" menu shows only when a measurement finds hidden tabs, and a CSS tie keeps the pill from shrinking – the design draws the chevron always.
- Views: works (four states, Save / Back / Save as new), but only on the dashboard's front card; gone when nothing is open; lists only the views the page fills. Not built from crafts-v2 `save`: the workspace tab's own menu, the changed dot, Ctrl S, Rename.
- Look: 59 local style rules in 27 files (worst: the 3D model canvas, the Map tool strip, the Library); raw selects (settings language, library toolbar); solid blue "Published" badges (status/chapter-badges).
- Wording: de = en (2575 keys); 36 messages end in "…"; ~47 hard-coded texts (most in the kit page).
- Scene page: read-only + Publish, no deck link anywhere. Catalog and Library: separate, no shared concept.
- Built and waiting for the plan: #842 thumbnails (branch feature/842-thumbnails 0d6f9b476 – workspace snapshots on the cards, stored stills, kit Thumb).

## Open questions for the plan
- The lap's goal: finish lap 1 first, or take the new concepts in.
- Where Views / Save workspace live inside a workspace (the tab's own menu as drawn, a bar icon, both).
- Which design pictures first; the deck's side in parallel or after.
- What the menu holds again (notes 1, 2) and what a craft may move into the bar.

## The design base (today's screens, `now/`)
creator-1-character (bar + panels, day) · creator-2-dashboard (the menu button's overlay) · creator-3-views (Views, only on the front card) · creator-4-scene (scene page, read-only) · creator-5-map (0 px stage) · creator-6-scene-night-owner (the owner's circled screenshot). Deck: `deck-*.png` shot by the deck orchestrator from main as is. **Today's deck has no Rigging or XR workspace:** on main they are Build (stage) · Module · Show · Home (+ asset / media); Rigging, XR and the creator-model bar are #830, paused halfway, not on main.

## Pictures to draw (from the base, each change marked, nothing else touched)
1. The ☰ menu and the switcher apart. **The menu's contents are the owner's list from 2026-11-23** (workspace-thoughts.md points 34, 36, 39 – not a new proposal): the Views list with Save workspace, Search (for those who don't know the keys yet), Settings, Docs, the build (version, account, load meters) – plus, from today, the edit language. History is a view; the bar's right end stays undo · redo · Publish · account · menu, icons only. Alt Tab / Ctrl K / ⌘K open today's dashboard as the switcher, without its search field (point 38 "the menu button is the dashboard" is replaced by today's note 1). Creator and deck.
2. A craft moves a control into the bar: the edit language in the bar for the translator.
3. Workspace editing reachable inside a workspace (where: open – the owner decides).
4. Map: today's page with its stage width back (a fix, no design change).
Later, after these: the scene page + open in the deck; Catalog + Library; the one-look sheet.

## Decided in the planning session (2026-11-24)
- **B1 – Views live only in the ☰ menu** (owner: *"Only the ☰"*), not on the workspace tab's menu. Still open: whether the workspace tab keeps its own menu for Save / Rename / Back to the last save (B2).
- **A3 – no Views on the switcher's cards** (owner: *"the views there – that is stupid. Redraw"*).
- **One switcher for both apps** (owner: *"workspaces from the deck or the creator should be shown vice versa … those workspace links will open the respective deck if active, jump into the workspace and the last content it had"*) – how: DECK-LINK.md.
- **Short placeholders** (owner: *"too long placeholder text – make it simple"*): a search field says "Find", no list of what it finds.
- **Decided: the creator becomes an Electron app in this lap** (owner: *"perhaps we can also plan to put the creator into an Electron app, since we are slowly tying it more into a desktop editing experience"* · *"Electron for this lap"*). Electron ships its own Chromium (as the deck OS does). For the plan: Windows, macOS and Arch; its own `scutoons-creator://` and one instance; the deck link through the two apps on the machine (DECK-LINK.md); files and local folders; signing and updates beside today's static deploy (whether the web creator stays: open). No earlier decision against it (2026-10-16 "PWA, not native yet" is the reader's).
- **The user menu opens under the account icon; the swipe up was meant for the switcher – and is dropped** (owner, on A5: *"explain why this should go down there?"* · *"I wanted the app switcher dashboard to be swiped up. But also this behaviour interferes with system defaults so it's very unlikely feasible"*). Point 56 was read wrong last night (the user menu instead of the switcher). The swipe up from the bottom edge collides with the systems' own gestures (iPad home, Windows / Android taskbar), so no swipe: the switcher opens with Alt Tab / Ctrl K / ⌘K, the user menu drops down under the account icon like the ☰ menu.
