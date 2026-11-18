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
