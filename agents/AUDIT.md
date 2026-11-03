# AUDIT — the protocol (limit 80 lines; read when merging deck / shared / viewer work or running an audit)

Model: **research/BRIDGE-AUDIT.md** (#357, first pass 2026-11-03). Why it exists: the one-core rule (RULES A1–A7) was written down and
agents still worked around it — a TS copy of the cathedral scene drifted from the X1's `cathedral.toml`, P6 was merged on its report
without a look at the diff. Reading rules does not hold; looking at code does. Owner 2026-11-03 [audit]: "we will rerun the audit once we
are done with rig editor, etc. and often, since i see agents working around this." · [no-duplicate]: "no duplicate code base in the end.
but this is part of the parents job to check this."

## 1 · What an audit is
A read-only pass over the browser side and every platform's glue that answers one question per feature: **is this logic, and is it in the
core?** It never changes code. Result: a table + tickets, nothing else. Not an audit: a code review of one branch (that is the merge check, §3).

## 2 · When it runs
| Trigger | Kind | Who |
|---|---|---|
| every merge of deck / shared / viewer work | **merge check** (§3 steps 1–4, minutes) | the orchestrator |
| after every ~5 merged deck / shared / viewer tickets | **quick pass** (§3 all automatic steps + §4 report grep) | the orchestrator, or a Sonnet worker |
| end of every night shift | **quick pass** + the morning summary names the deltas | the night orchestrator |
| after every leg (rig editor #282 #283 #314, ABI v2 #339–#346, the media leg, the live epic's UI) | **full hand pass** (§3 all + §5 table) | a read-only **Opus** session, fresh context |
| the owner asks, or a report says "kept in TS" | any | whoever notices |
Counting is the orchestrator's duty: it writes "merged since last audit: N" in its HANDOFF.

## 3 · The checklist (in this order; stop and ticket at the first failure)
1. `npm run check:core-only` — green, and its baseline did not grow (`scripts/core-only-baseline.json` vs `main`'s; growth needs
   `Owner-approved-baseline: …` in a commit message; `// core-only-ok: #N` lines are listed by `--report` — read them, each needs a live ticket).
2. `npm run check:deck-deps` (#363) — no platform library in a core crate (scripts/deck-deps.json), main.rs and the native logic files did not grow
   (scripts/native-logic.json); a new bridge file has its `adapter:` / `ui:` line in scripts/bridge-files.json. `npm run audit:bridge` gives the counts and deltas.
3. Headless deck host (#363, `cargo run -p deck-headless --bin deck-scenarios`): pass / xfail / FAIL per scenario; a move ticket turned its `expected_fail`
   green and removed the line; what the browser shows that no scenario can reach through the core is a missing capability – a ticket and an x-scenario.
4. **The diff check** at merge: `git diff main...<branch> -- shared/ viewer/ creator/` for new scene / patch / rule / default / range / text
   literals, tempo or beat math, MIDI status bytes, branching on catalog kinds. Say in the merge note that it was checked.
5. Twin hunt: same constants in TS and Rust (`[1, 2, 4, 8, 10]`, `0.88`, fps tables), data files under a bridge root that are not generated
   from `deck/` (hash), a second parser / validator / state machine for something the core has.
6. UI authority (#362): every control and readout on `/live/deck` and the creator's deck half — who owns the state, who computes the number
   or text. Exactly one owner, and it is the core; the Svelte side formats nothing it could receive.
7. Monitoring parity: FPS, frame time, CPU, audio load, glitches, lead, limiter — one definition and one threshold set in the core, shown
   alike in browser and native (BRIDGE-AUDIT §2 is the table to fill).
8. Native side too: logic in a *platform* crate (main.rs, hud.rs …) that the wasm build cannot reach is the same defect mirrored.

## 4 · Read what agents wrote (grep, every pass)
`git log --since=<last audit> --format=%B` and the workers' issue comments / reports for: `kept in TS because` · `for now in the page` ·
`trimmed copy` · `twin of` · `same rule as` · `as the native` · `stays in the page` · `until the core has` · `workaround` · `fallback` ·
`mirror`. Every hit is a ticket or a fix — an agent that writes the phrase has told you where the twin is.

## 5 · Output (research/BRIDGE-AUDIT-<date>.md, line 1 = status)
The table of BRIDGE-AUDIT §1: **feature · where it lives (file, lines) · verdict · native today · cost S/M/L · ticket**.
Verdicts: **P** periphery (only this platform can do it — keep; say why) · **W** logic in the wrong place · **D** duplicated (a twin exists in Rust)
· **U** UI authority (a second copy of state or a number the core has). Plus: the baseline entry by entry, monitoring parity, the ranked moves
(rank = drift already seen × what a native or third device would redo ÷ cost), and "not judged here". Every claim checked in the code at a named commit.

## 6 · From findings to tickets
One ticket per move: labels `chore` (or `bug` if behaviour is wrong) + `area:platform` (or the area) + `complexity:S/M/L`; body = the table row,
the files, the native twin, done-when "no logic/scene/rule/default/text in TS/Svelte that the core has or should have"; **Part of #311**
(the engine epic) and linked to the audit issue. Board: Backlog is the owner's to set (RULES C6) — the audit lists the new numbers in its
comment and says "owner: sort into Backlog". Existing baseline tickets are re-pointed to the move tickets. The next engine phase waits for the
fix tickets only if the owner says so (the owner did for P9, 2026-11-03 [audit]: "if that research finds something, create issues to fix it, and only
afterwards continue. with our plan").

## 7 · What counts as a workaround (a finding even if it works)
Any decision, number, text, scene, default, validation or state machine in TS / Svelte / a platform crate that the core has, or should have so a
native or third device gets it for free — however small, trimmed, temporary or "just for the page". Includes: a TS constant replacing a retired
patch; a status text built in Svelte; a clamp or range typed twice; a relay client written again; a feature flag that keeps both copies alive; a
`core-only-ok` without a ticket; baseline growth by renaming. NOT a workaround: the stopwatch, permissions, Web MIDI / Gamepad / WebSocket / fetch /
AudioWorklet / canvas access, storage, DOM — periphery (P), listed as such with the reason.

## 8 · Who runs it
A **read-only Opus session** (RULES B5), fresh context, brief by the template (ORCHESTRATION) with the previous audit report and the list of
merged tickets since. It does not fix, and the session that wrote the code never audits it. The orchestrator runs the merge check itself.
Usage report at the end (RULES B9).

## 9 · Escalation to the owner
Stop and tell the owner (a `decide` issue + the morning summary) when: the baseline would have to grow; a finding needs a behaviour or threshold
choice (which HUD thresholds are right); a periphery verdict is arguable; the same twin comes back after it was ticketed; an agent
removed or loosened the guard to get through; two audits in a row find new W/D in the same area (the brief or the rule is the problem, not
the agent). Record the answer as a DECISIONS line the same day.
