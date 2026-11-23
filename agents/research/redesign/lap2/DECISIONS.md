<!-- status: for the owner's review, 2026-11-24 – every design decision of the lap 2 planning session in one list, sorted by area. A summary, not a new source: the owner's words are in NOTES.md and in agents/decisions/law.md and creator.md (2026-11-24); the rules are RULES H4, H10, H11, H13, H14, H15. -->
# Lap 2 – the decisions of 2026-11-24

One line per decision. **Open** lists what still needs the owner's word; **Thought aloud** is recorded but not decided.

## 1 · How lap 2 works
- Plan and designs first, tickets after. No ticket per remark.
- Every item has a creator side and a deck side.
- Designs start from today's screens, change only what was asked, and mark each change.
- Lap 2 is a design and UX round. No fundamental refactoring in it.
- No app code until today's designs are reviewed. Then one long night run.

## 2 · The look
- **Palette D – no signal hue.** On, focus, active and selected are the text colour. The one primary action is the pressed fill. Colour only says ok, warning, error. (H11)
- **Grounds are see-through**, never an opaque grey: the text colour at a low strength (control 6 %, hover 10 %, selected 16 %). Menus stay opaque.
- **Kit D is approved** (kit-d.html), all 14 points as drawn; the tab's hover border is the text colour.
- **The kit gets a prominent place in the code base and one shared source of truth** for the deck's UI crate and the creator's UI kit – tokens and components, to go to for reference and to refine.
- **Red and green stay** for ok and error. **A colour-blind token set** comes beside them (blue, yellow, vermilion), each state with a mark (✓ ! ✕).
- **The hue is taken out of the UI's own parts, not out of what it shows.** What keeps its colours: signals (ports and cables, the deck's signal table; kinds OSCT has and the table lacks are adopted), the mixer (its strips and meters), and thumbnails and pictures.
- **Tabs and panel titles:** no border at rest, the active one brighter, a rounded border only on hover and while dragged. (H4)
- **Icon buttons:** a standalone one is round and may stand without a pill; groups of icons and toolbars stay in their pill. (H4)
- **Badges** are small and delicate. Design pictures use the kit. (H13)
- **No info in the UI unless asked:** explanations are hover text, shown in the Info view. (H14)
- **Depth layers:** stage · surfaces (bar, panels, status line) · popovers (menus, pickers) · the overlay (the switcher, a panel taken full). Under the overlay everything is blurred, the bar included. Blur exists on that level only. "Reduce motion" turns the blur off too – one setting for animations and effects. (H15)
- **A panel taken full is a box** close to the window's edges (the margin panels have today), not the whole window; the workspace stays behind it, blurred.
- **The switcher's content has a maximum width**, centred.
- **No thick left bar** on a selected row. (H10)
- **Texts and placeholders are short.** A search field says "Find".
- **The tools do not flag what is missing.** The Library's "Blank – wanted", "Missing versions" and "Missing credits" go, in the design and in the code: a gap may be on purpose. (H17)

**Open**
- The four layer names are my wording – confirm or rename.
- The switcher's maximum width: 1160 px is drawn – confirm or give the number.
- Do Search and dialogs sit on the overlay level (blurred behind) or on the popover level (sharp)?
- How a disabled primary button looks.
- A quiet select for filters and sorts in a toolbar: no box at rest, a rounded border on hover and while open, as tabs have. The approved Select is boxed at rest (kit-d.html shows both).
- The see-through grounds as redrawn on kit-d.html: do they match what you meant?

