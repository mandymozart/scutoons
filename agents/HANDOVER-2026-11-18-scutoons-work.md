# Handover — SCUTOONS work on `develop` (2026-11-18)

For the next OpenCode session that continues the **work**, not the setup. Read `AGENTS.md` → `CLAUDE.md` → `agents/RULES.md` → `agents/HANDOVER.md` first. The Vertex/AI setup is settled and documented in `agents/HANDOVER-2026-11-18-vertex.md`.

## Settled — do not revisit
- **Branch:** local `develop`, nothing pushed. Owner: "i want to in branch develop for now". Local `main` is ahead of `origin/main` by 183 commits; do not push.
- **Models:** owner confirmed "models are fine". The B10 ladder (`google-vertex`, `vienna-struggle` service account) is approved — no more model-selection work. Use `/models` → `google-vertex/<model>` per kind; session-only, never rewrite the root default:
  - orchestrator/review → `gemini-4-hydrogen` medium/high · code.S → `gemini-3.5-flash-lite` medium · code.M → `gemini-4-hydrogen` high · code.L → `gemini-4-carbon` high · code.design → `gemini-4-argon` high.
- `C:\Development\agents\models.json` is still Claude-only; ask before changing that shared file. Do not revoke `…oFtw` or `…KYPIrVg`, and do not restore the hardcoded AI Studio key.

