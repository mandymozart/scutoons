# SCUTOONS – orchestration specifics (limit 100 lines)

The workspace rules are `C:\Development\agents\ORCHESTRATION.md` (roles, resources, slots, brief, merging, usage §10–§11; the old path
`C:\Development\ORCHESTRATION.md` only redirects). This file holds only where SCUTOONS differs or adds; where the two disagree this file wins
for SCUTOONS, the owner's words win over both. The law is RULES.md (B = sessions, A7 = the core guard); this file is the how.

## Where SCUTOONS differs
- **Workers run in their own worktree on `<kind>/<issue>-<title>`** and never merge or push (RULES B2); the shared checkout
  `C:\Development\scutoons` is the orchestrator's. Merge into local `main` after the checks and the diff check, on the owner's word (RULES B3). (2026-11-02 [no-cloud])
- **Never push at night.** A push to `main` deploys; the owner pushes or says "push" (RULES B4). Actions minutes no longer matter (2026-11-01 [public-repo]),
  so one push per finished step is fine on the owner's word. The deck CI runs locally (`npm run deck:ci`), not on GitHub (RULES A13).
- **One git operator** per checkout at a time: while a worker commits in the shared checkout, the orchestrator commits nothing there.
- **Over Remote Control** sessions may not be startable – the worker is then a background agent (`Agent`, one at a time, same brief).
- **Two local sessions at most** across all projects (RULES B1); an audit or review counts as one.

## Starting a worker (model and effort)
`start_session` cannot pick a model above the orchestrator's own. To run a worker on a stronger model (Opus 5.5 for engine fundamentals,
design, audits – RULES B5): 1. `start_session` with the brief's title and a one-line "wait for the full brief"; 2. at once
`set_session_model` (+ `set_session_effort`) on the new session id; 3. `stop_session` its first turn (it only works while the session is
running, so do 2 and 3 right after 1, before the filler turn ends); 4. `SendMessage` the full brief (worked for the P8 / audit workers, 2026-11-03). Why a filler first: if the stop comes late, the first turn runs the
whole brief on the old model (#380 ran completely on Sonnet that way, 2026-11-03, and had to be reviewed again on Opus). Check with `get_session` that the model is set. Large
findings and hand-over notes go to the shared notes folder named in the brief, not into messages.

## Brief template (copy, fill, send – ≤ 25 lines; rules go BY POINTER, never pasted)
```
#N <what> — session title starts with #N
Issue: #N (`gh issue view N --comments`; `npm run backlog -- set N "In progress"`, at the end "Review on staging").
Background: 3–5 lines. Owner decisions that bind this: keys [a] [b] in agents/decisions/<topic>.md (the worker greps them), not pasted text.
Read, nothing more: CLAUDE.md → agents/RULES.md (whole) → agents/HANDOVER.md → <topic doc §x>.
Branch: <kind>/N-<title> from main <sha>, own worktree, own `npm install`. No merge, no push, no attribution line (RULES B2, B4).
Scope: <checklist>. Other workers touch: <files> — stay out.
Acceptance: <the issue's done-when>. ALWAYS, for deck / shared / viewer / creator-deck work: "no logic/scene/rule/default/text in
  TS/Svelte that the core has or should have" — `npm run check:core-only` green, baseline not grown, `npm run deck:ci`; say "twins
  introduced: none". Anything you keep outside the core is a ticket first (AUDIT.md §7).
Checks (named, one at a time, each once): <list>.
Model · effort · caps: <e.g. Sonnet 5.5 medium · 100 turns / $15 | Opus 5.5 high> (agents/models.json; complexity:S/M/L on the issue).
Report ≤ 15 lines: branch + tip SHA · done per checklist item · checks and results · left out and why · twins/"for now": none or listed ·
  servers stopped · turns / cost. Comment it on the issue. Last: `python C:\Development\agents\costs\report_usage.py --issues N`.
```
An orchestrator that pastes rule text into a brief is doing it wrong: if a rule is missing from RULES, fix RULES; if the worker did not
follow a pointer, that is a finding, not a reason to paste. Grep the area's decisions file before filing an issue or writing a brief (RULES B6).

## Checks (one at a time)
- `deck/` (Rust workspace): `cargo test` and `cargo clippy` in the crate touched; the `pipewire` feature builds only on Linux. This machine's clippy
  may be newer than a worker's: `npm run deck:ci` here before every merge.
- Web apps: `npm run check` and the area's `check:*` script; no dev server unless the task needs a look.
- Server: `npm run dev:db test` (MariaDB, ~15 min, 487 tests on 2026-10-29) – it uses Docker, so it needs a **slot**.
- Never chain the merge commit behind a piped check (`npm run deck:ci | tail && git commit`): the pipe's exit code is `tail`'s, so a failed CI
  still commits (2026-11-01, PR #227). Run the check, read its own exit code, then commit.
- **Before every merge of deck work: the diff check** (AUDIT.md §3.4): `git diff main...<branch> -- shared/ viewer/ creator/` for new scene /
  patch / rule / default / range / text literals or branching on catalog kinds in TS / Svelte; a twin of Rust logic is a defect (RULES A6) –
  fix or ticket it before merging; say in the merge note that it was checked. Do not merge on the worker's report alone (P6, 2026-11-03).
- **Rerun the audit often** (AUDIT.md §2; owner 2026-11-03 [audit]): after every leg, at the end of every night shift, after every ~5 merged
  deck / shared / viewer tickets; write "merged since last audit: N" in HANDOFF. `npm run audit:bridge` is its counter (deltas since the last
  `bridge-audit-N` tag – tag the commit of a finished pass). A move ticket (#359 #360 #364–#376) merges only with its headless `expected_fail` scenario green.

## Heavy work that needs a slot (workspace §3)
- The deck ISO: `npm run deck:iso` (archiso in Docker, ~1.8 GB; `DECK_OUT` picks the folder under `deck/os/out/`).
- The local MariaDB (`npm run dev:db`), the dev servers (vite starts at 8000) and any headless browser.

## Hardware – only the owner
- Writing a stick needs an elevated PowerShell on the owner's desk (UAC cannot be clicked over Remote Control); never the LaCie (owner data);
  back a stick up before overwriting it.
- Tests on the X1 / any deck are morning work for the owner: the worker lists exactly what to look at.

## Gallery and morning
- No gallery (workspace "Publishing"): screenshots go into the chat with the summary. Deck work is mostly code-only: the summary says so and
  names what the owner checks on the X1. Web UI changes get browser screenshots as usual.
- The morning summary replaces HANDOVER "Last run"; every issue gets its own comment (RULES C5).
