# GOTCHAS — tool traps on this machine (limit 80 lines; one line each, newest knowledge replaces the old)

From the old MEMORY.md (archive/MEMORY.md, 2026-09-28 … 10-04) and later sessions. Add a trap when it cost you time.

## Windows, shells, scripts
- **Parallel `deck:ci` runs clash** (several worker sessions at once): `shared/live-deck`'s relay test binds fixed port 7072 (set `LAB_RELAY_PORT` to a free port per run), and `deck-outputs`' `midi_bytes_through_a_mock_port_and_the_conductor` is timing-sensitive under load (passes alone). Re-run, do not chase a merge fault.
- The Bash tool's heredocs mangle backslashes (a PHP `'\\'` arrives as `'\'`, even with quoted `'EOF'`) and break on
  some quote mixes – write scripts / patches with the Write tool, not a heredoc.
- Python on Windows writes CRLF in text mode – open files with `newline='\n'` (or `newline=''` when editing); a heredoc
  piped into `python -` breaks on non-ASCII unless `PYTHONUTF8=1`. CRLF makes `schema.js --check` report schemas as stale.
- PowerShell: `git commit -m @'…'@` with quotes breaks native argument passing → `git commit -F <file>` (UTF-8 without
  BOM; `Set-Content -Encoding utf8` writes a BOM). `[IO.File]` paths resolve against the process cwd – use absolute paths.
- symfony/console prints CRLF on Windows – diff with `--strip-trailing-cr`.
- Figma bridge: never run the relay as a Claude background task (it dies with the session) – `supervise.mjs --detach`;
  after a relay.mjs change `supervise.mjs --restart`; saving plugin/code.js or ui.html restarts the running plugin (Figma's
  hot reload brings it back, the call in flight is lost) – not during an export run (tools/figma-bridge/README.md).
- PHP curl on Windows has no CA bundle → `CurlHttpClient` uses `CURLSSLOPT_NATIVE_CA`.

## Svelte, Vite, paraglide
- A new paraglide key must exist in de + en before a component uses it (else `m.x is not a function`, the page hangs on
  "Loading …"); removing keys while a dev server runs breaks its module graph – keep them until a restart.
- A `vite build` of the viewer rewrites `src/lib/paraglide` as per-message modules → the running dev server 404s
  ("Failed to fetch dynamically imported module") until `npm run check` regenerated them; no restart needed.
- A plain `vite build` bakes the production URLs – for local measurements set `VITE_API_URL` / `VITE_CDN_URL`.
- `vite preview` (sirv) caches the file list at start – restart it after every build; a build while it serves kills it.
- A throwaway route written again at the path of one deleted earlier can be served as the OLD module by the running
  creator dev server – edit the new file once (any change) and it serves the new one.
- Svelte 5 `<select value={x}>` with `{#each}` options can render the last option first → `selected={…}` on the options.
- A Svelte 5 action's `update` can run before the same flush's text node changes → measure in a microtask.
- A snippet variable named like a `{#snippet}` breaks svelte-check with odd "Snippet" type errors.
- An `$effect` that reads a `$state` it also writes (even inside a helper it calls, or its cleanup) loops →
  `effect_update_depth_exceeded`, and that error stops EVERY later update of the page (all buttons look dead) – write
  such flags, never read them in the effect (2026-10-17 marquee-text.svelte: the mini player froze the viewer).
- On $state data `const l = (o.list ??= [])` is the RAW array – pushes into it change the data unseen (no redraw); assign,
  then read back through `o` (creator clip.ts `listAt`; 2026-10-17 pasted boxes / items missing on the canvas).

