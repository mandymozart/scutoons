Status 2026-11-10: research and concepts, nothing built and nothing to be built yet (#482; the owner: "we're still collecting concepts"); owned by the XR session (branch `wip/482-xr-experiences-7f10f7`), the owner's statements from here on in decisions/xr.md. §1 and §2 are read from the code of both projects; §3 and §4 are proposals; §5 is decided by the owner (OSCT's code copied in, lazily loaded; the unit is an Experience of type book, show or poster; a scene belongs to its experience; three.js accepted; image tracking only); §6 is open; §7 the order, agreed. Plaques and an image on a screen are tested by the owner and work (decisions/xr.md): no test of them is planned.

# XR — taking over OSCT's XR methods into SCUTOONS

Owner 2026-11-10 ([xr], decisions/deck.md, five lines): XR targets are built in and the jump type XR stays; XR items are offered; OSCT's design (data, table structure) is taken over fairly directly, its methods and not its content; XR is a mode of its own in the browser, not a deck; an XR scene can appear in a story, be opened directly from outside (the vinyl record), activate objects in a room, or sit on an image a deck shows on a screen; VR scenes are walkable deck scenes, later, out of scope. Open: the unit that groups XR content (toon, chapter, or an order of its own by format: book, show, space).

Sources: a read-only survey of both repositories on 2026-11-10; OSCT is `C:\Development\osct\osct`. File references are in the survey's report on #482. The shared core the owner postponed is researched in XR-SHARED-CORE.md; its §5 (advice for the copy) is applied before slice 2 and 3.

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
| Objects in a room | Works where the object carries or is a flat image (a plaque, a label, a poster, a sleeve): an image target. A sculpture as such cannot be tracked: no world or object tracking. | — (owner 2026-11-10: tested exhaustively, down to very small images; plaques it is) |
| An image on a screen | An image target like a printed one. Owner 2026-11-10: tested, works really well. | how a Show's screen surface names what a deck shows on it (implementation, not a test) |

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

## 5 · Decided by the owner (2026-11-10, [xr], as revised the same day)

1. **OSCT's code is copied; the shared core is dropped for now.** The editor is integrated into the creator and the engine into the reader, as a module that is loaded lazily, without fail, so readers who never open XR carry none of it.
2. **The unit is an Experience,** and it has a type. Three to begin with: **Book, Show, Poster.** A record (vinyl) is a special kind of Book.
3. **Inside a type, its surfaces.** A Book has cover, back and spread. A Show has walls, screens and tabletops, freely scalable (the editor for scaling comes later). A Poster is one surface.
4. **A scene belongs to its Experience,** exactly as in OSCT a scene belongs to its book. "Scene" is the word.
5. **A scene is tied to toons through the event system,** like a location with jump-to. XR scenes go into the address book, and a scene's targets (its parameters) can be addressed through the address system.
6. **A second 3D engine in the browser is accepted** (three.js beside the deck's), knowing it has to be maintained.
7a. **Svelte is the primary frontend.** OSCT's editor is Svelte 5 (SvelteKit 3) already; its reader is plain TypeScript with custom elements; its engine is plain TypeScript. So the engine is copied as it is, the reader's XR interface around it (scan mode, entry sheets, unlocking) is written as Svelte components in the viewer, and the editor is carried into the creator.
7. **Image tracking only for now.** Exhibitions: plaques at the objects, in effect codes one dials in with. No room tracking.

8. **Surfaces are flexible** (owner 2026-11-10, decisions/xr.md): a surface is created in a certain size, named, and used later; various formats, certain wall sizes. Front, back and gatefold are surfaces like all the others.
9. **A surface has a place on the editor's stage**, the demo scene, so the example reads well: a book's surfaces are bound side by side (spreads); an exhibition's walls stand together, three or four in one, as rooms; posters hang singly or lined up one after another on a poster wall.
10. **What an entry does in SCUTOONS:** it rather unlocks 3D views, videos and items, or jumps to a scene: interactive stories with SCUTOONS's existing toolset. The entry without a target that is only read in a list (OSCT's glossary) stays in for now; whether SCUTOONS's reader needs it is to be thought about.

Drafts shown to the owner on 2026-11-10 (the XR session's notes folder, `482-xr/`): 2 the experience document · 3 the hierarchy, stored flat and shown stacked in the editor (the owner: good) · 4 the stored shape, A or B.

## 6 · Open
- **The stored shape of an experience** (draft 4, XR-SHARED-CORE.md §5 and §8): A, OSCT's configuration kept as it is inside the document with SCUTOONS's fields beside it (the research's advice; the tree is then surface › entry › target › scene, as OSCT's), or B, SCUTOONS's own four lists (draft 3; surface › target › scene, the entry beside). Not answered.
- Surfaces and the stage (points 8 and 9): whether a named size is kept as a format to reuse across surfaces, and whether a room is a stored group of surfaces or only their arrangement on the stage. A draft of it (5) is not drawn. OSCT's stage today: one book lying on a desk, leafed spread by spread (`editor/src/lib/stage/mock-stage.ts`).
- From draft 2, not answered: whether a record's disc label is a surface; whether the jump's field offers scenes beside experiences and targets.

- How a Show's screen surface names the still a deck shows (the tracking itself is tested and works, owner 2026-11-10: "It's more about the implementation").
- What "addressing a scene's parameters" needs: by decision [address] an address is real only where a receiver exists. The reader's XR mode would be that receiver; which parameters a scene exposes follows OSCT's behaviour rules (`set`, `emit`).
- ~~The list of XR jumps that exist in chapters today.~~ Made 2026-11-10: none. Production's flow (`read_flow`, all toons) has no jump to XR and the local seed has none; the QR code type `xr` redirects to the code's own `redirect_to`. The URL shape is free.

## 7 · Order of work

Not started: the owner builds nothing yet (2026-11-10). The order below stands for when the build is called. Ready for it on the branch `wip/482-xr-experiences-7f10f7`: its own packages and test database (`scutoons_xr482_test`); baselines green for the viewer, the creator, PHPStan, core-only and the server contracts.

1. **The data, in SCUTOONS's own shape:** the catalog document kind `experience` (type book · show · poster; surfaces; targets; scenes; entries), its schema taken from OSCT's contract, types generated ([contract]); experiences, their surfaces' targets and scenes as references in the address book (#477); the XR jump's one field offers them; the reader's page `/xr/<experience>/<target>` as a shell that names what it would open.
2. **The engine in the reader:** OSCT's engine copied in as a lazily loaded module on that page; one experience of type book authored as a document; an entry that gives an item.
3. **The editor in the creator:** OSCT's workspace (tree, stage, inspector) for an experience.
4. **Later:** plugins, the Show's scalable surfaces in the editor, scene parameters as addresses.
