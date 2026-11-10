Status 2026-11-10: research, finished once as the owner asked (#482); nothing built, nothing changed in OSCT. Read from OSCT's code at commit ebbadf3b (1.64.6), read-only. §5 is to be applied before the engine or the editor is copied (XR.md §7 slice 2 and 3). §8 is the owner's.

# One XR core out of OSCT, with SCUTOONS as its second title

The owner's task (2026-11-10, decisions/deck.md [xr]): find out what it takes to make one shared XR core out of OSCT that SCUTOONS uses as its second title, on the condition that the editor and the part in the reader are shared in it, not only the engine. The generalising itself happens with the next XR project; until then SCUTOONS copies (XR.md §5). OSCT keeps running as it is.

How it was made: a background agent of the orchestrator read OSCT's engine, contract, reader and editor and was stopped part-way (the owner: one research, not two; decisions/xr.md). The XR session took its note over, checked what it builds on against the code (marked ✔ below), and read the rest: the server, the CLI, the MCP, the interfaces, SCUTOONS's side. Not checked is listed in §7. OSCT is `C:\Development\osct\osct`; its own plan for a core is `agents/STRATEGY.md` there ("scutoons-animus-ar", phases A to C).

## 0 · The answers in short

1. **The cut line.** The engine and the contract are almost title-free already; what is OSCT's in them is small and named (§1). The reader's interface and the editor's book pages, shell and stage set are OSCT's. The server mixes generic editing rules with the book's pages in one file. The editor has no seam to its server: 25 of its files import the API client directly, and 16 more modules of its library call it.
2. **A core with editor and reader part** is six pieces: the contract, the engine, a framework-free reader controller, Svelte reader views, the editor's generic Svelte parts behind one backend interface, and the stage-frame page (§2). Two of them do not exist yet: the reader controller and the backend interface.
3. **Where SCUTOONS does not fit.** One real conflict: surfaces without pages, because in OSCT an entry finds its surface by its page number and the guard refuses a surface format it does not know. The rest is an adapter or a small addition (§3).
4. **The steps:** ten, the first worth doing anyway is the editor's backend interface in OSCT (§4).
5. **For the copy:** copy engine and contract unchanged with the commit recorded, adapt only through the seams they have, keep OSCT's shapes and names inside, SCUTOONS's words outside (§5). This bears on the shape of the experience document: the owner's choice in §8.
6. **Svelte:** the engine needs none and depends on nothing of OSCT's reader ✔. OSCT's reader does not have to move to Svelte for one core: both readers can run over the same engine and the same controller, each with its own interface (§6).

## 1 · The cut line, part by part

| Part (lines, non-test) | Generic method | OSCT's title | Tangled |
|---|---|---|---|
| **Engine** `engine/src` (8.9k) | the scene, entities, the tracker behind `IImageTracker`, the 8th Wall loader, passes, plugins, behaviour, look-around, the content loader. Imports only three, `@shared` and itself ✔; everything a host differs in is passed in: `ArSceneOptions` (tracker, graphics, progress, debug), `setDeviceStorage`, the loader's `baseUrl` / `bundled` / `fetch` / `assetsBase` ✔ | the unlock animation's gold palette (`scene/celebration.ts:50-57` ✔), the built-in world `cafe`, `utils/trigger-protocol.ts`, names with `osct` | the GLSL names `osctTime`, `osctUv`, `osctLayer<i>` (`utils/layers.ts` ✔) are part of the plugin API: renaming breaks plugins. Book-shaped but general: the window of ±1 neighbouring spreads, page turning |
| **Contract** `shared/` (3.7k) | entities, components, behaviour, passes, materials, plugins, placement, filters, lights, expressions (copied from SCUTOONS), the guards | `BookData` (author, publisher, print or digital), `EntryCategory`, the tutorial steps, `environment.world: "cafe"` required | **the page rule**: `SpreadData.firstPage/lastPage` and `EntryData.page` are required, the page finds the spread (`engine/src/content/game-config.ts:307-316` ✔), a target sits inside its entry, the guard refuses an unknown spread format (`shared/guards/game-config.ts:208` ✔), and a new format is a MAJOR change by OSCT's RULES #10 |
| **Reader** `client/src` (11.2k) | the glue to the engine (`components/ar-bridges/`, 0.4k), the managers for history, targets, spreads, the progress record, the preloader (together 1.5 to 2k of logic) | 38 custom elements bound to OSCT's store singleton, i18next and the gold design: scan mode, entries, header, tutorial, splash, settings, account; the links (`?osct=`), about 50 `osct-*` storage keys; the PWA manifest and the texts naming the book and the imprint | the logic lives in managers on OSCT's store, not in a controller of its own |
| **Editor** `editor/src` (20.4k) | the stage core with the plugin sandbox (3.4k), the panels: tree, entity, behaviour rules, passes, environment, library, code (2.9k), the code tab with Monaco (2.2k), the controls (1.5k), helpers (2.5k), the workspace page (0.9k) | the book on the desk (`mock-stage.ts`, `mock-tracker.ts`, `stage-layout.ts`: 2.2k), the book pages: covers, search, import with comparison, spread grid, publish (2.1k), the shell: sign-in, top bar, AI connections, admin (1.5k) | **no seam to the server**: `lib/api.ts` (0.56k) is imported directly by 25 files ✔ (4 pages, 7 book and shell components, 13 panels, the stage; 2 of them for types only) and by 16 more modules of `lib/` ✔ (the stores for content, entries, environment, library, plugins, session). Texts are hard-coded English (SCUTOONS's creator uses message files) |
| **Server** `server/api/src` (5.6k) | editing an entity, a target, passes, the environment; plugins; assets with versions (`<book>/<asset>/v<n>`, a replace relinks every use); the two-step import with comparison; draft assembly from rows; publish (build folder, `versions.json`, `latest.json`); the change and publish logs; removal with its impact; sign-in, sessions, CSRF, roles, tokens, progress per user and book | `LoginMail.php` (the book, its author, the imprint, gold colours), `Config.php` (the mail sender's name, `OSCT_*` variables), the cookie `osct_session`, the token prefix `osct_`, the newsletter opt-ins in `users` and `Auth.php`, the compat tutorial | **`Books.php` (1.4k) holds both**: `addEntry` needs a page and finds the spread by it (`:489` ✔), `addSpread` and `addCover` number pages, `renumber` shifts spreads, entries and printed targets (`:843`), print-first against digital-first, and `parse` puts an entry on a spread by page range. **One book per environment** is in the store's layout (`<env>/<build>/`, no book in the path) and in publish (`Books.php:1007-1012` ✔: 409 `other-book`). About 25 constants are PHP copies of contract rules, pinned by a test in `mcp/` |
| **CLI** `scripts/src` (3.2k) | the content build from YAML folders (entities, plugins, validation, the schema lock, media optimisation), the CDN layout with sync, purge and activate | `import-delivery.ts` and the Python layout scripts (the publisher's print PDFs by file name), the trigger protocol, the placeholder sounds | the storage client knows only bunny; one request sends OSCT's own host as its origin (`lib/content-sync.ts:275`); the build writes into the reader's source folder; spreads and entries are built by page |
| **MCP** `mcp/src` (1.7k) | 35 tools, each a thin wrapper over one or two routes of the editor API; the content model's description read from `shared/` | the server's name `osct`, the `osct://` resources, `OSCT_API_URL` / `OSCT_EDITOR_TOKEN`, the prompt texts | the tools' words are the book's (page, spread, cover, print-first) |

What the engine assumes of its page (to know when it is loaded into SCUTOONS's reader): it listens to taps on `document` and skips interface marked `[data-app-ui]` or real buttons; its camera canvas lies under the page (`z-index: -2`), so the page must be transparent on the XR route; it adds hidden media elements to `body`; it expects `window.XR8` from a script at a base URL the host passes; and it keeps one loaded configuration and one plugin registry per page. Plugins are uninstalled only by the editor's mirror (`engine/src/scene/plugins.ts:691` ✔); loading a second configuration leaves the first one's plugins registered, so the reader holds one experience per page load until that is changed.

## 2 · The core with editor and reader part

| Piece | From | What a title passes in |
|---|---|---|
| **contract** | today's `shared/`, generalised (§3) | nothing |
| **engine** | today's `engine/`, with its own package and three as a peer | the tracker, graphics and progress sources, device storage, the configuration (as a function that returns it), the 8th Wall base URL; new: the celebration's colours and the built-in world as a theme |
| **reader controller** (new, no framework) | `ar-bridges`, the managers, the progress record, the preloader | a progress store, a callback for "open entry" and for a jump, a hook when a target is unlocked (the title gives its reward there) |
| **reader views in Svelte** (new) | written new over the controller: scan overlay, found indicator, surface menu, entry sheet, progress (2.5 to 3k) | theme tokens, texts |
| **editor in Svelte** | the editor's generic parts | one **backend interface** in place of `lib/api.ts` (its list below), an authenticated fetch, routes or callbacks in place of `goto`, texts, a theme, and a **stage set** per experience type in place of the book on the desk |
| **stage-frame page** | the sandboxed iframe's route | the host serves it; its policy is set by a meta tag, so it moves as it is |

**The backend interface, function by function** (from `editor/src/lib/api.ts`; each is one route of the editor API today):

| Group | Functions | In SCUTOONS |
|---|---|---|
| the tree | `getBook` (spreads with their entries and targets), `listBooks` | read from the experience document |
| entities, targets | `putEntity`, `putTargetEntity`, `putTarget` | an edit of the document in memory |
| entries | `addEntry`, `getEntries`, `putEntry`, `deleteEntry` | the same |
| spreads | `addSpread`, `putSpread`, `deleteSpread` | the same; page numbering is the book's and stays behind the interface |
| book, environment, passes | `putBookTexts`, `putEnvironment`, `putBookPasses` | the same |
| draft | `getDraftManifest`, `draftBaseUrl` (the stage loads the draft as a configuration) | the configuration made from the open document |
| assets | `listAssets`, `uploadAsset`, `replaceAsset`, `assetUsages`, `deleteAsset` | SCUTOONS's assets API |
| plugins | `listPlugins`, `getPlugin`, `putPlugin`, `deletePlugin`, `uploadPlugin`, `replacePluginFile`, `pluginUsages` | later (XR.md §7) |
| publish, import | `publishBook`, `listEnvironments`, `previewImport`, `importBook` | SCUTOONS's save and publish of a document; import is OSCT's |
| not in the interface | sign-in, tokens, admin, health (17 functions) | the host's shell |

**The reader controller's surface** (from `client/src/components/ar-bridges/` and the managers): to the scene it calls `load(surface)`, `setState`, `celebrate(target)`, `refresh`, `dispose`; from the scene it hears `status`, `targetFound`, `targetLost`, `targetTapped`, `scanning`, `spreadSeen`, `ready`; it keeps the progress (unlocked targets, consulted entries, variables, the last surface) and preloads the neighbouring surfaces. In OSCT these are wired to its store and router inside `ar-bridge.ts`; the controller is that wiring with the store and the router passed in.

Each title keeps: its shell (sign-in, account, admin, AI connections), its server and storage, its book or catalog pages, its onboarding, its theme and texts, its CLI and deploy.

**Loading in SCUTOONS** (versions read from both `package.json` and the installed packages):

| | OSCT | SCUTOONS | Verdict |
|---|---|---|---|
| SvelteKit | 3.1 (editor) | 2.55 (creator, viewer) | the editor's Kit imports are `$app/paths` (11 files), `$app/state` (6), `$app/navigation` (4), `$app/types` (1) ✔; all exist in 2.55; one break: the type `AssetPath` is `Asset` in 2.55 (`editor/src/lib/stage/stage-core.svelte.ts:2` ✔) |
| Svelte | 5.57 | 5.55 | the newest feature the editor uses is `{@attach}` ✔, which 5.55 has |
| three.js | 0.174 | 0.186 (creator; the viewer has none listed) | the plugin API hands plugins `api.three`, so the three version is part of the plugin contract: both titles need one version. Not run on 0.186 (§7) |
| TypeScript | 6.0 (editor), 5.8 (reader) | 5.9 | not checked for 6-only syntax (§7) |

In the viewer, `import()` of the engine on the `/xr` route makes its own chunk, so readers who never open XR load none of it; the 8th Wall files and their licence are served by the viewer or the CDN. The viewer's service worker caches public library answers and CDN images only (`viewer/src/lib/sw/cache-rules.ts` ✔): models and video of a scene are not kept offline today.

## 3 · Where SCUTOONS's decisions do not fit OSCT's contract

| SCUTOONS's decision | Verdict | Why |
|---|---|---|
| Experience with a type (book, show, poster; record) | **contract addition**, small | an optional type on the book; the engine does not read it; the editor's stage set and pages do |
| Surfaces beyond the page (wall, screen, tabletop, poster) | **real conflict** | no pages: `firstPage`, `lastPage` and `entry.page` are required and find the surface; the guard refuses the format; OSCT's server renumbers pages. Either entries name their surface (a MAJOR change in OSCT) or a number stands in for the page. A target's printed box needs a surface of any size |
| A document versioned per save, not row tables | **adapter** | the loader takes a configuration from any source. SCUTOONS's creator edits in memory and saves on Save (`creator/src/lib/editor/doc.svelte.ts` ✔), as OSCT's editor does, so one save is one version |
| SCUTOONS's assets, accounts, roles | **adapter** | asset URLs can be absolute already; the Library needs an asset backend; sign-in, roles and the CSRF rule are the host's (`server/src/Http/CurrentSession.php` ✔) |
| An entry gives a catalog item | **adapter and a small addition** | the engine emits `targetUnlocked`; the title gives the item. Which item is stored in a title-owned field on the entry, or beside the configuration |
| Jumps between chapter and scene | **contract addition** | a behaviour action "goto" with an address the host resolves; an older app skips an action it does not know. OSCT's own plan asks for it (STRATEGY "Flows", the owner 2026-10-31) |
| Story variables | **adapter, one gap** | OSCT's `book` scope goes through the progress source, so SCUTOONS maps it to its one save game. SCUTOONS's values are text or number (`shared/content/src/save-game.ts:15` ✔), OSCT's include true and false: a conversion rule is needed |
| Texts per language | **adapter** | OSCT's content has one language (plain strings); SCUTOONS's rule is a text per language. Translations are kept beside the configuration and laid over it for the reader's language |
| A scene with an id and a name, on several targets | **fits as it is** | OSCT's shared entity: a target names it by reference (`entities`, `{ ref }`), a node has a name since schema 2.12 |

## 4 · The steps from today's OSCT to a core both use

Sizes: S days, M weeks, L months of agent work with the owner's review.

| # | Step | Size | Who | Risk for the running OSCT |
|---|---|---|---|---|
| 1 | **The editor's backend interface**: the direct imports of `lib/api.ts` replaced by one injected interface; OSCT's implementation is today's file. **Worth doing even if the core never happens**: the editor becomes testable without a server, and it is the shape SCUTOONS's copy needs | S to M | OSCT alone | low: no feature changes |
| 2 | Engine and contract as packages: their own `package.json`, three as a peer, the celebration's colours and the world as a theme, the `osct*` GLSL names kept with aliases | S | OSCT alone | low |
| 3 | three 0.174 to 0.186 in OSCT | S to M | OSCT alone | medium: the look on phones and the plugins need checking |
| 4 | The reader controller out of `client/`, OSCT's custom elements kept over it | M | OSCT alone | medium: the store is the reader's heart |
| 5 | Instance configuration and storage keys with the title in them (OSCT's phase A) | M | OSCT alone | medium: stored progress moves to new keys |
| 6 | The contract generalised: experience type, surfaces without pages, goto, a title-owned field | M | OSCT with SCUTOONS's input | a MAJOR or MINOR change by RULES #10; content is deployed apart from the app |
| 7 | The editor's stage set per type; the book pages out of the generic editor | M | partly together | low while the book stays the default |
| 8 | Svelte reader views over the controller | M | SCUTOONS (OSCT needs none) | none |
| 9 | SCUTOONS's adapters: documents, assets, save game, rewards, routes | M | SCUTOONS | none |
| 10 | Where the core lives and how it is versioned (OSCT's open decision #47; RULES #10's one version becomes one per title and one for the core) | S to M | the owner's decision | process |

Against OSCT's own plan: steps 1, 2 and 4 are refactorings with no change a reader sees, so they fit before or after the Amsterdam content phase; steps 3 and 5 change what phones show or store and should not land during it; step 6 waits for the next XR project, as the owner said. SCUTOONS's copy (XR.md §7) does steps 8 and 9 on its side first and shows what steps 1 and 6 have to carry.

## 5 · Advice for the copy SCUTOONS makes meanwhile

Read and apply before slice 2 (the engine) and slice 3 (the editor).

1. **Copy `engine/` and `shared/` unchanged into one vendored folder** with the OSCT commit recorded there. Edit nothing inside. Adapt only through the seams the engine has: `ArSceneOptions`, device storage, the graphics and progress sources, the loader's `bundled` and `fetch`, `IImageTracker`, the 8th Wall base URL.
2. **Keep every edit that cannot be avoided as a named patch** in a list beside the copy (the fixes for three 0.186, the `Asset` type, the celebration's colours), written so OSCT could take it back.
3. **Do not rename inside**: the contract's types and fields, the plugin API, the `osct*` GLSL names.
4. **SCUTOONS's words outside**: the document kind, the address book's references, the routes, the storage keys (they are passed in anyway), everything a person reads.
5. **Hand the engine exactly OSCT's configuration.** Whatever the experience document looks like when stored, one function makes a `GameConfiguration` from it, and nothing else in SCUTOONS feeds the engine. The closer the stored shape is to OSCT's, the smaller that function and the cheaper the later merge (the choice: §8).
6. **New surface formats stay SCUTOONS's** and reach the engine as a plain spread until OSCT's contract has them. A copy that adds formats to the contract forks it: the expensive case.
7. **The editor:** copy only the generic parts (stage core with the sandbox, panels, controls, code tab) and put them behind a backend interface from the first day, the same one step 1 gives OSCT. The Show's stage set is a new module beside the book's, not an edit of `mock-stage.ts`. The book pages, the shell and the admin are not copied.
8. **The reader:** the Svelte views sit over a controller without a framework, never on the engine directly. That controller is the core's later reader part.
9. **One three version** in SCUTOONS (0.186 for creator and viewer).

Expensive for a later merge: edits scattered inside engine or contract, a renamed contract, the stage set rewritten in place, panels that call SCUTOONS's API directly (OSCT's tangle repeated), a second three version.

## 6 · Svelte

The owner: "Svelte wird das primäre Frontend." What that means per part:

- **Engine: plain TypeScript, stays so.** It imports nothing of the reader ✔ and is loaded by any page.
- **Reader:** what is engine-side logic in OSCT's reader is about 1.5 to 2k lines (the glue, the managers, the progress record, the preloader); what is its own interface is everything else: 38 custom elements on OSCT's store, i18next and gold design. They cannot be mounted in Svelte without taking that store along. So SCUTOONS writes its reader interface in Svelte, 2.5 to 3k lines, over a controller.
- **Must OSCT's reader move to Svelte for one core? No.** One core in logic (engine, contract, controller) with two interfaces works. Sharing the interface too would need OSCT's reader to mount Svelte components, which its RULES forbid today ("vanilla custom web components, no framework", under "do not swap"): the owner's decision, not a technical need. Svelte 5 can mount into a custom element without SvelteKit, so a gradual move stays possible.
- **Editor: Svelte 5 already.** It carries over; the differences are SvelteKit 3 against 2.55 (one type) and its hard-coded English texts.

## 7 · Not checked

- Nothing was built or run, by instruction: how the engine behaves on three 0.186, and whether the editor compiles under SvelteKit 2.55 and TypeScript 5.9, are read from imports and versions only.
- TypeScript 6-only syntax in OSCT's editor.
- The server, the CLI and the MCP were read by a search agent of this session; the quoted lines marked ✔ were checked by hand, the rest of their tables is that agent's reading.
- The hand-over's line counts per folder and its table of the reader's parts were not recounted.

## 8 · For the owner

1. **The stored shape of an experience** (bears on slice 1): OSCT's configuration kept as it is inside the document, with SCUTOONS's own fields beside it; or SCUTOONS's own shape (drafts 2 and 3) with a mapping. Put to the owner in the chat on 2026-11-10; the answer goes into decisions/xr.md.
2. Does OSCT's reader stay on custom elements over a shared controller, or move to Svelte so the interface is shared too?
3. Surfaces without pages in OSCT's contract: entries name their surface (a MAJOR change), or a number stands in for the page?
4. Does OSCT do step 1 (the editor's backend interface) now, before SCUTOONS's copy of the editor (slice 3) settles?
5. One three version for both titles: does OSCT move to 0.186?
6. OSCT's issue #255 (opened by the research, "In progress" on OSCT's board): who posts the result there, since OSCT's orchestrator is to stay out?