## CSS
- base.css `h1–h6 { font-weight: 400 }` beats `:where()` rules → tools.css headings use `.sc-tool :where(h…)`.
- Creator-wide button rules are zero-specificity and `inline-flex`: row buttons need `justify-content`, not `text-align`.
- `container-type` (on `.slice`, a collage `.piece` …) is a stacking context + containing block for `position: fixed`
  only in older engines – Chrome < 129, Firefox < 131, Safari / every iPhone browser up to iOS 18 (layout containment,
  dropped 2024): code must work both ways – a blend group gets its own `isolation: isolate`, fixed layers live outside
  `.slice` (fx `fxLayer`), a text box past its slice is cut on an old iPhone (research/TEXT-ACROSS-SLICES.md §1).
- A modal `<dialog>` (`showModal`) inside a `display: none` ancestor opens as an invisible 0 × 0 modal and makes the
  page inert (the phone's closed inspector sheet hides its content) – pickers live outside panels that can hide.

## Browser checks
- The Browser pane's key name `Return` does not fire `Enter` – use `Enter`.
- The pane's clicks are mouse clicks, also in its mobile preset. A finger's tap ends in a click AFTER pointerup, on what
  opened there (a picker's card) – `swallowTapClick` (creator lib/editor/tap-click.ts); test touch over CDP
  (`Emulation.setTouchEmulationEnabled` + `Input.dispatchTouchEvent`, check:creator-chapter's phone case).
- The pane is shared with the owner and other sessions – work in your own tab, check state before asserting.
- Another session's dev-server restart re-mounts via HMR and drops unsaved creator edits – check field values before saving.
- The library answers with `max-age=60`: after a publish the old bundle stays in the browser for a minute.
- A viewer URL without a locale prefix follows the browser language (headless = en) – checks use `/de/<toon>`.
- Guests can't read draft chapters locally without a code – run such checks as the test user.
- Several `intersect` slice events with the same trigger fire in no fixed order.
- The Browser pane can't register service workers – test the PWA in headless Chrome over CDP (research/PWA.md §6).
- A viewer build can crash natively (exit 0xC0000409) at random – run it again.
- Playwright (check:endless-engines) needs the builds of the pinned playwright-core: `npx playwright-core install chromium
  firefox webkit` – `npx playwright install` fetches the newest playwright and other builds.
- Playwright scales wheel deltas by the DPR in Chromium (compare with the deltaX the page got); its Firefox sends no
  pointerup for a release outside the viewport and, under load, returns from `wheel` before the page has the event (wait
  for it); other sessions' load or dev-server restarts flake the endless checks' timing steps – rerun.

## Server, DB
- The dev API runs the working tree: a new table without its migration breaks reads for everyone – migrate right away.
- Two test runs at the same time share ports and the mail log → spurious failures; rerun alone.
- phpMyAdmin's connection is utf8mb4_unicode_ci, the tables utf8mb4_general_ci – migrations compare columns with
  literals, never user variables (#1267).
- A pasted URL can't go in a path segment (`%2F` → 404) – use `?code=`. PHP hex literals ≥ 2^63 are floats.
- Slim can't bind static closures as route handlers – use invokable classes.
- **SCUTOONS MCP restart (Claude Code desktop has no Reconnect for user servers):** find your session –
  PowerShell: walk `Get-CimInstance Win32_Process -Filter "ProcessId = $PID"` up to the `claude.exe` that is the parent
  of a `node … tools/mcp/bin/scutoons-mcp.mjs`; `Stop-Process` that launcher + its `main.ts` child; the next MCP call
  starts it again with the new code. The session's tool LIST may stay the old one until a new session – a new tool can
  need a new session (2026-10-14: upload_score_file).
- **Sound in a browser check:** Playwright's Chromium cannot load an AudioWorklet module on this PC (even a blob processor never resolves), so live-core's Start stalls in "starting" there – check the sound in the in-app browser; its pane gets no animation frames while off screen (fps / levels stay 0: replace `requestAnimationFrame` with a 16 ms timer in the page first). Git Bash rewrites `VITE_API_URL=/api` into a Windows path – prefix `MSYS_NO_PATHCONV=1`.