## Current checkout
- `develop` head `11097002` (work handover), before it `c55cfceb` (rule), before it `1d064d30` (local main base: #590 and #326 merged).
- **Preserve these uncommitted local files** — they are not part of this handover: `agents/costs.json`, `package.json`, `package-lock.json`, untracked `agents/gemini-usage.mjs`, `logs/`. Commit only your own paths (RULES B7).
- End every session with the usage report (RULES B9).

## Work queue (in order)
1. **Book the Vertex setup session's cost.** Ledger `agents/costs.json`; helper `agents/gemini-usage.mjs`; prices `C:\Development\agents\prices.json`; tests `python C:\Development\agents\costs\tests\run.py` → 13/13. `service: "gemini"`, tiers `std`/`batch`/`flex`/`priority`, `input = total_input_tokens - total_cached_tokens`, `cache_read = total_cached_tokens`, cache writes 0, `output = total_output_tokens`, thinking informational only; no ledger schema change. Earlier entry: `session-gemini-setup-20261118`, `est_usd: 0.0919`.
2. **Confirm `npm run deck:ci` on local `develop` after #326.** Guards also worth running: `npm run check:core-only` (baseline 3 hits), `npm run check:deck-deps`, `npm run check:generate`; headless host with no `expected_fail`.
3. **#622 — SDK sections, first of the two worktrees.** `C:\Development\scutoons\.claude\worktrees\agent-a30cef44db83deadc`, branch `feature/622-module-sections`, head `c0d399c0` with 12 modified files (catalog builtin/groups/lib, editor inspector + controls, module-macros, module SDK tests, analog params/tests, space graph/modreg/being). It has **uncommitted edits** — commit them on that branch (own paths) before rebasing onto current `develop`. Review diff, rebase, run checks, then merge into local `develop` **only on the owner's word** (RULES B3).
4. **#621 — camera move repeat, second.** `C:\Development\scutoons\.claude\worktrees\agent-a37153a65348c175e`, branch `feature/621-move-repeat`, head `be7db0d9` with 9 modified files (protocol schema + generated.rs, space relay/camera/DOP moves/track/mod, `shared/live/src/protocol.ts`, `package-lock.json`). Also uncommitted; same sequence: commit, review, rebase onto `develop`, checks, merge on the owner's word only. Watch the `package-lock.json` overlap with the main checkout — keep them separate.
5. **#623 — editor crash `0xC0000409`** with a full scene of video layers plus a character; smaller runs are fine. Still undiagnosed; needs a reproducible case before a fix.
6. **Browser deck runtime stays blocked by #290 / #306** (owner bench X1 / S22, audio sampler, hardware decoders, browser AV1). #590 is `Refs mandymozart/scutoons#590`, not `Fixes`; the ticket went back to Backlog — do not treat module slots as shippable in the browser yet.
7. **#326 remains open** for X1/S22 checks, audio sampler, hardware decoders and browser AV1, despite the first slice being merged locally.

## Guardrails
- Never push. Work and merge locally only, and merge only after checks and the owner's explicit "merge".
- One issue per worker, own worktree, own branch, `Refs #N` / `Fixes #N` in commits, comment the result on the issue (RULES B, C).
- Owner reviews/merges the parked session worktrees listed in `git worktree list`; agents do not touch them.
- Keep context small: grep and targeted reads; do not re-read whole files.

## From the Claude orchestrator (night shift 2026-11-17 → 18)
Claude's workers stopped on the weekly limit (resets 2026-11-22 04:00 Berlin). Their work is uncommitted in the worktrees above; nothing is lost.
- **#622** stopped mid-edit "wiring it into the Node branch of `draw`" (the ring's area clusters). Before that: the macro `section`/`slot`, the descriptor and `param_groups`, analog's params, the inspector and kit controls. Spec = the A4 table in #622's second comment (areas SRC/FLTR/AMP/LFO/VOICE, pages ≤ 8, twin pages share slots 0–7). Still to do: finish `graph.rs`, the library card, regenerate the catalog (`check:generate`), the tests, screenshots, deck:ci.
- **#621** stopped at "the Director: repeat rounds, timed fly-through, and a cue's start beat". Protocol schema, the relay and the track/moves edits are in progress. Still to do: the Director, the column-scoped cut (cell role/outlet) on the bar/phrase, the DOP repeat chip and per-point times, the tests.
- Both worktrees started from origin/main and already have local main merged. Merge `develop` in again before the final checks.
- **deck:ci after #326 did not finish.** The run (default tier) was green through `🦀 tests` (1225 s) and then exited with code 4 in `🧩 module tests`, while I committed the deck-deps change during it ("tree changed"). Rerun it before trusting develop. `check:deck-deps` is green at 1d064d30.

### Queue after #622 / #621 (Cyberdecks epic, owner-approved)
#608 status bar + Log panel · #607 Windows title bar (minimal buttons, no logo, mac default) · #606 camera focus outlets (no DOF) · #617 PipeWire reconnect · #618 rows port wiring on #591's type table · #591 leftovers (groups, comments, previews, latent grid, focus flash, selection) · #605 live cameras steps 1–5 (agents/research/LIVE-CAMERAS.md) · #609 hand-off (F9, lock) · #329 remainder (YUV/YCoCg in a shader, FSR 1, A/V sync, zero-copy, the editor's live video card) · #623 the editor crash.
Review list (probably done, check and close after the push): #503, #494/#501, #507/#508/#510/#511, #534, #468, #461, #445, #439.

### Open questions for the owner (don't decide them)
- The push: 183 commits ahead; about 90 issues would close.
- Close the superseded tickets #266, #299, #384, #385.
- #612 CPU bar scaling.
- #584 background as a skybox.
- The node panel summary.
- #605 details: do guests see locked cameras; the 5 s timeout.
- A hardware session for MIDI/DMX and for #326's X1/S22 checks.

### Routine and traps
- `npm run deck:ci` runs only what changed since the last green run; `--all` is the full default tier. Main builds in deck/target; worktrees build in .git/deck-target/<id>. After each merge: `npm run deck:target -- --clear`. Keep an eye on the disk (C: has about 90 GB free; the full tier takes about 45–60 min).
- Wait on a process's exit, never on a grep of its output: two workers hung that way.
- Known timing flakes that pass alone:
  - live_quic `when_udp_dies_mid_show…`;
  - deck-native `audio::watch::a_device_lost_while_playing…`.
  Rerun once; don't "fix" them blindly.
- On Windows, `git worktree remove` often leaves the folder behind ("Directory not empty"): unlock it, `remove -f -f`, then delete the folder.
- Don't commit on the branch while deck:ci runs on it: it stops stamping and can fail.
- Design: night mode is the default; panels are 50 % with a ring at night and 80 % with no outline by day; no blur.
- Third-party reference images stay local (gitignored); the ABC Areal fonts are never in the repo.
- Decisions go into `agents/decisions/deck.md` (newest first, the owner's words quoted).
- Desktop control is allowed for screenshots, but close everything opened afterwards.
- Don't open scenes against the production API with the owner's token; use `--scene-file` fixtures.
