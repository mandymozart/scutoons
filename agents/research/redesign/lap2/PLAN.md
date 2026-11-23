<!-- status: draft for the owner's review, 2026-11-24; the deck side read and corrected by the deck orchestrator the same day (marked "deck:") – the plan of lap 2 (epic #626), creator and deck. Not agreed yet: no tickets are filed and no code is written from it until the owner has reviewed it. Decisions: DECISIONS.md. Screens: SCREENS.md. The owner's words: NOTES.md. -->
# Lap 2 – the plan (draft)

## The goal
One look and one shell for the creator and the deck, as decided on 2026-11-24: the approved kit everywhere, the same bar, menu and switcher in both apps, the two apps opening each other, and the Show's events picked at the row. A design and UX round.

## The limits
- **No fundamental refactoring.** The creator inside the deck's wasm (#764) and the spores canvas in the creator are not in this lap.
- **Designs before code.** Every package is built from a picture that is drawn in the approved kit first.
- **Both apps.** Every package names its creator side and its deck side. The deck side is the deck orchestrator's.
- **Nothing a decision does not cover.** A package with an open point waits for your word or is cut from the run; no worker chooses for you.

## The packages
Each becomes one or a few tickets once you agree. Sizes and costs are set with the tickets.

### P0 · The pictures (first, no code)
Redraw the set of SCREENS.md in the approved kit with today's decisions: 32 screens are still in the old palette, 7 have the right shape and old colours. Night and day.
- **Follows:** kit-d.html, RULES H4, H10, H11, H13, H14, H15, DECISIONS.md.
- **Done when:** every screen marked old or shape in SCREENS.md is current, each change against today numbered, nothing drawn that no decision covers.

