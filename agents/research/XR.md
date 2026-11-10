Status 2026-11-10: research, nothing built (#482). §1 and §2 are read from the code of both projects; §3 and §4 are proposals; §5 is decided by the owner (shared core, the unit is an Experience, scenes in the catalog, three.js accepted, image tracking only); §6 is open; §7 a proposed order.

# XR — taking over OSCT's XR methods into SCUTOONS

Owner 2026-11-10 ([xr], decisions/deck.md, five lines): XR targets are built in and the jump type XR stays; XR items are offered; OSCT's design (data, table structure) is taken over fairly directly, its methods and not its content; XR is a mode of its own in the browser, not a deck; an XR scene can appear in a story, be opened directly from outside (the vinyl record), activate objects in a room, or sit on an image a deck shows on a screen; VR scenes are walkable deck scenes, later, out of scope. Open: the unit that groups XR content (toon, chapter, or an order of its own by format: book, show, space).

Sources: a read-only survey of both repositories on 2026-11-10; OSCT is `C:\Development\osct\osct`. File references are in the survey's report on #482.

## 1 · XR in SCUTOONS today

- **The jump type exists, its page does not.** `JumpToTargetType.XR` (`shared/content/src/enums.ts`); an XR jump stores `{ type: 'xr', value }`; the viewer builds `/xr/<value>` (`viewer/src/lib/visual-novel/jump-to.ts`); there is no `/xr` route. The address book lists no XR entries ("no catalog").
- A QR code type `xr` exists (admin catalog, `qr/src/Landing.php`): it redirects, nothing more.
- **WebXR is used once, for something else:** the handheld camera (#469) streams a phone's pose with `immersive-ar`.
- The viewer has no three.js; the deck's 3D is wgpu (native and wasm). Nothing in `deck/` renders XR.

## 2 · How OSCT does it

| Part | OSCT |
|---|---|
| Unit | **The Book is the only container.** Everything is keyed by book; nothing is shared between books. |
| Inside a book | Spreads (a double page, the cover, the back: a page range) › entries (glossary, video, text, link; on a page) › at most one **target** per entry › an **entity tree** anchored on the target (1 unit = the target's width). |
| Target | An image: the printed picture. Nothing is precompiled; the image becomes a 480×640 grey frame at run time. 10 per spread, 4 tracked at once. |
| Scene logic | Behaviour rules without code: events found · lost · tap · unlocked; actions play · pause · stop · show · hide · set · emit; variables scoped spread · stage · session · book. Plugins in TypeScript add layers, node types, materials, passes. |
| Environment | A world around the reader for the whole book, rotation only (gyroscope). Not a container. |
| Runtime | three.js and the open-source 8th Wall image-tracking engine (MIT, on the device, no service, no cost). No WebXR. Android and iOS browsers. |
| Not there | World tracking (the SLAM binary's licence rules it out), rooms, anchors, an exhibition mode. 6DoF would need WebXR `immersive-ar`: Android Chrome only, and it stops image tracking. |
| Storage | Typed tables per kind keyed `(book_id, id)` with the object in `data_json`: books, spreads, entries, targets, entities, plugins, assets, changes, book_versions. The rows are the draft; publishing writes an immutable build to a content store. |
| Editor | SvelteKit: tree, a stage running the same engine over a simulated page, inspector, library, code. 20k lines. The engine is 9k, the reader 11k, the server 6k. |
| Its own plan | A white-label core ("scutoons-animus-ar") with **SCUTOONS as the second instance**; flows (goto between spreads and books) built on SCUTOONS's VN logic; spread formats beyond page, cover and back: **wall, object, façade** (`agents/STRATEGY.md`, the owner 2026-10-28 and 10-31). |

## 3 · The four ways in, against what the method can do

| Way in | With OSCT's method | Open |
|---|---|---|
| From a story | The XR jump leads to the XR mode's page for a scene; the mode needs building, the jump exists. | — |
| Directly from outside | The same page by link or QR code; the QR type `xr` exists. | — |
| Objects in a room | Works where the object carries or is a flat image (a plaque, a label, a poster, a sleeve): an image target. A sculpture as such cannot be tracked: no world or object tracking. | whether plaques are enough for an exhibition |
| An image on a screen | An image target like a printed one. Never tried on a screen (brightness, moiré, refresh, the picture moving). | **a test with a deck showing a still is the first thing to do** |

## 4 · Proposal: the unit and the mapping into the catalog

**The unit is the Book, as in OSCT (named Experience by the owner, §5), and "format" is what a spread is.** A book is an ordered set of surfaces; a surface carries targets; a target carries a scene. OSCT already plans the surfaces beyond the page.

| Surface (spread format) | The target on it | The owner's case |
|---|---|---|
| page, cover, back | a printed picture | a printed book, a record sleeve |
| wall, object, façade | a plaque or picture at the thing | an exhibition |
| screen | a still a deck shows | XR on a scene of a show |

So book, space and show are not three orders: a space is a book whose surfaces are walls and objects, and a show's screen is a surface a book can name. Printing stays possible because the book keeps pages.

**Where each OSCT kind lands in SCUTOONS**

| OSCT | SCUTOONS | Note |
|---|---|---|
| Book | a new catalog document kind, `book:<id>`, optionally of a toon | versioned like every document (`entity_versions`), not OSCT's row tables: the one deliberate change |
| Spread, entry, target | parts of the book document | as OSCT's contract has them |
| Entity tree, behaviour, passes | the XR scene, inside the book document | OSCT's contract taken as it is |
| Asset | SCUTOONS's assets | exists |
| Plugin | later; not in the first step | the editor sandbox is its own piece of work |
| Behaviour variables | story variables | the expression syntax is already shared |
| Unlocking an entry | giving a catalog item | "XR items": an entry can give an item, and an item or product can name the scene that shows it (the record) |

**In the address book (#477):** `book:<id>` and `book:<id>.<target>` as references (no receiver, so no address); the jump type XR picks one in the one field.

**Stories:** a chapter jumps to a book's target (`xr`), and a scene's behaviour can jump back into a chapter. One flow, as OSCT's own plan asks.

## 5 · Decided by the owner (2026-11-10, [xr])

1. **The code is shared.** OSCT's core becomes a common core both projects use ("an important step of development"). So nothing of OSCT's engine or editor is copied into SCUTOONS; the core has to be cut out of OSCT first (its phase B), and SCUTOONS is its second title.
2. **The unit is an Experience.** A general word; book, space and show are its formats, and a poster or an event is simply a show. §4's "Book" reads as "Experience"; the surfaces stay (page, cover, back · wall, object, façade · screen).
3. **A scene belongs to the catalog,** assigned to the toons in some form. So a scene is not owned by its experience as in OSCT: the one place where OSCT's structure is not taken over as it is.
4. **A second 3D engine in the browser is accepted** (three.js beside the deck's), knowing it has to be maintained.
5. **Exhibitions: plaques at the objects only,** in effect codes one dials in with. XR is image tracking only for now. No room tracking.

## 6 · Open

- What the parts inside an experience are called and how they are cut. The owner: pages; "windows, scopes"; a right wall, a left wall, the background loading by itself. And whether "scene" is the word for the XR scene, beside the deck's scenes and a chapter's groups.
- How a scene is assigned to toons: a field on the scene, or the experience carries the toon.
- The screen test (§3): a deck shows a still, a phone tracks it with OSCT's engine as it is. Not done.
- The list of XR jumps that exist in chapters today. Not made.

## 7 · Order of work (proposal)

1. **In OSCT:** the core cut out as a package (engine, contract, behaviour), with OSCT as its first title. This is OSCT's own phase B and its orchestrator's work.
2. **In SCUTOONS, not waiting for 1:** the catalog document kind `experience` with its schema and generated types ([contract]), scenes as catalog documents, experiences and targets as references in the address book (#477), the XR jump's field offering them, and the reader's page `/xr/<experience>/<target>` as a shell that says what it would open.
3. **Then:** the core loaded on that page, one experience authored as a document, the screen test, an entry that gives an item.
4. **Last:** the editor, from the shared core.