## 3 · The shell
- **The ☰ menu and the switcher are two things.** The menu holds your list: Views with Save workspace · Search · Settings · Docs · the build (version, account, load meters) · the edit language.
- **Views live only in the ☰ menu.**
- **The switcher** (the name from now on) comes on Alt Tab / Ctrl K / ⌘K. No search field, no Views on its cards, no bar.
- **One switcher for both apps**, the same order in both. A card of the other app opens that app on the workspace and its last content.
- **Cards are real captures** of the workspace as last left, filling the whole card, the text over the picture.
- **The deck keeps Home** when nothing is open.
- **Search is its own overlay** (Alt Space / ⌘ Space).
- **The user menu drops under the account icon.** No swipe.
- **A craft shapes the bar:** the translator's bar carries the edit language.
- **The Show's own pill goes.** Cues, Problems, Who, Session, Vote and Demos are views of the Show workspace, listed in the ☰ Views. The bar's middle holds the workspace row and nothing else. (H16)
- **Resume across devices** (built, #838). Existing members are asked their crafts once.

**Open**
- Does the workspace tab keep its own menu (Save · Rename · Back to the last save · Save as new · Delete)?
- The switcher's foot (account, Settings, Docs): stays, or goes because the ☰ holds it?
- Does the deck's switcher show a Needs review box?
- Do Help and Go to belong in the ☰ menu?

## 4 · The two apps
- **The creator becomes an Electron app in this lap** – a thin wrapper over the same code. The web version stays for now.
- **The apps open each other:** a `scutoons-deck://` link, the last state kept per device on the server, no calls to localhost from the page. Windows, macOS, Arch Linux.
- **XR is a creator workspace**, not a deck one.
- **The spores canvas is the deck's**, shown in the creator through the deck's wasm – with #764, not in this lap. The creator's Generate stub stays until then.

## 5 · Who edits what
- **Asset editing is the deck's.** The creator previews and offers "Edit in deck" (images, sound, video, 3D).
- **Modules are edited in the deck** (and by MCP and an external editor).
- **Not twice:** the creator's Rig panel loses its Bones and Clips lists; it shows the model's line and "Open in the deck".
- **"Import images" goes** from the Map, the Catalog and Characters. Bulk comes through MCP; the Library places things into slots.
- **A scene in the creator has a preview** that covers the whole back of the page; "Edit in deck" goes from there. The page previews and links, it does not edit.
- **The writing pages are redesigned** for a good writing experience: Kanon, Notifications, Credits, Edit, Guide.
- **The extended Library lists things and files** (LIBRARY.md). A thing is a document with a type and an address – a character, a scene, a toon, a place, an item; a file is a picture, a model, a sound, a video.
- **A thing is stepped into.** A character's personas, looks and moods stay inside it, with the files it uses beside them.
- **A cast is characters assigned to a toon:** found in the Library, edited in the toon's season. Opening a character from there comes to the Characters workspace.
- **A list of characters or scenes beside the Library is redundant.** The Scenes workspace has no list of scenes of its own.
- **Thumbnails:** scenes, places and videos get preview pictures, made in the background and shared across systems (#842).

**Thought aloud**
- Trees: one per workspace, named like it, borrowable by other workspaces (TREES.md).
- A character is 2D-first or 3D-first, as a preference.
- Module, instrument, preset and rig are one mechanism; a spore is a module with another driver (MODULES-RIGS.md, MODULES-DNA.md).
- The creator's module editor might have to be removed.

**Open**
- **The module editor was never designed** – in neither app. The creator's (Sound design) is an empty frame in the draft; the deck's is drawn only with nothing open. Before it is drawn: is the creator's editor removed, and what happens to what only it has (see PLAN.md, P9)?
- What a good writing experience holds (C14) – your words, or a first draft from me to correct.
- The extended Library's consequences to confirm (LIBRARY.md): one picker instead of five sheets; the Catalog's and the Toon's top-level lists go; the Cast tab becomes "cast in"; trees do not travel; which types it lists; how much is built in lap 2.
- Trees: the Library's "Items to open" against the trees; a borrowed tree – jump or reference.
- Where the 2D-first / 3D-first preference sits.
- Does the word "instrument" stay? MODULES-DNA.md points 2–7.

## 6 · Events and spores
- **One ⚡ slot on everything that listens** – a show's row, a scene's listen spore, a generator's run, an XR rule.
- **Two ways to connect:** press the slot and Find, or weave the cable in the Spores view. Both write the same line.
- **"Fires on" needs no tab or sheet.** The picker opens under the slot; pills widen it: This toon · Show · Universe.
- **Several listeners can hang on one event. A row may fire on several events.**
- **Sources that fire a row in this lap:** a story event, another row's fire, a vote's result. Place, device, jobs and assets stay greyed – they get their own plan after this round.
- **Any reader's event counts.** A filter to allow some automatically can come later.
- **The tab is "Spores"** (the deck's "Nodes"). Spores are just "spores".
- **Events are addressed `event:<id>`**; readers never get the list (#707).
- **The demo is how triggering works** (spores-demo.html): a found target → relay → row → decks; a sheet changed → the generator runs → succeeded → the slot and a row.

## 7 · Found broken (fixed with the lap)
- The Map's stage is 0 px wide.
- The workspace row hides Map; its "more" menu only shows after a measurement.
- The deck's "fires on" list has no ground; the stage shows through.
- 59 local style rules in 27 files; raw selects; solid blue "Published" badges.
- Wording: 36 messages end in "…", about 47 hard-coded texts, German equals English.

## 8 · Built and held
- **#842 thumbnails** (branch `feature/842-thumbnails`): needs the full-bleed cards and the stills for scenes and places.
- **#830 the deck on the same model** (branch, paused): continues with this plan.
