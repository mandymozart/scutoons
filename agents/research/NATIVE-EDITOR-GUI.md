Status 2026-11-08: research for #393, undecided. Recommendation: browser for content, a native Rust editor for live-sensitive creation, on our own component library over winit + wgpu; a small proof first, GPUI not as the host. The owner decides from §6 / §7.

# NATIVE-EDITOR-GUI – a native Rust GUI for the editor, and the split from content management

Owner 2026-11-08 (#393): "since we slowly reaching a breaking point of native and browser implementation … gpui, developed by the zed team … it talks about using even browser bridges once we embed wasm ... can we investigate how we would split the content management and the creation process of live sensitive data." Then: "also consider writing our own ui component library as we go, since making components has also not been the most costly aspect of the svelte project so far." · "its actually part of our design process."

All external facts were read on 2026-11-08 from the repository or crate named; each dated where the date matters. Sizes: S = one worker session, M = a few sessions, L = a week or more, several tickets.

## 0. The short answer
- **GPUI cannot host our stage on Windows today.** Upstream has no way to put a foreign GPU texture into a GPUI window there (§2). That alone rules it out as the host, on the machine the owner works on.
- **The "browser bridges" are not a bridge to our wasm** (§1). GPUI compiles to a browser canvas; that gives us nothing the browser deck lacks.
- **Our stage is already easy to host:** deck-space draws into any `wgpu::TextureView`. The toolkit that shares our wgpu device embeds it for free.
- **The owner's own-library idea is the strongest option once the substrate is borrowed** (§4): components ours, layout / text / accessibility / windows from small crates. It is the only option where the editor, the deck's own UI (#386) and the module interfaces (#419) share one set of controls.
- **The split costs little:** the creation side is under 4,000 lines today against about 55,000 in the creator (§5). Content management stays in the browser for good.
- **Speed is not the reason to go native.** #306 measured wasmtime 2.6–3× slower than Chrome's V8 per block on the PC. The reasons are the real engine in the editor, local builds in seconds, devices, and dropping browser development of modules (#290).

## 1. GPUI today
| | Finding |
|---|---|
| Licence | [`gpui`](https://github.com/zed-industries/zed/blob/main/crates/gpui/Cargo.toml), `gpui_wgpu`, `gpui_web`: Apache-2.0. Zed's own `editor` crate is GPL-3.0-or-later, so Zed's code editor is not available to us without taking the GPL. |
| Outside Zed | [crates.io `gpui`](https://crates.io/crates/gpui) is 0.2.2 of 2025-10-22, over a year behind a repository still changing weekly. The [README](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md) calls it pre-1.0 with frequent breaking changes and wants the latest stable Rust (Zed pins 1.98.1; our workspace declares 1.80). [gpui.rs](https://www.gpui.rs/) says it stays tied to Zed for the near future. In practice people use a snapshot: `gpui-pre` 0.3.11 (2026-11-02, snapshot of a Zed commit, published by one account) or forks. |
| Platforms, renderer | Own window layer per OS, no winit. macOS: Metal. **Windows: its own DirectX renderer, no wgpu** (`gpui_windows` has no wgpu dependency). Linux and web: `gpui_wgpu`, on a wgpu git revision (30.0.0 development, 2026-09-15). Ours is 25. Recipe: one shader per primitive, glyphs in an atlas ([Zed, 2023-03-07](https://zed.dev/blog/videogame)). |
| Docs | A thin site, examples in the repository. The real manual for building apps is Longbridge's [GPUI Kit](https://github.com/longbridge/gpui-kit) (0.9.0, 2026-11-02; Apache-2.0): 75+ components, docking, a code editor with Tree-sitter + LSP, used in their commercial app. It pins `gpui-pre`. |
| Accessibility | AccessKit is in GPUI (workspace dependency, `a11y` example). |

**Web, and the "browser bridges".** The web backend exists: [`gpui_web`](https://github.com/zed-industries/zed/tree/main/crates/gpui_web), merged 2026-02-26 ([#50228](https://github.com/zed-industries/zed/pull/50228)), still receiving commits on 2026-10-29. Its source says: "GPUI's browser platform uses one document-owned canvas and supports one top-level window." WebGPU first, WebGL2 as the fallback; threads need nightly Rust and a cross-origin-isolated page; the gallery builds on nightly. I found nothing called a browser bridge in Zed's repository or docs. What the owner read is most likely one of these:
1. `gpui_web`: a whole GPUI app compiled for a browser canvas. Exists. Inside it are small adapters to the browser (hidden input for IME, Fetch). It does not connect GPUI to other wasm.
2. [`gpui-webview`](https://github.com/longbridge/gpui-kit/tree/main/crates/webview) (Longbridge, on Wry): a real browser view inside a GPUI window, by its own README experimental; on Windows it is WebView2 and covers GPUI's popovers. It has nothing to do with wasm in GPUI, and it brings the browser back into the native app, the same objection as any webview shell under `[rust-first]`.
3. `gpui-shell` (Longbridge): an optional JavaScript extension runtime. Not relevant.

**What it means for us:** nothing. The browser deck already runs our core and stage as wasm and has no UI layer left to replace (#362, #385). Our wasm is hosted by our own code in either world.

## 2. The decisive question: can a GPUI window host our wgpu stage?
**What we have** (`deck/Cargo.lock`): wgpu 25.0.2, naga 25.0.1, winit 0.30.13, raw-window-handle 0.6.2, cpal 0.15. **What the stage needs from a host:** deck-space is already host-independent: `SpaceGpu::new(device, queue, space, format, size)` (`deck/native/src/render.rs:761`), `resize(device, size)` and `draw(queue, encoder, target: &TextureView, size, space, time)` (`deck/space/src/space/gpu.rs:523, 556`). `deck/space-web` drawing into a canvas is the proof. Device needs are modest: no special features, `downlevel_webgl2_defaults` limits (`render.rs:215`). A host provides a wgpu 25 device + queue, a colour target at panel size, one call per frame with the time, and pointer and keys translated into the panel. Audio runs on its own cpal / PipeWire thread beside any UI. **What must change for every host (S–M):** `Renderer::new` takes the winit window and creates the surface itself (`render.rs:184`), and the deck's loop lives in `deck/native/src/main.rs` (1,799 lines). Split "bootstrap" from "draw one frame into a target" and lift the loop into a library. That is also the direction of `check:deck-deps` ("main.rs only shrinks").

**GPUI's answer, upstream, on Windows: no.**
- [`surface()`](https://github.com/zed-industries/zed/blob/main/crates/gpui/src/elements/surface.rs) and `PaintSurface` exist only for macOS / iOS and take a CoreVideo buffer.
- On Windows [`draw_surfaces`](https://github.com/zed-industries/zed/blob/main/crates/gpui_windows/src/directx_renderer.rs) (line 816) returns without drawing anything.
- `canvas()` only reaches GPUI's 2D paint calls; the accepted answer in [GPUI Kit #2594](https://github.com/longbridge/gpui-kit/discussions/2594) (2026-07-25) reaches the same conclusion for 3D viewports.
- [Zed discussion #64849](https://github.com/zed-industries/zed/discussions/64849) (opened 2026-09-28) proposes externally owned surfaces; no maintainer reply and no PR by its last comment, 2026-10-28; it explicitly keeps wgpu types out of GPUI.

**What remains, none good:** (a) a child OS window with our wgpu over the panel: popovers and menus go underneath, no clipping (the webview's limits). (b) Render offscreen and copy to the CPU each frame: 8.3 MB per 1080p frame, about 500 MB/s at 60 fps, fine for a prototype only. (c) Patch GPUI: a shared D3D11 texture from our Vulkan or DX12 device, sync per OS, and we carry a fork of fast-moving code. Forks exist that go this way (WGPUI, gpui-ce, bite-gpui); I did not evaluate them; the page of `gpui_interop` timed out (504).

## 3. Alternatives on the same criteria
| | wgpu stage in a panel | Text, code editor | Docking | Accessibility | Wasm | Licence, maturity | As a base for our own components |
|---|---|---|---|---|---|---|---|
| **GPUI + Kit** | Not on Windows (§2) | Best: Kit's editor, LSP | Kit, serialisable | AccessKit | Canvas, one window | Apache-2.0; pre-1.0 | Good (`gpui-base`, flex styling), but stays outside the deck's pass |
| **egui** 0.37 (2026-10-15) | Excellent: `CallbackTrait` paints with the same device. Same winit. **0.32 is the last on wgpu 25** | `TextEdit` + layouter; no real Rust editor | `egui_dock`, `egui_tiles` | AccessKit | eframe, canvas | MIT / Apache; widely used | Weak: immediate mode, its look and layout model show through |
| **Iced** 0.14 (2025-12-07) | Good: `shader` widget. wgpu 30, a winit fork | `text_editor` widget, no LSP | `pane_grid` (tiles, no tabs) | No AccessKit in the workspace | Yes | MIT; README says experimental | Fair: a real custom-widget trait |
| **Slint** 1.18.1 (2026-09-21) | Through `unstable-wgpu-29/30` features | `TextInput`; none for code | None found | AccessKit (winit backend) | Yes | GPL-3 / royalty-free / commercial | A second language (`.slint`) against Rust-first |
| **Dioxus native / Blitz** 0.2.1 (2026-10-16) | `wgpu_texture` example exists. Needs winit 0.31 beta, renderer from a git branch, wgpu 30 | Basic inputs | None found | AccessKit | Dioxus web = DOM, a different renderer | MIT / Apache; README says beta | High in idea: HTML + CSS, so our CSS could travel. Too early |
| **Xilem / Masonry** 0.4.0 (2025-10-29) | Masonry runs in any wgpu window; main is on wgpu 28 | Basic | None found | AccessKit | xilem_web = DOM | Apache-2.0; README says experimental | Fair; a moving target |
| **Own, on `deck-ui`** | Trivial: same device, same pass, no wgpu upgrade | Ours on [parley](https://github.com/linebender/parley) 0.12 (2026-10-09: an editing module, an `editor` example); code editor L | Ours: DOCKING.md is already the design | `accesskit_winit` 0.34, node by node | The space-web way: same draw code on a canvas | All MIT / Apache; we own the risk | It is the option |

Every option that shares our device on a current release also forces a wgpu upgrade first (25 → 28–30; M, touches the deck and space-web). Only the own library and egui 0.32 avoid that for the proof.

## 4. Our own component library (the owner's point)
It changes the weighing. GPUI Kit's largest gift is 75 ready components; if making them is part of the design process, that gift is not the point. What remains worth buying is the substrate. So:
- **Yours by design:** every control, the docks, the inspector, the node view, the meters. `shared/ui` shows the scale: 56 Svelte components, about 12,500 lines with tokens, and DESIGN.md + DOCKING.md already specify them (rem sizes, one control height, round vs square, tiling regions, Rail, Divider).
- **Not components, so borrowed:** the browser gave us these free, and writing buttons does not replace them. Windows and input: winit. GPU: wgpu. Layout: [taffy](https://github.com/DioxusLabs/taffy) 0.15 (flex and grid). Text shaping and editing: parley. Accessibility: AccessKit. Clipboard and file dialogs: small crates.
- **Layers, within today's rules:** `deck-ui` stays the platform-free model (no wgpu, `DECK-UI-MODEL.md`). A new kit crate holds the tree, layout, focus, input and tokens and outputs a draw list; no wgpu, builds for wasm, guarded by `check:deck-deps`. A thin adapter turns the draw list into wgpu draws (rounded rects by distance function, a glyph atlas: Zed's published recipe; `gpui_wgpu`'s shaders are Apache-2.0 and readable).
- **What only this option gives:** one set of controls for the editor, the deck's own UI (#386) and the browser deck's canvas (RULES A1 / A6, "the deck's own UI is the only UI"). 2D controls and a module's 3D interface (#419) in one pass. No dependence on Zed's roadmap.
- **Where it can go wrong, measured by the proof (§6):**
  1. **Text.** Umlauts, dead keys, selection, IME: this is what the browser hid.
  2. **The design loop.** Svelte shows an edit at once; Rust rebuilds. The answer is Unreal's split (owner 2026-11-08): parameters change live in the running editor, code compiles. The compiled code declares which parameters exist; changing a value is live, changing the declaration is code. Live: tokens, a control's position, size and layout (kept as data, not Rust), parameters, 3D placement and outlet mapping. Compiled without leaving the app: module code, the instance swapped while the editor keeps running. Wirings: open (§8). Rebuild and restart: only a new kind of control or new behaviour, in a gallery binary that is also the design tool. Final build: publish through the compile service.
  3. **Size in the browser deck.** Parley with Unicode data costs wasm (space-web is about 1 MB gzip today); the deck may need a smaller text path with our own fonts only.
  4. **The code editor** is the one truly large component (§7, question 2).

## 5. The split
| Piece | Today | Side | Where it lives |
|---|---|---|---|
| Toons, seasons, chapters, slice editor, storyboard, script, collage, cover, credits, Kanon, map, flow | creator, about 45,000 lines | Content | Browser, unchanged |
| Catalog lists, items, recipes, products, assets, usages, review, publish, users, roles | creator, admin, API | Content | Browser, unchanged |
| Instruments, presets, rigs **as catalog entries** (list, used by, rig coverage, review, publish) | creator, admin | Content | Browser; "Open in editor" where editing starts |
| Row view / stage builder (arranges a show over the story; not built) | planned, a creator lens | Content | Browser; it picks scenes, it does not make them |
| Module authoring: Rust, Parameters, compile, problems | `shared/rig-designer` | Live-sensitive | Native editor |
| Wiring, hand rewiring, addressing (#203, #425) | Nodes tab read-only; concept | Live-sensitive | Native editor (external only, as decided) |
| Rigs: Assignment, moods, code rigs (#283, #412, #425 slices 2–3) | Assignment tab, in progress | Live-sensitive | Native editor |
| 3D objects, outlet mapping, Blender / Tripo (#419) | concept | Live-sensitive | Native editor; files on disk make the round trip easy |
| Sound design workspace, mood audition, signal strip | creator `/edit/[key]` | Live-sensitive | Native editor |
| Scene editor (#244, #245; not thought through yet) | not built | Live-sensitive | Native editor; #245's "where it lives" is answered by this |
| Conductor view; deck pages | viewer | Neither | Unchanged; live/deck plays final builds only (2026-11-04) |

**How the two sides share data.**
- **The API and the catalog stay the single source (RULES E3, E4).** The editor reads and writes the same keys (`module:<id>`, `rig:<subject>`, `scene:<id>`, `preset:<id>`) through `PUT /api/entities/{key}`. There is no native file format as truth, and the canonical `/edit/<kind>:<id>?w=&sel=` address stays, opened as a deep link.
- **Drafts:** every save is a new version. `PUT` already takes `base` and answers 409 when it is not the newest (`EntityStore`), so a browser and an editor editing at once are already handled.
- **Local working copy:** one folder per open key holding the sources and a media cache; rebuilt from the API at any time. Works offline for editing and audition; saving waits for the network.
- **Compile:** local builds reuse `deck/build` as a library with the same rules (pinned RUSTFLAGS, the determinism checks), in seconds, for audition in wasmtime. **The compile service stays the only producer of a published module** (`modules/<hash>.wasm` from the sources): a locally built wasm is never uploaded. The hashes can be compared to catch a drift.
- **Publish stays in the browser**, like the AI connections' draft → link (RULES G2; `ApiTokenScope`). The editor gives the link.

**Sign-in (flow only).** The deck's handshake already fits (`Domain/Decks/Decks.php`, #180): the app makes a key pair, asks with its public key, shows a short code; the member approves it on the account page; the app signs a challenge and gets a short-lived token, renewed by itself. Nobody copies a cookie or pastes a token. New: deck tokens are read-only (`DeckTokenScope`), so the editor needs its own kind with a scope that allows drafts, compile and asset upload and refuses what `ApiTokenScope` refuses; creators and admins only. S–M.

**What happens to the code already written.**
| Piece | Lines | Fate | Size |
|---|---|---|---|
| Core: engine, module-host, deck-space, rigs, `deck/build`, schemas | all | **Kept, the point of the core** | – |
| Server: entities, modules, assets, versions | all | Kept; one new token kind | S–M |
| Rust tab, `code-editor`, `editor-cm`, CodeMirror | ~500 | Replaced. External editor first (S), own editor later (L) | S, later L |
| Parameters tab | ~90 + descriptor | Rebuilt from the kit; the descriptor is Rust already | S |
| Problems console, diagnostics, messages | ~720 | Rewritten in Rust; the rules of what shows are small | S–M |
| Nodes tab, `nodes.ts` | ~300 | Layout idea kept, rewritten; hand rewiring (#203) built natively only | M–L |
| Assignment tab, `assignment.ts` | ~310 | Moved last: it is a form and works today in the browser | M |
| `rig-coverage-list` | ~65 | **Stays** (content side) | – |
| Sound workspace (`creator/.../sound`), `/edit/[key]` | ~820 | Kept until the native workspace has parity, then removed; the route becomes the deep link | S |
| `shared/engine-host` | ~900 | Stays: `viewer/live/deck` uses it; the creator stops | – |
| #290 browser backend | open | Re-scoped: final builds only, no development audition (2026-11-04) | smaller |
| Rule A12, [designer-in-creator], EDITOR.md "creator app" | docs | Superseded if the owner decides so | S |

## 6. Recommendation
**Split as in §5. Content management stays in the browser for good. For creation, stay in the browser until a small proof has passed, then build a native editor on our own library. Do not adopt GPUI.** Reasons: the stage embeds with no compromise; the same controls serve the editor and the deck; it fits the core and the owner's way of designing; every crate is permissively licensed and replaceable. Its true cost is the substrate, and the proof tests that before anything is committed.

**Is "stay in the browser for now" honest?** For the next weeks, yes. The browser workspace works today, and Live restarts 2026-11-16. What should stop now is only new Svelte investment that a native editor would throw away, above all the canvas for hand rewiring.

**Order.**
1. Now: keep the wiring model in the Rust core (#203 concept, addressing, descriptor groups). That survives either way. #425 slice 1 (silence) is core work.
2. Split the deck-native renderer and loop into a library (S–M). Useful regardless.
3. **The proof** (S–M, a branch that may be thrown away).
4. Owner decides. If yes: the kit (M), docks + basics as the first workspace needs them (M), as a design process with a gallery.
5. The editor token + reading and saving drafts (S–M).
6. Rust + Parameters + local build + audition, with an external code editor (M). Now module work is native-first.
7. Nodes and hand rewiring #203 (M–L), rigs #412, #425 (M), then Assignment (M); the browser workspace is removed.
8. 3D outlet mapping #419 (M), the scene editor #244 / #245 (L), the deck's own UI #386 on the same kit, after the engine work as decided.

**The proof: one window, Windows dev PC, wgpu 25.** Left panel: the deck-space stage with the being, drawn on the shared device. Right panel: a Divider, tabs, a `scrub-number` bound to a rig knob, one text field. The engine plays a rig through cpal. Passes when:
- UI pass ≤ 2 ms per frame at 1080p with 200 controls, and the stage's frame time is unchanged;
- no audio glitch in 10 minutes of dragging and resizing (deck-core's monitor);
- the text field handles umlauts, dead keys, selection, copy and paste, and an IME;
- a changed token or a moved / resized control shows in under 1 s with no restart, new widget code in under 15 s in the gallery;
- the code stays under about 2,500 lines.

**What would change this.**
- Zed lands Windows external surfaces (#64849) **and** the owner wants an in-app code editor early → GPUI + our own components becomes the better deal.
- The text or design-loop gate fails → stay in Svelte for creation and take the browser route (MODULE-BENCH option A with the loop counter, which passed on the PC in #306).
- Blitz leaves beta and its texture embedding holds up → look again, because our CSS could carry over.
- Creators who must work on machines without an install → the browser stays.
- The #306 X1 / S22 numbers decide how modules run, not which GUI.

## 7. Questions for the owner
1. **Foundation: our own library on winit + wgpu + taffy + parley, or GPUI with our components?** Recommended: our own, and the proof first.
2. **The Rust tab: an external editor (VS Code or Zed on the files, watched, rust-analyzer for free) to start, or in-app from day one?** Earlier you asked for "a rust code editor, which holds files … make it proper". Recommended: external first, because it is proper immediately; the in-app editor (L) later, as its own design piece.
3. **May new UI for hand rewiring wait for the proof, and be built natively only?** Recommended: yes; model and addressing work continues in the core.
4. **Does the deck's own UI (#386) use the same library?** Recommended: yes; that is what makes owning it pay off.
5. **Publish stays in the browser, the editor only saves drafts and gives the link?** Recommended: yes.
6. **Which systems must the editor run on first?** Recommended: Windows and Linux; macOS when a creator needs it (wgpu covers it, signing is the work).

## 8. Proposed tickets (not filed)
- Split deck-native's renderer and loop into a library (S–M).
- Proof: one window, stage + panel, the gates of §6 (S–M; `decide` after).
- After the owner's yes: the kit crate + wgpu draw adapter + gallery (M) · tokens from `shared/tokens` to Rust (S) · `deck/editor` with docks (M) · an editor token kind + approval on the account page (S–M) · local build through `deck/build` as a library (S) · Rust + Parameters workspace (M) · Nodes and hand rewiring, moved from #203 (M–L) · rig workspace #412, #425 (M) · Assignment (M) · retire `/edit/[key]`, update A12 and EDITOR.md (S).
- **Open, owner 2026-11-08: does hard wiring make it faster or slower? Bench it (S, in `deck/module-bench`, PC + X1): N modules wired by the engine, the same merged in one module, the same native.** What the code and #306 / #396 already say: the engine fixes the graph when it is built (levels, a buffer pool; a lone unscaled link costs no step), so a wiring written in code and one drawn by hand run the same. Only merging modules could gain: entering a module costs at most about 5–11 µs per block (`walk`, the smallest, tuned vs deck config; budget 2667 µs), one fault then takes all of them, and rebuilds grow. The larger levers are elsewhere: wasmtime settings (fm16 96 → 36 µs, native Rust 36) and audio links that split FM / sine voices over levels, which breaks their 8-lane batches. Across modules control is one block late (2.7 ms) and feedback at least one block: a timing limit, not a speed one. Unreal 5.8 ([MetaSounds](https://dev.epicgames.com/documentation/en-us/unreal-engine/metasounds-the-next-generation-sound-sources-in-unreal-engine), [Presets](https://dev.epicgames.com/documentation/en-us/unreal-engine/metasounds-reference-guide-in-unreal-engine), [Live Updates](https://dev.epicgames.com/documentation/en-us/unreal-engine/metasound-builder-api-in-unreal-engine), beta since 5.5): the author wires a graph as data, the engine compiles it to a fixed object with data by reference; presets lock the graph and expose inputs; topology edits are heard live only while auditioning. Recommended, for the owner: the same three classes here: parameters live, wiring as a graph document fixed at build (no cargo, crossfaded in audition, editor only), code compiled.
