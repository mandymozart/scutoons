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

## Heavy work that needs a slot (workspace §3)
- The deck ISO: `npm run deck:iso` (archiso in Docker, ~1.8 GB; `DECK_OUT` picks the folder under `deck/os/out/`).
- The local MariaDB (`npm run dev:db`), the dev servers (vite starts at 8000) and any headless browser.

## Hardware – only the owner
- Writing a stick needs an elevated PowerShell on the owner's desk (UAC cannot be clicked over Remote Control); never
  the LaCie (owner data); back a stick up before overwriting it.
- Tests on the X1 / any deck are morning work for the owner: the worker lists exactly what to look at.

## Gallery and morning
- Deck work is mostly code-only: the gallery entry says so and names what the owner checks on the X1. Web UI changes
  get browser screenshots as usual.
- The morning summary replaces HANDOVER "Last run"; every issue gets its own comment (RULES "Backlog").
