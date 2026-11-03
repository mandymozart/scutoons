# SCUTOONS – orchestration specifics

The workspace rules are `C:\Development\ORCHESTRATION.md` (roles, resources, slots, brief, merging, gallery). This file
holds only where SCUTOONS differs or adds; where the two disagree, this file wins for SCUTOONS, the owner's words win
over both. The git and backlog rules themselves stay in RULES.md ("Backlog", "Git").

## Where SCUTOONS differs
- **One checkout, `main`.** Workers commit on `main` in small steps – no worktrees, no branches – unless the owner asks
  for a worktree by name (RULES "Git"); a worktree branch is merged only when the owner says so.
- **Never push at night.** A push to `main` deploys. The owner pushes, or asks for it; then bundled (issue 73), never
  per commit. "CI green before merge" (workspace §6.3) therefore happens after the owner's push – check the deploy
  and deck CI then.
- **One git operator** in the checkout at a time: while a worker commits on `main`, the orchestrator commits nothing.
- **Over Remote Control** sessions cannot be started – the worker is then a background agent of the orchestrator
  session (one at a time, same brief).

## Checks (one at a time)
- `deck/` (Rust workspace): `cargo test` and `cargo clippy` in the crate touched; the `pipewire` feature builds only on
  Linux.
- Web apps: `npm run check` and the area's `check:*` script; no dev server unless the task needs a look.
- Server: `npm run dev:db test` (MariaDB, ~15 min, 487 tests on 2026-10-29) – it uses Docker, so it needs a **slot**.

- Never chain the merge commit behind a piped check (`npm run deck:ci | tail && git commit`): the pipe's exit code is
  `tail`'s, so a failed CI still commits (2026-11-01, PR #227 – fixed in a follow-up commit). Run the check, read its
  own exit code, then commit.
- A cloud worker's clippy is its container's toolchain; this machine's may be newer (1.99 `manual_slice_fill`) –
  `npm run deck:ci` here before every merge, as the rules say.

- **Before every merge of deck work: the duplicate check.** `git diff main...<branch> -- shared/ viewer/ creator/` for new scene / patch / rule / default / range / text literals or branching on catalog kinds in TS / Svelte; a twin of Rust logic is a defect (RULES "No second copy of core logic"), fix or ticket it before merging; say in the merge note that it was checked.

- **Rerun the bridge audit (#357) often** (owner 2026-11-03: "we will rerun the audit once we are done with rig editor, etc. and often, since i see agents working around this"): after every leg (rig editor, ABI v2 build, media leg, live UI), at the end of every night shift and after every 5 merged deck / shared / viewer tickets; read agents' reports for "kept in TS because", "for now in the page", "trimmed copy"; the guards catch the rest (#363): `npm run audit:bridge` (check:core-only v2 + its report and the deltas since the last `bridge-audit-N` tag – tag the commit of a finished hand pass), `npm run check:deck-deps` (no platform library in a core crate, main.rs only shrinks) and the headless scenarios in deck:ci. A move ticket (#359 #360 #364–#376) is accepted when its `expected_fail` headless scenario turns green and the line is removed.

## Heavy work that needs a slot (workspace §3)
- The deck ISO: `npm run deck:iso` (archiso in Docker, ~1.8 GB; `DECK_OUT` picks the folder under `deck/os/out/`).
- The local MariaDB (`npm run dev:db`), the dev servers (vite starts at 8000) and any headless browser.

## Hardware – only the owner
- Writing a stick needs an elevated PowerShell on the owner's desk (UAC cannot be clicked over Remote Control); never
  the LaCie (owner data); back a stick up before overwriting it.
- Tests on the X1 / any deck are morning work for the owner: the worker lists exactly what to look at.

## Gallery and morning
- No gallery (workspace "Publishing"): screenshots go into the chat with the summary. Deck work is mostly code-only:
  the summary says so and names what the owner checks on the X1. Web UI changes get browser screenshots as usual.
- The morning summary replaces HANDOVER "Last run"; every issue gets its own comment (RULES "Backlog").