### P1 · The kit
- **Deck (the source):** palette D in deck-kit's tokens – no accent, no lime, no green switch; the see-through grounds; the colour-blind token set; tabs and panel titles without a border at rest; the signal table unchanged; the outlet types' colours as a table beside it.
- **One shared source of truth for the kit, in a prominent place of the code base** (your word, 2026-11-24): the tokens and each component's states are written once; the deck's UI crate and the creator's UI kit both come from it, checked in deck:ci and the style check so they cannot drift. Today they are kept in step by hand, and the approved kit is only a research page. **Deck: a neutral file, in a top-level `kit/` folder** (`kit/tokens.toml` and a README of the components and their states). It is cheap there: deck-kit already reads a `tokens.toml` on top of its built-in values. The tokens' defaults move out of the Rust code into that file, deck-kit builds it in as its base, and a small script generates the web tools' CSS from it – small. The components' states (a tab's rest, hover, dragged …) become named entries in the same file, done component by component together with the restyle – medium. **Decided 2026-11-24: the `kit/` folder.**
- **A kit page in each app, built from the real components**, is the reference to look at and refine; kit-d.html moves out of the research folder to that place.
- **Creator:** the tokens generated from the deck; the kit's components to the new look (tabs, chips and badges, icon buttons, the primary button, focus, selected rows); the colour-blind mode as a setting – in the deck's settings too; the kit page.
- **Size of the change in the web tools:** the accent is used in 201 places, lime in 36, the green in 23.
- **Done when:** the kit page in both apps matches kit-d.html, the generator runs in the checks, and the style check fails on any colour outside the tokens.

### P2 · The shell
- **The bar:** item tabs · workspace row with its ⌄ always · undo, redo, Publish, account, ☰ as icons.
- **The ☰ menu:** Views with Save workspace · Search · Settings · Docs · the build · the edit language. Views with the four states for every view, only here.
- **The switcher:** over the blurred bar, stage and panels; no bar, no search field, no Views; a maximum width; one list for both apps in the same order.
- **A panel taken full:** a box close to the window's edges over the blurred workspace.
- **Search** as its own overlay. **The user menu** under the account icon. **The translator's bar** with the edit language.
- **Deck:** the same bar, menu and switcher (#830 continues from its branch); the Show's Cues, Problems, Who, Session, Vote and Demos become views; Home stays when nothing is open, built like the creator's dashboard (your review of 2026-11-23). #830 merges main first.
- **Blur in the deck** (deck: decided there): a still of the frame, taken when the overlay opens, blurred once and drawn under it. It costs nothing per frame and the projector's output is not touched. **A known limit:** during a live show the operator's stage preview stands still while the switcher is open. You said: try it; "reduce motion" turns the blur off as it turns the animations off, in both apps.
- **The Show has three more sheets today** – Flash, Scene and New show (and "fires on", which P7 replaces). What each becomes is open.
- **Fixes with it:** the workspace row that hides Map; Views reachable only from the dashboard.
- **Done when:** A1–A9, B1, B4 and D1–D4 look and behave as drawn in both apps.

### P3 · Thumbnails (#842, built and held)
- The switcher's cards as full captures with the text over them.
- Stills for scenes and places, and preview pictures for videos, made in the background, stored once and shared across systems.
- **Deck:** takes a scene's still once it has settled and uploads it. It can already render a still without a window. The upload runs under the signed-in member, so a deck that is not signed in makes none. What "settled" means is set on the ticket.
- **Done when:** the switcher and the Library show real pictures for workspaces, scenes, places and videos, and a second machine gets them without making them again.

### P4 · The apps open each other
- `scutoons-deck://` registered on Windows, macOS and Arch Linux; the last workspace and item kept per device on the server (#838 is built).
- A card of the other app in the switcher opens it there.
- "Edit in deck" / "Open in the deck" on assets, scenes, rigs and modules in the creator.
- **Deck:** opens on the workspace and item it is handed; its own cards for the creator's workspaces.
- **Deck: there is no installer for Windows or macOS today.** The deck is packaged for Arch only; the X1 runs the deck's own system image, which has no link handler. The link on Windows and macOS therefore needs installers first – a package of its own, or this package names only Arch. Open.
- **"Open in the deck" on a machine without a deck** needs an answer (a download link?). Open.
- **Done when:** A3b and D8 work on one machine with both apps installed.

### P5 · The creator as an Electron app
- A thin wrapper over the same code: `scutoons-creator://`, one instance, Windows, macOS, Arch Linux. The web version stays.
- **Done when:** the installed creator opens, signs in, and P4's links work from it.

### P6 · Not twice
- "Import images" removed from the Map, the Catalog and Characters.
- The creator's Rig panel without Bones and Clips: the model's line and "Open in the deck".
- Images, sound, video and 3D in the creator as previews with "Edit in deck".
- **The Scenes workspace's own list of scenes goes** (its Tree panel); the Library filtered to scenes opens them.
- **The scene page:** the scene's preview covers the whole back, "Edit in deck" on it, the facts and versions in a panel (C5). The preview is the scene's still from P3.
- The Library's "To do" group goes: "Blank – wanted", "Missing versions", "Missing credits", and the asset details' "Show the families missing versions" (H17).
- The creator's module editor: see P9.

### P7 · The Show: events at the row (deck)
- The row's ⚡ slot opens Find under it, with the pills This toon · Show · Universe. No "fires on" sheet.
- A row may fire on several events; several rows may listen to one event.
- Sources: a story event, another row's fire, a vote's result. Place, device, jobs and assets are listed greyed.
- The tab "Nodes" is "Spores". The selected row has no thick left bar. The picker list gets its ground.
- **One change to the stored show:** a row holds several events instead of one. It is the only change to stored data in this lap. Deck: the reader takes both forms, an old single event reads as a list of one; shows are written in the old form until every deck has the new reader, so a deck on an older update never gets a show it misreads; the production shows move by a migration; the server's check and the browser's reader change with it.
- **A guard against loops:** a row fired by another row can come round (A fires B fires A) – one fire per row per pass.
- **Done when:** D3 behaves as show.html `?v=1` and spores-view.html `?v=1…3` show it, and a loop of rows stops after one pass.

### P9 · The module editor (found missing in the review)
No picture of a module open exists for either app. Today there are two editors:
- **The creator's** (Sound design, `/edit/module:…`): the tabs Parameters · Nodes · Rust · Assignment, the code in the browser, Problems, a Console, Derive, the module heard on the being with its meters and knobs. It still has the old look (a white tab strip).
- **The deck's** (the Module workspace): a working copy on the machine, the code in your external editor, built by the deck's own compiler, Problems, Parameters as scrub numbers, the module heard alone on the stage, Derive for a factory module.

Decided: modules are edited in the deck. Thought aloud: the creator's editor might have to be removed.
- **To draw first (P0):** the deck's Module workspace with a module open (D4b), in the kit; and what a module is in the creator once its editor is gone – a preview with "Edit in deck".
- **Waits for your word:**
  - Is the creator's editor removed in this lap, or only marked and left?
  - What only the creator's editor has: the Nodes and Assignment tabs, the Console, and writing code without the deck installed. Do they move to the deck, or go?
  - A published module is made by the compile service from the creator. With the editor gone, Publish stays in the creator's one Publish.
- **Not checked yet:** whether the deck already covers Nodes and Assignment in other views (asked of the deck orchestrator).

### P10 · The writing pages
Kanon, Notifications, Credits, Edit and Guide are redesigned for a good writing experience, not only moved to the kit.
- **First (P0):** capture today's five pages, then one draft of the writing page for you to correct – nothing is built before that.
- **Waits for your word:** what a good writing experience holds.

### P11 · The extended Library (LIBRARY.md)
The Library lists things and files; a thing is stepped into; an assignment shows as a place and as "used in"; a picker is the Library opened from a slot.
- **First (P0):** the Library's screens redrawn to it (C12, F1, the slot's pick, C1, C4, C5, C9, A4, and the panel wherever it shows).
- **Creator:** the Library lists things with their type beside the files; stepping into a character; places for a season's cast listing its characters; "used in"; the cast edited in the toon's season.
- **Deck:** the same Library, the same panel (decided).
- **In lap 2** (decided). It is the largest package: the Library learns to list things of eleven kinds beside the files, in both apps.
- **Types** (decided): Toon · Character · Scene · Show · Place · Item · Module · Instrument and rig · Generator, and for now Kanon texts and people.
- **Waits for your word:** the five consequences marked "to confirm" in LIBRARY.md, and how the type filter is drawn with fifteen types.

### P8 · Bugs and wording
- The Map's stage at 0 px.
- The 59 local style rules, the raw selects, the solid blue "Published" badges.
- Wording: messages ending in "…", hard-coded texts, the translation dogma on the pages that miss it.

## The order
1. **P0** – the pictures. Nothing else starts before them.
2. **P1** – the kit, the deck first because the tokens come from it.
3. **P2** and **P8** side by side. **P7** can run beside them: it is the deck's alone and needs the kit, not the shell.
4. **P3** and **P4**.
5. **P5**, then **P6**.
6. **P9**, **P10** and **P11** once their open points are answered.

## The night run
- Two sessions at a time across all projects, one worker after another, by `C:\Development\agents\ORCHESTRATION.md` and scutoons' own `agents/ORCHESTRATION.md`.
- The creator orchestrator briefs the creator side; the deck orchestrator the deck side and every push.
- After each merge: the checks, and screenshots in the chat.
- A package whose picture is not current does not start. A package with an open point does not start.
- What needs you is collected in one list for the morning, not asked at night.
- Nothing is tested on the X1 at night. Each deck ticket ends with what you look at on the X1 in the morning.
- Every deck push runs the deck's full checks, so deck merges queue behind each other, with creator merges in between.

## Not in lap 2
- The creator inside the deck's wasm, and the spores canvas in the creator (#764).
- Events from places, devices, jobs and assets firing a row – a plan of their own after this round.
- The Reception view and the network's top level as a built view (#763) – the drafts stand.
- Module, instrument and rig as one stored thing (MODULES-DNA.md) – the concept is not finished.
- Trees across workspaces (TREES.md) – not decided.
- XR at a place and Map as a view – owned by the scenes epic (#714, #482).

## Open before the run
These block a package. Each is in DECISIONS.md under its area.
| Point | Blocks |
|---|---|
| The four layer names; Search and dialogs on the overlay level | P2 |
| The switcher's maximum width (1160 px drawn) | P2 |
| The workspace tab's own menu | P2 (B2) |
| The switcher's foot; the deck's Needs review box; Help and Go to in the ☰ | P2 |
| A disabled primary button; the see-through grounds as redrawn | P1 |
| The outlet types' list and colours. Deck: four – page · stage · target · device ("light" renamed, so one colour covers DMX, MIDI and OSC, as the stored kind does; the protocol is a mark on the lane); no colour for sound alone or for a sub show. Three of the started colours sit close to a signal's (page and picture, stage and vote, target and address): either outlet colours stay on lane and column heads only, never on cables or ports, or they take hues the signals do not use; each needs a mark in the colour-blind set | P1 |
| The Show's Flash, Scene and New show sheets: a view, a dialog on the overlay level, or gone | P2 (deck) |
| Installers for the deck on Windows and macOS: a package of this lap, or the link on Arch only | P4 |
| "Open in the deck" with no deck installed | P4 |
| The module editor: is the creator's removed, and what happens to what only it has | P9 |
| What a good writing experience holds | P10 |
| The extended Library: its five consequences to confirm; the type filter with fifteen types (LIBRARY.md) | P11 |
| P0 as part of the night run, or drawn and reviewed by you before it | everything |

These do not block the run; their screens stay out of it:
- Catalog and Library as one concept (F1).
- Where the 2D-first / 3D-first preference sits (C4).
- The trees' open questions; the word "instrument"; MODULES-DNA.md points 2–7.
