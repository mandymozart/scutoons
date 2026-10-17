# GOTCHAS — tool traps on this machine (limit 80 lines; one line each, newest knowledge replaces the old)

From the old MEMORY.md (archive/MEMORY.md, 2026-09-28 … 10-04) and later sessions. Add a trap when it cost you time.

## Windows, shells, scripts
- The Bash tool's heredocs mangle backslashes (a PHP `'\\'` arrives as `'\'`, even with quoted `'EOF'`) and break on
  some quote mixes – write scripts / patches with the Write tool, not a heredoc.
- Python on Windows writes CRLF in text mode – open files with `newline='\n'` (or `newline=''` when editing); a heredoc
  piped into `python -` breaks on non-ASCII unless `PYTHONUTF8=1`. CRLF makes `schema.js --check` report schemas as stale.
- PowerShell: `git commit -m @'…'@` with quotes breaks native argument passing → `git commit -F <file>` (UTF-8 without
  BOM; `Set-Content -Encoding utf8` writes a BOM). `[IO.File]` paths resolve against the process cwd – use absolute paths.
- symfony/console prints CRLF on Windows – diff with `--strip-trailing-cr`.
- PHP curl on Windows has no CA bundle → `CurlHttpClient` uses `CURLSSLOPT_NATIVE_CA`.

## Svelte, Vite, paraglide
- A new paraglide key must exist in de + en before a component uses it (else `m.x is not a function`, the page hangs on
  "Loading …"); removing keys while a dev server runs breaks its module graph – keep them until a restart.
- A `vite build` of the viewer rewrites `src/lib/paraglide` as per-message modules → the running dev server 404s
  ("Failed to fetch dynamically imported module") until `npm run check` regenerated them; no restart needed.
- A plain `vite build` bakes the production URLs – for local measurements set `VITE_API_URL` / `VITE_CDN_URL`.
- `vite preview` (sirv) caches the file list at start – restart it after every build; a build while it serves kills it.
- Svelte 5 `<select value={x}>` with `{#each}` options can render the last option first → `selected={…}` on the options.
- A Svelte 5 action's `update` can run before the same flush's text node changes → measure in a microtask.
- A snippet variable named like a `{#snippet}` breaks svelte-check with odd "Snippet" type errors.
- On $state data `const l = (o.list ??= [])` is the RAW array – pushes into it change the data unseen (no redraw); assign,
  then read back through `o` (creator clip.ts `listAt`; 2026-10-17 pasted boxes / items missing on the canvas).

## CSS
- base.css `h1–h6 { font-weight: 400 }` beats `:where()` rules → tools.css headings use `.sc-tool :where(h…)`.
- Creator-wide button rules are zero-specificity and `inline-flex`: row buttons need `justify-content`, not `text-align`.
- `container-type` on `.slice` makes it a stacking context + containing block for `position: fixed`; chapter-1 split
  text boxes rely on the later slice painting over the earlier one – keep it.

## Browser checks
- The Browser pane's key name `Return` does not fire `Enter` – use `Enter`.
- The pane is shared with the owner and other sessions – work in your own tab, check state before asserting.
- Another session's dev-server restart re-mounts via HMR and drops unsaved creator edits – check field values before saving.
- The library answers with `max-age=60`: after a publish the old bundle stays in the browser for a minute.
- A viewer URL without a locale prefix follows the browser language (headless = en) – checks use `/de/<toon>`.
- Guests can't read draft chapters locally without a code – run such checks as the test user.
- Several `intersect` slice events with the same trigger fire in no fixed order.
- The Browser pane can't register service workers – test the PWA in headless Chrome over CDP (research/PWA.md §6).
- A viewer build can crash natively (exit 0xC0000409) at random – run it again.

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
