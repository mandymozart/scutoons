Status 2026-11-10: research, nothing built, no direction chosen (#476). The map (§1–§3) is measured from the code and the deploy files; the options (§4–§5) wait for the owner's answers to §6.

# ARCHITECTURE: languages, runtimes, satellites, and how to have fewer

Owner 2026-11-10 ([architecture], decisions/deck.md): "i see in our general architecture on big gap. rust in deck, svelte for frontend other places, and php for the server. some magic container something else. … how to consolidate satelite services and reduce language and framework glutter". The trigger: "to power up the wasm only for a type head api. not smart."

Counts are lines of tracked files (`git ls-files`, without vendor, node_modules, target), 2026-11-10.

## 1 · What runs, where

| # | Runtime | Language, framework | Size | Runs on | Called by | Deployed by |
|---|---|---|---|---|---|---|
| 1 | API `api.scutoons.com` | PHP, own kernel + Composer | 39.9k lines in 276 files, 39 controllers, ≈ 310 routes; tests 17.3k; 90 SQL migrations | all-inkl shared hosting (no long-running process) | every app, the decks, the MCP, the relay (certificate) | push to main → FTP (`deploy/targets.json`) |
| 2 | QR `qr.scutoons.com` | plain PHP | 1.2k | all-inkl | printed codes | FTP |
| 3 | Viewer `scutoons.com` | SvelteKit, TS | 14.8k TS + 23.6k Svelte | all-inkl, static | readers; holds the browser deck `/live` | FTP |
| 4 | Creator | SvelteKit, TS | 17.0k TS + 38.2k Svelte | all-inkl, static | creators | FTP |
| 5 | Admin | SvelteKit | 1.3k TS + 10.8k Svelte | all-inkl, static | admins | FTP |
| 6 | Shop | SvelteKit | 0.5k | all-inkl, static | buyers | FTP |
| 7 | Embassy | SvelteKit | 2.8k | all-inkl, static | the owner (leads, mail) | FTP |
| 8 | Shared packages (17) | TS, Svelte | 39.2k TS + 19.8k Svelte (+ 10.8k tests); `shared/content` alone 19.3k | inside apps 3–7 | — | with the apps |
| 9 | The core, as wasm | Rust → wasm (`space-web` 3.9 MB, `live-core` 1.4 MB, `relay-web` 0.9 MB; 2.9 / 0.5 MB after the build) | built from row 10 | the browser: viewer `/live`, the creator's sound workspace | 7.9k lines of TS bridge (engine-host, live-core, live-deck, live, rig-designer) | copied into `viewer/static/live`, `creator/static/live` |
| 10 | Deck workspace | Rust, 44 crates | ≈ 118k lines; `space` 26.8k, `native` 14.2k, `engine` 8.6k, `relay` 7.3k, `core` 6.6k, `editor` 5.2k | — | — | — |
| 11 | Native deck + deck editor | Rust, own UI | part of row 10 | Windows, Linux, macOS; the deck OS (live ISO) | the relay, the API | update bundles (`deck/update`), `npm run deck:iso` |
| 12 | Live relay `live.scutoons.com` | Rust, static binary | 7.3k | bunny Magic Containers, app `scutoons-live`, container 1 | decks, phones, conductor | `live-image.yml` |
| 13 | Module build service | Rust + rustc, cargo, wasmtime in the image | 3.1k | the same bunny app, container 2 | the API only (`ModuleCompiler.php`, `CoreValidator.php`) | `build-image.yml` |
| 14 | MCP server | TS on Node, one bundle | 6.2k | the member's own machine | the member's Claude → the API | built with the API, `GET /api/mcp/download` |
| 15 | Scripts and checks | Node `.mjs` (24.5k), TS (0.5k), Python (1.6k), shell (0.5k) | 27k | dev PC, GitHub Actions (15 workflows) | developers, agents | — |
| 16 | Side tools | Node, Python | figma-bridge 2.1k, rooms 1.6k, vinyl3d 0.5k, clock-measure 0.3k | dev PC | the owner | — |
| 17 | Database, CDN | MariaDB 10.11; bunny.net | — | all-inkl; bunny | the API | migrations run with the deploy |
| 18 | Dev containers | MariaDB, Adminer | — | Docker on the dev PC | `npm run dev:db` | — |

In numbers: **5 languages that carry rules** (Rust 118k, PHP 41k, TS 79k, Svelte 95k, SQL) plus `.mjs` tooling; **3 hosts** (all-inkl, bunny containers, the member's or venue's machine); **2 production containers** in one app; **7 FTP targets**; **5 SvelteKit apps**; **2 UI stacks for editing rigs** (the deck editor in Rust with its own UI, `shared/rig-designer` in Svelte).

## 2 · Where data crosses a language

| Boundary | From → to | Kept in step by | State |
|---|---|---|---|
| Live protocol | `deck/protocol/schema.json` → `generated.rs` + `shared/live/src/protocol.ts` | generated, `check:generate` | good: one source |
| Wire codecs, clock state | Rust layout tables → `shared/live-core/src/{wire,signals,clock-state}.ts` | generated from Rust | good |
| Kind catalog | Rust `deck-schema` → `kinds.json` → TS (`shared/live`, MCP) and PHP (`KindCatalog.php`) | generated, `deck:ci` | good: Rust is the source for three languages |
| Core in the browser | wasm exports (15 in `space-web`) ↔ TS bridge | `check:core-only`, `bridge-files.json` | guarded; costs a 3–4 MB start |
| Core on the server | PHP → HTTP → build service `POST /validate` (rigs, presets, instruments) | the core is the only validator; 503 without it | good, and the precedent for §5 |
| Module compile | PHP → HTTP → build service | one implementation | good |
| REST API | PHP JSON ↔ `shared/api` (3.0k TS, written by hand) | by hand; PHPUnit and TS tests apart | **no shared schema** |
| Content documents | `server/schema/content/*.schema.json` (≈ 20) ↔ `shared/content` types (TS, by hand) ↔ PHP validators | by hand; some golden fixtures | **twins** |
| Content rules | PHP `Domain/*` ↔ `shared/content` (19.3k TS) | by hand | **twins, drifted** (§3) |
| MCP | TS tools → the REST API | by hand | a third reader of the API shapes |
| Address grammar | Rust `rig/address.rs` (482 lines, the one parser, A14); TS and PHP only hold address strings | A14 | one source today; the typeahead would be the first second reader |

## 3 · The same rule in two languages

From the #474 inventory (first comment there; 132 chains, ≈ 2,250 lines, 20 groups of twins):

| Pair | Groups | Examples | Drifted |
|---|---|---|---|
| PHP ↔ TS | 13 | content localisation, toon languages, quality variant, default look and body, wear slots, legacy character ids, collage style, respawn range, item max, `merged_into`, save-game merge, tile colour | yes: legacy ids (PHP drops the persona), quality on a null level (`medium` / `src`), toon languages (a step only in PHP) |
| Rust ↔ PHP | 1 | preset lineage | PHP collects only |
| Rust ↔ TS | 3 | vote tally, default music track, knob label | the file pick differs |
| same question, several rules | 3 | "which portrait when a mood is missing" (4 rules), locale order (9 orders), two `NEAR` mood tables | yes |

Reading: the deck side is nearly clean (rules A1–A7 hold, three Rust ↔ TS twins left). **The gap is between PHP and TS**: content and catalog rules live in `server/src/Domain` and again in `shared/content`, with no guard like `check:core-only`. #474 moves value resolution onto one resolver in the core; what #474 does not answer is how PHP and the plain reader pages reach that resolver.

## 4 · Options (each: what it removes, what it costs)

Costs are estimates of order, not plans. S = days, M = weeks, L = months of agent work with owner review.

| # | Option | Removes | Costs | Risk |
|---|---|---|---|---|
| A | **A Rust API replaces PHP** (the server links the core) | PHP as a language (41k + 17k tests), the HTTP hop to the build service, all Rust ↔ PHP twins; server and deck share types | L: ≈ 310 routes, auth, mail, push, migrations, storage rewritten; **leaves all-inkl** (needs a long-running process: bunny containers or a server; the DB moves or is reached remotely) | highest; touches every printed URL and the deploy; the PHP ↔ TS twins only become Rust ↔ TS twins unless C comes too |
| B | **PHP stays as shell, asks the core** (grow `/validate` into "the core, hosted": resolve, addresses, content rules) | PHP's copies of rules as they move; no rewrite of storage, auth, mail | M: per rule one endpoint + PHP forwards; the API then depends on the container for reads, not only for saves (latency, uptime, cache) | medium; shared hosting cannot run the core in-process (to check: whether all-inkl allows running a binary) |
| C | **The server is the only place for rules; clients are thin** (the API serves resolved values, `shared/content` shrinks to types and drawing) | the 13 PHP ↔ TS twin groups, a large part of `shared/content` (19k) | M–L: the creator previews drafts before they are saved and the reader works offline (PWA), both need the rule on the client today | medium; fights the creator's live preview unless the preview asks the server |
| D | **One schema, generated for every language** (as the protocol and the kind catalog already are): API shapes, content documents, and rule *tables* (locale order, legacy ids, mood tables, quality ladder) | hand-written `shared/api` types, the schema twins, the drift of table-shaped rules; the logic around the tables stays twice | S–M per area; no hosting change; can go area by area | lowest; removes drift more than code |
| E | **Fewer frontends and UI stacks** (admin + embassy + shop as one back-office app; one answer to "rig editing in Svelte or in the deck editor") | 2–3 deploy targets and workflows; up to 3k Svelte + 1.8k TS of `rig-designer` if rig editing lives only in the deck editor | S for the merge of small apps; M for the rig editing question | low; no language removed |
| F | **Fewer satellites** (the build service becomes the one core service; scripts move from `.mjs` to TS as J1 says; side tools stay tools) | one concept ("a compile container") turns into the general one; 24.5k `.mjs` is a rule debt, not a runtime | S–M | low |

They combine. Two coherent end states:
- **"Rust everywhere it decides"**: A (+ C by wasm or server). Three languages left: Rust, TS/Svelte for UI, SQL. Expensive, and a hosting move.
- **"Three languages, one source each"**: B + D (+ E, F). PHP stays for storage, auth and mail; every rule has one home (the core for the deck world, PHP for accounts and commerce) and the other languages get generated tables and types. Cheap steps, no hosting move, fits #474.

## 5 · The case that started it: one typeahead over addressable things

What a typeahead needs: (1) the grammar (the reserved words of `address.rs`), (2) the address book (what exists: characters, scenes, items, tracks, knobs), (3) a matcher, (4) the final validity check.

| Route | How | Creator starts wasm | New twin | Cost |
|---|---|---|---|---|
| T1 the core in the creator (the proposal of [typeahead]) | wasm answers each keystroke | yes, 3–4 MB for a text field | none | rejected by the owner 2026-11-10 |
| T2 **a baked address book + a plain matcher** | the core writes the book (every address with label, kind, origin, as it writes `kinds.json` today) when a catalog entry is saved; the creator filters that list by substring; the native editor and the conductor ask the core directly; validity stays the core's on save (`/validate`) | no | a ≈ 30-line string filter, no grammar in TS | S |
| T3 the server answers | `GET /api/addresses?q=` → PHP → the core service | no | none | S–M; a network round trip per keystroke, depends on the container |
| T4 a tiny wasm of the address crate only | split `address` out of `space`, ≈ 100 kB | yes, but small | none | M (crate split) |

T2 is option D in small: Rust is the source, a generated file crosses the boundary, the client holds no rule. T3 is option B in small.

## 6 · Questions that decide the direction

1. **Hosting:** does the API stay on all-inkl shared hosting for the next year, or may everything move to containers (bunny)? Without a move, option A is out and B needs the container for reads.
2. **PHP:** is PHP itself the clutter to remove (A), or is the problem that rules live twice (B + D keep PHP as a shell)?
3. **Where may a rule run for the reader and the creator's preview:** on the server only (C), or also on the client from generated tables (D), or by wasm (the owner said no for a typeahead; is that a no for the reader and the plain creator pages in general)?
4. **Rig editing:** one editor (the deck editor) or two (plus the Svelte designer in the creator)?
5. **When:** before or after 2027-02-05 (deck release) and 2027-02-28 (WEBTOON)? A large move before those dates competes with them.

## 7 · Decisions

None yet beyond [architecture] (2026-11-10, decisions/deck.md): the research itself.
