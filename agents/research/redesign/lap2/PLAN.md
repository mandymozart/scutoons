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

### P0 · The pictures (done 2026-11-24, no code)
The set of SCREENS.md is redrawn in the approved kit with the day's decisions, each screen checked for leftover colours: draft.html `?s=<id>`, the shots in `shots/`.
- **What is not drawn:** the screens marked "words" in SCREENS.md (the workspace tab's menu, the module editor in both apps, the writing pages); the eight feature drafts brought in from other sessions have their palette moved, not a redraw.
- **Rule for the run:** a package is built from its pictures. Where a picture is missing, the package does not start.

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
- **Deck:** the same Library, the same panel (decided). Deck: its Library already mixes things and files – one entry with a typed address, 13 kinds (Module, Instrument, Rig, Scene, Character, Item, Place, Image, Audio, Video, Model, Animation, File). What it needs:
  - **One source for both apps** – the main cost. Today each deck workspace assembles its own list. A server endpoint gives the Library of a toon, things and files, typed; both apps read it. Without it the deck would rebuild the creator's grouping a second time. The endpoint is the creator's and the server's to define; the deck's reader with an offline cache is medium.
  - **Stepping into a thing:** its entry is flat today; it needs levels and "used in". Personas and moods are known to the deck; "used in" for things has no endpoint yet. Medium.
  - **New kinds:** Toon, Show, Generator, Kanon, Person – small each, but each needs a decision on what dropping it does and where it opens.
  - **Item subtypes** (recipes, products, editions, albums, home scenes): small once the endpoint delivers them typed.
  - **A video's frame** comes from P3's stills, not made per deck.
  - **Order:** after P3 and after the endpoint exists. Medium to large for the deck.
- **In lap 2** (decided). It is the largest package: the Library learns to list things of eleven kinds beside the files, in both apps.
- **Types** (decided): Toon · Character · Scene · Show · Place · Item · Module · Instrument and rig · Generator, and for now Kanon texts and people.
- **Decided with it:** one list, one picker (the five sheets go), the Catalog's and the Toon's trees without their top level, the Cast tab as "cast in", "By cast" listing characters.

### P12 · The Storyboard
Drawn from today's page: C3, C3b, C3c.
- **The kit look** on its steps, tools, Inspector and Timeline – comes with P1.
- **The viewport's ground** – with P8.
- **The Handheld panel** as drawn (C3b): the code to scan and the waiting state, Disconnect · Recenter · Record, the conditions and the trim's sentence as hover text.
- **Waits for your word** (a question card each): the Tree as the storyboard's own scenes and shots (C3); the shot's picture as a slot with "Open the generator" as the way into the Generate workspace (C3c).
- **Not drawn:** the steps Script, Layout and Export, the Variants view, the 3D view of a handheld shot, the panel once a phone is connected.

### P8 · Bugs and wording
- The Map's stage at 0 px.
- The Storyboard's viewport ground: white by night and dark by day today (found 2026-11-24 on Cat Pope · chapter 3).
- The 59 local style rules, the raw selects, the solid blue "Published" badges.
- Wording: messages ending in "…", hard-coded texts, the translation dogma on the pages that miss it.

## The order
1. **P0** – the pictures: done.
2. **P1** – the kit, the deck first because the tokens come from it.
3. **P2** and **P8** side by side. **P7** can run beside them: it is the deck's alone and needs the kit, not the shell.
4. **P3** and **P4**. **P11** follows P3: its tiles come from the stills, and both apps wait for its endpoint.
5. **P5**, then **P6**.
6. **P9** and **P10** once their open points are answered.

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

## Settled for the run
You approved these on 2026-11-24 ("yes, use them").
| Point | Settled as | Package |
|---|---|---|
| The four layer names | stage · surfaces · popovers · overlay | P2 |
| Search and dialogs | on the overlay level, blurred behind | P2 |
| The switcher's maximum width | 1160 px | P2 |
| The workspace tab's own menu | none in this lap; Views and Save workspace are in the ☰ | P2 |
| The switcher's foot | goes; the ☰ holds it | P2 |
| The deck's Needs review box | left out | P2 |
| Help and Go to | in the ☰ menu | P2 |
| A disabled primary button | the pressed fill, faded | P1 |
| The see-through grounds | as redrawn on kit-d.html | P1 |
| Outlet colours | four – page · stage · target · device – on lane and column heads only, never on cables or ports; a mark each in the colour-blind set | P1 |
| The Show's Flash, Scene and New show sheets | all three are dialogs on the overlay level – one-off tasks that end in a confirm or a save (decided after the deck orchestrator's check; the first default, views, was wrong). Cues, Problems, Who, Session, Vote and Demos stay views | P2 (deck) |
| Deck installers for Windows and macOS | not in this lap; the link between the apps works on Arch only | P4 |
| "Open in the deck" with no deck installed | the installed creator offers a download link; the web version just tries | P4 |
| Dropping a Toon, a Show, a Generator, a Kanon text or a person | nothing yet; each opens in its own workspace | P11 |

## Open – out of the run until you say
| Point | Package |
|---|---|
| The module editor: is the creator's removed, and what happens to what only it has | P9 |
| What a good writing experience holds | P10 |
| The Storyboard's tree as its scenes and shots; the shot's picture as a slot with "Open the generator" (two question cards on C3 and C3c) | P12 |

These do not block the run; their screens stay out of it:
- Where the 2D-first / 3D-first preference sits (C4).
- The word "instrument"; MODULES-DNA.md points 2–7.
